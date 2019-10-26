/*
===============================================================================

  PROGRAMMERS:

    martin.isenburg@rapidlasso.com  -  http://rapidlasso.com
    uday.karan@gmail.com - Hobu, Inc.

  COPYRIGHT:

    (c) 2007-2014, martin isenburg, rapidlasso - tools to catch reality
    (c) 2014, Uday Verma, Hobu, Inc.
    (c) 2019, Thomas Montaigu

    This is free software; you can redistribute and/or modify it under the
    terms of the GNU Lesser General Licence as published by the Free Software
    Foundation. See the COPYING file for more information.

    This software is distributed WITHOUT ANY WARRANTY and without even the
    implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.

  CHANGE HISTORY:
    6 June 2019: Translated to Rust
===============================================================================
*/

//! Module with the definition of a RGB struct and implementations of
//! Compressors and Decompressors

use num_traits::clamp;

use crate::las::utils::flag_diff;
use crate::packers::Packable;

fn u8_clamp(n: i32) -> u8 {
    clamp(n, i32::from(std::u8::MIN), i32::from(std::u8::MAX)) as u8
}

pub trait LasRGB {
    fn red(&self) -> u16;
    fn green(&self) -> u16;
    fn blue(&self) -> u16;

    fn set_red(&mut self, new_val: u16);
    fn set_green(&mut self, new_val: u16);
    fn set_blue(&mut self, new_val: u16);
}

/// Struct representing a RGB component of a point, in compliance with
/// the LAS spec
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct RGB {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
}

impl RGB {
    pub const SIZE: usize = 6;
}

impl LasRGB for RGB {
    fn red(&self) -> u16 {
        self.red
    }

    fn green(&self) -> u16 {
        self.green
    }

    fn blue(&self) -> u16 {
        self.blue
    }

    fn set_red(&mut self, new_val: u16) {
        self.red = new_val;
    }

    fn set_green(&mut self, new_val: u16) {
        self.green = new_val
    }

    fn set_blue(&mut self, new_val: u16) {
        self.blue = new_val;
    }
}

struct ColorDiff(u8);

impl ColorDiff {
    fn from_points<P: LasRGB, OP: LasRGB>(current: &P, last: &OP) -> Self {
        let v = (flag_diff(last.red(), current.red(), 0x00FF) as u8) << 0
            | (flag_diff(last.red(), current.red(), 0xFF00) as u8) << 1
            | (flag_diff(last.green(), current.green(), 0x00FF) as u8) << 2
            | (flag_diff(last.green(), current.green(), 0xFF00) as u8) << 3
            | (flag_diff(last.blue(), current.blue(), 0x00FF) as u8) << 4
            | (flag_diff(last.blue(), current.blue(), 0xFF00) as u8) << 5
            | ((flag_diff(current.red(), current.green(), 0x00FF)
            || flag_diff(current.red(), current.blue(), 0x00FF)
            || flag_diff(current.red(), current.green(), 0xFF00)
            || flag_diff(current.red(), current.blue(), 0xFF00)) as u8)
            << 6;

        Self { 0: v }
    }

    fn new(v: u8) -> Self {
        Self { 0: v }
    }

    fn lower_red_byte_changed(&self) -> bool {
        self.0 & (1 << 0) != 0
    }

    fn upper_red_byte_changed(&self) -> bool {
        self.0 & (1 << 1) != 0
    }

    fn lower_green_byte_changed(&self) -> bool {
        self.0 & (1 << 2) != 0
    }

    fn upper_green_byte_changed(&self) -> bool {
        self.0 & (1 << 3) != 0
    }

    fn lower_blue_byte_changed(&self) -> bool {
        self.0 & (1 << 4) != 0
    }

    fn upper_blue_byte_changed(&self) -> bool {
        self.0 & (1 << 5) != 0
    }
}

impl Packable for RGB {
    fn unpack_from(input: &[u8]) -> Self {
        Self {
            red: u16::unpack_from(&input[0..2]),
            green: u16::unpack_from(&input[2..4]),
            blue: u16::unpack_from(&input[4..6]),
        }
    }

    fn pack_into(&self, output: &mut [u8]) {
        u16::pack_into(&self.red, &mut output[0..2]);
        u16::pack_into(&self.green, &mut output[2..4]);
        u16::pack_into(&self.blue, &mut output[4..6]);
    }
}

pub mod v1 {
    //! Contains the implementation for the Version 1 of the RGB Compression / Decompression
    //!
    //! The algorithm is pretty simple:
    //!
    //! - Each bytes of each color components are encoded separately with their own context.
    //! - A byte is compressed only if it has changed
    //! - A u8 symbol is first encoded with the information on which byte changed or not
    use std::io::{Read, Write};
    use std::mem::size_of;

    use crate::compressors::{IntegerCompressor, IntegerCompressorBuilder};
    use crate::decoders::ArithmeticDecoder;
    use crate::decompressors::{IntegerDecompressor, IntegerDecompressorBuilder};
    use crate::encoders::ArithmeticEncoder;
    use crate::las::rgb::LasRGB;
    use crate::las::utils::{lower_byte, read_and_unpack, upper_byte};
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;
    use crate::record::{FieldCompressor, FieldDecompressor};

    use super::{ColorDiff, RGB};

    const LOWER_RED_BYTE_CONTEXT: u32 = 0;
    const UPPER_RED_BYTE_CONTEXT: u32 = 1;
    const LOWER_GREEN_BYTE_CONTEXT: u32 = 2;
    const UPPER_GREEN_BYTE_CONTEXT: u32 = 3;
    const LOWER_BLUE_BYTE_CONTEXT: u32 = 4;
    const UPPER_BLUE_BYTE_CONTEXT: u32 = 5;

    pub struct LasRGBDecompressor {
        last: RGB,
        byte_used_model: ArithmeticModel,
        decompressor: IntegerDecompressor,
    }

    impl Default for LasRGBDecompressor {
        fn default() -> Self {
            Self {
                last: Default::default(),
                byte_used_model: ArithmeticModelBuilder::new(64).build(),
                decompressor: IntegerDecompressorBuilder::new()
                    .bits(8) // 8 bits, because we encode byte by byte
                    .contexts(6) // there are 6 bytes in a RGB component
                    .build_initialized(),
            }
        }
    }

    impl LasRGBDecompressor {
        pub fn decompress_byte<R: Read>(
            &mut self,
            decoder: &mut ArithmeticDecoder<R>,
            context: u32,
            last_byte_value: u8,
        ) -> std::io::Result<i32> {
            self.decompressor
                .decompress(decoder, i32::from(last_byte_value), context)
        }
    }

    pub struct LasRGBCompressor {
        last: RGB,
        byte_used_model: ArithmeticModel,
        compressor: IntegerCompressor,
    }

    impl Default for LasRGBCompressor {
        fn default() -> Self {
            Self {
                last: Default::default(),
                byte_used_model: ArithmeticModelBuilder::new(64).build(),
                compressor: IntegerCompressorBuilder::new()
                    .bits(8)
                    .contexts(6)
                    .build_initialized(),
            }
        }
    }

    impl<R: Read> FieldDecompressor<R> for LasRGBDecompressor {
        fn size_of_field(&self) -> usize {
            3 * size_of::<u16>()
        }

        fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()> {
            self.last = read_and_unpack::<_, RGB>(src, first_point)?;
            Ok(())
        }

        fn decompress_with(
            &mut self,
            decoder: &mut ArithmeticDecoder<R>,
            buf: &mut [u8],
        ) -> std::io::Result<()> {
            let color_diff =
                ColorDiff::new(decoder.decode_symbol(&mut self.byte_used_model)? as u8);

            if color_diff.lower_red_byte_changed() {
                let new_lower_red = self.decompress_byte(
                    decoder,
                    LOWER_RED_BYTE_CONTEXT,
                    lower_byte(self.last.red),
                )?;
                self.last.red = new_lower_red as u16 | self.last.red & 0xFF00
            }

            if color_diff.upper_red_byte_changed() {
                self.last.red |= (self.decompress_byte(
                    decoder,
                    UPPER_RED_BYTE_CONTEXT,
                    upper_byte(self.last.red),
                )? as u16)
                    << 8;
            }

            if color_diff.lower_green_byte_changed() {
                let new_lower_green = self.decompress_byte(
                    decoder,
                    LOWER_GREEN_BYTE_CONTEXT,
                    lower_byte(self.last.green),
                )?;
                self.last.green = new_lower_green as u16 | self.last.green & 0xFF00;
            }

            if color_diff.upper_green_byte_changed() {
                self.last.green |= (self.decompress_byte(
                    decoder,
                    UPPER_GREEN_BYTE_CONTEXT,
                    upper_byte(self.last.green),
                )? as u16)
                    << 8;
            }

            if color_diff.lower_blue_byte_changed() {
                let new_lower_blue = self.decompress_byte(
                    decoder,
                    LOWER_BLUE_BYTE_CONTEXT,
                    lower_byte(self.last.blue),
                )?;
                self.last.blue = new_lower_blue as u16 | self.last.blue & 0xFF00;
            }

            if color_diff.upper_blue_byte_changed() {
                self.last.blue |= (self.decompress_byte(
                    decoder,
                    UPPER_BLUE_BYTE_CONTEXT,
                    upper_byte(self.last.blue),
                )? as u16)
                    << 8;
            }
            self.last.pack_into(buf);
            Ok(())
        }
    }

    impl<W: Write> FieldCompressor<W> for LasRGBCompressor {
        fn size_of_field(&self) -> usize {
            6
        }

        fn compress_first(&mut self, dst: &mut W, buf: &[u8]) -> std::io::Result<()> {
            self.last = RGB::unpack_from(buf);
            dst.write_all(buf)
        }

        fn compress_with(
            &mut self,
            mut encoder: &mut ArithmeticEncoder<W>,
            buf: &[u8],
        ) -> std::io::Result<()> {
            let current_point = RGB::unpack_from(buf);
            let sym = ((lower_byte(self.last.red()) != lower_byte(current_point.red())) as u8) << 0
                | ((upper_byte(self.last.red()) != upper_byte(current_point.red())) as u8) << 1
                | ((lower_byte(self.last.green()) != lower_byte(current_point.green())) as u8) << 2
                | ((upper_byte(self.last.green()) != upper_byte(current_point.green())) as u8) << 3
                | ((lower_byte(self.last.blue()) != lower_byte(current_point.blue())) as u8) << 4
                | ((upper_byte(self.last.blue()) != upper_byte(current_point.blue())) as u8) << 5;

            encoder.encode_symbol(&mut self.byte_used_model, sym as u32)?;
            let color_diff = ColorDiff::new(sym);

            if color_diff.lower_red_byte_changed() {
                self.compressor.compress(
                    &mut encoder,
                    lower_byte(self.last.red) as i32,
                    lower_byte(current_point.red()) as i32,
                    LOWER_RED_BYTE_CONTEXT,
                )?;
            }

            if color_diff.upper_red_byte_changed() {
                self.compressor.compress(
                    &mut encoder,
                    upper_byte(self.last.red) as i32,
                    upper_byte(current_point.red()) as i32,
                    UPPER_RED_BYTE_CONTEXT,
                )?;
            }

            if color_diff.lower_green_byte_changed() {
                self.compressor.compress(
                    &mut encoder,
                    lower_byte(self.last.green) as i32,
                    lower_byte(current_point.green()) as i32,
                    LOWER_GREEN_BYTE_CONTEXT,
                )?;
            }

            if color_diff.upper_green_byte_changed() {
                self.compressor.compress(
                    &mut encoder,
                    upper_byte(self.last.green) as i32,
                    upper_byte(current_point.green()) as i32,
                    UPPER_GREEN_BYTE_CONTEXT,
                )?;
            }

            if color_diff.lower_blue_byte_changed() {
                self.compressor.compress(
                    &mut encoder,
                    lower_byte(self.last.blue) as i32,
                    lower_byte(current_point.blue()) as i32,
                    LOWER_BLUE_BYTE_CONTEXT,
                )?;
            }

            if color_diff.upper_blue_byte_changed() {
                self.compressor.compress(
                    &mut encoder,
                    upper_byte(self.last.blue) as i32,
                    upper_byte(current_point.blue()) as i32,
                    UPPER_GREEN_BYTE_CONTEXT,
                )?;
            }
            self.last = current_point;
            Ok(())
        }
    }
}

pub mod v2 {
    //! Contains the implementation for the Version 2 of the RGB Compression / Decompression
    use std::io::{Read, Write};

    use crate::decoders::ArithmeticDecoder;
    use crate::encoders::ArithmeticEncoder;
    use crate::las::rgb::LasRGB;
    use crate::las::utils::{lower_byte, read_and_unpack, upper_byte};
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;
    use crate::record::{FieldCompressor, FieldDecompressor};

    use super::{ColorDiff, RGB, u8_clamp};

    pub(crate) struct RGBModels {
        byte_used: ArithmeticModel,
        lower_red_byte: ArithmeticModel,
        upper_red_byte: ArithmeticModel,
        lower_green_byte: ArithmeticModel,
        upper_green_byte: ArithmeticModel,
        lower_blue_byte: ArithmeticModel,
        upper_blue_byte: ArithmeticModel,
    }

    impl Default for RGBModels {
        fn default() -> Self {
            Self {
                byte_used: ArithmeticModelBuilder::new(128).build(),
                lower_red_byte: ArithmeticModelBuilder::new(256).build(),
                upper_red_byte: ArithmeticModelBuilder::new(256).build(),
                lower_green_byte: ArithmeticModelBuilder::new(256).build(),
                upper_green_byte: ArithmeticModelBuilder::new(256).build(),
                lower_blue_byte: ArithmeticModelBuilder::new(256).build(),
                upper_blue_byte: ArithmeticModelBuilder::new(256).build(),
            }
        }
    }


    pub(crate) fn compress_rgb_using<W: Write>(
        encoder: &mut ArithmeticEncoder<W>,
        models: &mut RGBModels,
        current_rgb: &RGB,
        last_rgb: &RGB) -> std::io::Result<()> {
        let mut diff_l = 0i32;
        let mut diff_h = 0i32;
        let mut corr;

        let color_diff = ColorDiff::from_points(current_rgb, last_rgb);
        encoder.encode_symbol(&mut models.byte_used, color_diff.0 as u32)?;

        //TODO replace these as u8 as u32
        if color_diff.lower_red_byte_changed() {
            diff_l = lower_byte(current_rgb.red) as i32 - lower_byte(last_rgb.red) as i32;
            encoder.encode_symbol(&mut models.lower_red_byte, diff_l as u8 as u32)?;
        }

        if color_diff.upper_red_byte_changed() {
            diff_h = upper_byte(current_rgb.red()) as i32 - upper_byte(last_rgb.red) as i32;
            encoder.encode_symbol(&mut models.upper_red_byte, diff_h as u8 as u32)?;
        }
        if (color_diff.0 & (1 << 6)) != 0 {
            if color_diff.lower_green_byte_changed() {
                corr = lower_byte(current_rgb.green) as i32
                    - u8_clamp(diff_l + lower_byte(last_rgb.green) as i32) as i32;
                encoder.encode_symbol(&mut models.lower_green_byte, corr as u8 as u32)?;
            }

            if color_diff.lower_blue_byte_changed() {
                diff_l = (diff_l + lower_byte(current_rgb.green()) as i32
                    - lower_byte(last_rgb.green) as i32)
                    / 2;
                corr = lower_byte(current_rgb.blue()) as i32
                    - u8_clamp(diff_l + lower_byte(last_rgb.blue) as i32) as i32;
                encoder.encode_symbol(&mut models.lower_blue_byte, corr as u8 as u32)?;
            }

            if color_diff.upper_green_byte_changed() {
                corr = upper_byte(current_rgb.green) as i32
                    - u8_clamp(diff_h + upper_byte(last_rgb.green) as i32) as i32;
                encoder.encode_symbol(&mut models.upper_green_byte, corr as u8 as u32)?;
            }

            if color_diff.upper_blue_byte_changed() {
                diff_h = (diff_h + upper_byte(current_rgb.green) as i32
                    - upper_byte(last_rgb.green) as i32)
                    / 2;
                corr = upper_byte(current_rgb.blue) as i32
                    - u8_clamp(diff_h + upper_byte(last_rgb.blue) as i32) as i32;
                encoder.encode_symbol(&mut models.upper_blue_byte, corr as u8 as u32)?;
            }
        }
        Ok(())
    }

    pub struct LasRGBCompressor {
        last: RGB,
        models: RGBModels,
    }

    impl Default for LasRGBCompressor {
        fn default() -> Self {
            Self {
                last: RGB::default(),
                models: RGBModels::default(),
            }
        }
    }


    impl<W: Write> FieldCompressor<W> for LasRGBCompressor {
        fn size_of_field(&self) -> usize {
            3 * std::mem::size_of::<u16>()
        }

        fn compress_first(&mut self, dst: &mut W, buf: &[u8]) -> std::io::Result<()> {
            self.last = super::RGB::unpack_from(&buf);
            dst.write_all(buf)
        }

        fn compress_with(
            &mut self,
            encoder: &mut ArithmeticEncoder<W>,
            buf: &[u8],
        ) -> std::io::Result<()> {
            let current_point = super::RGB::unpack_from(&buf);
            compress_rgb_using(encoder, &mut self.models, &current_point, &self.last)?;
            self.last = current_point;
            Ok(())
        }
    }

    pub struct LasRGBDecompressor {
        last: RGB,
        models: RGBModels,
    }

    impl Default for LasRGBDecompressor {
        fn default() -> Self {
            Self {
                last: RGB::default(),
                models: RGBModels::default(),
            }
        }
    }

    pub(crate) fn decompress_rgb_using<R: Read>(decoder: &mut ArithmeticDecoder<R>,
                                                models: &mut RGBModels,
                                                last: &RGB) -> std::io::Result<RGB> {
        let sym = decoder.decode_symbol(&mut models.byte_used)?;
        let color_diff = ColorDiff { 0: sym as u8 };

        let mut this_val = RGB::default();
        let mut corr;
        let mut diff;

        if color_diff.lower_red_byte_changed() {
            corr = decoder.decode_symbol(&mut models.lower_red_byte)? as u8;
            this_val.red = corr.wrapping_add(lower_byte(last.red)) as u16;
        } else {
            this_val.red = last.red() & 0xFF;
        }

        if color_diff.upper_red_byte_changed() {
            corr = decoder.decode_symbol(&mut models.upper_red_byte)? as u8;
            this_val.red |= (corr.wrapping_add(upper_byte(last.red)) as u16) << 8;
        } else {
            this_val.red |= last.red() & 0xFF00;
        }

        if (sym & (1 << 6)) != 0 {
            diff = lower_byte(this_val.red) as i32 - lower_byte(last.red) as i32;

            if color_diff.lower_green_byte_changed() {
                corr = decoder.decode_symbol(&mut models.lower_green_byte)? as u8;
                this_val.green = corr
                    .wrapping_add(u8_clamp(diff + lower_byte(last.green) as i32) as u8)
                    as u16;
            } else {
                this_val.green = last.green() & 0x00FF;
            }

            if color_diff.lower_blue_byte_changed() {
                corr = decoder.decode_symbol(&mut models.lower_blue_byte)? as u8;
                diff = (diff + lower_byte(this_val.green) as i32
                    - lower_byte(last.green()) as i32)
                    / 2;
                this_val.blue = (corr
                    .wrapping_add(u8_clamp(diff + lower_byte(last.blue) as i32) as u8))
                    as u16;
            } else {
                this_val.blue = last.blue() & 0x00FF;
            }

            diff = upper_byte(this_val.red) as i32 - upper_byte(last.red) as i32;
            if color_diff.upper_green_byte_changed() {
                corr = decoder.decode_symbol(&mut models.upper_green_byte)? as u8;
                this_val.green |= (corr
                    .wrapping_add(u8_clamp(diff + upper_byte(last.green) as i32))
                    as u16)
                    << 8;
            } else {
                this_val.green |= last.green() & 0xFF00;
            }

            if color_diff.upper_blue_byte_changed() {
                corr = decoder.decode_symbol(&mut models.upper_blue_byte)? as u8;
                diff = (diff + upper_byte(this_val.green) as i32
                    - upper_byte(last.green) as i32)
                    / 2;

                this_val.blue |= ((corr
                    .wrapping_add(u8_clamp(diff + upper_byte(last.blue) as i32)))
                    as u16)
                    << 8;
            } else {
                this_val.blue |= last.blue & 0xFF00;
            }
        } else {
            this_val.green = this_val.red;
            this_val.blue = this_val.red;
        }
        Ok(this_val)
    }

    impl<R: Read> FieldDecompressor<R> for LasRGBDecompressor {
        fn size_of_field(&self) -> usize {
            6
        }

        fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()> {
            self.last = read_and_unpack::<_, RGB>(src, first_point)?;
            Ok(())
        }

        fn decompress_with(
            &mut self,
            decoder: &mut ArithmeticDecoder<R>,
            buf: &mut [u8],
        ) -> std::io::Result<()> {
            let this_val = decompress_rgb_using(decoder, &mut self.models, &self.last)?;
            self.last = this_val;
            this_val.pack_into(buf);
            Ok(())
        }
    }
}

pub mod v3 {
    //! Contains the implementation for the Version 3 of the RGB Compression / Decompression
    //!
    //! The version 3 of the compression / decompression algorithm
    //! is the same as the version 2, but with the support for the contexts system
    //!
    //! A V3 decompressor / compressor owns 4 contexts which are just rgb::v2 compressor or decompressors
    //! and it forwards the compression / decompression to the right context.
    use std::io::{Cursor, Read, Seek, Write};

    use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

    use crate::decoders::ArithmeticDecoder;
    use crate::encoders::ArithmeticEncoder;
    use crate::las::rgb::RGB;
    use crate::las::utils::{
        copy_bytes_into_decoder, copy_encoder_content_to, read_and_unpack, inner_buffer_len_of};
    use crate::packers::Packable;
    use crate::record::{
        LayeredFieldCompressor, LayeredFieldDecompressor,
    };

    use super::v2;

    struct LasDecompressionContextRGB {
        models: v2::RGBModels,
        unused: bool,
    }

    impl Default for LasDecompressionContextRGB {
        fn default() -> Self {
            Self {
                models: v2::RGBModels::default(),
                unused: false,
            }
        }
    }

    pub struct LasRGBDecompressor {
        decoder: ArithmeticDecoder<Cursor<Vec<u8>>>,
        changed_rgb: bool,
        requested_rgb: bool,
        layer_size: u32,
        // The last_rgbs are not part of the decompression context
        // as when decompressing, if the current context index has changed since
        // the last call, the index used for the last_rgb may not be the same as the
        // rgb context, not sure if its truly intentional, or if its a 'bug' in laszip
        contexts: [LasDecompressionContextRGB; 4],
        last_rgbs: [RGB; 4],

        last_context_used: usize,
    }

    impl Default for LasRGBDecompressor {
        fn default() -> Self {
            Self {
                decoder: ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new())),
                changed_rgb: false,
                requested_rgb: true,
                layer_size: 0,
                contexts: [
                    LasDecompressionContextRGB::default(),
                    LasDecompressionContextRGB::default(),
                    LasDecompressionContextRGB::default(),
                    LasDecompressionContextRGB::default()
                ],
                last_rgbs: [RGB::default(); 4],
                last_context_used: 0,
            }
        }
    }


    impl<R: Read + Seek> LayeredFieldDecompressor<R> for LasRGBDecompressor {
        fn size_of_field(&self) -> usize {
            std::mem::size_of::<u16>() * 3
        }

        fn init_first_point(
            &mut self,
            src: &mut R,
            first_point: &mut [u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            for rgb_context in &mut self.contexts {
                rgb_context.unused = true;
            }

            self.last_rgbs[*context] = read_and_unpack::<_, RGB>(src, first_point)?;
            self.contexts[*context].unused = false;
            self.last_context_used = *context;
            Ok(())
        }

        fn decompress_field_with(
            &mut self,
            current_point: &mut [u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            let mut last_item = &mut self.last_rgbs[self.last_context_used];

            // If the context changed we may have to do an initialization
            if self.last_context_used != *context {
                self.last_context_used = *context;
                if self.contexts[*context].unused {
                    self.last_rgbs[*context] = *last_item;
                    self.contexts[*context].unused = false;

                    last_item = &mut self.last_rgbs[*context];
                }
            }

            if self.changed_rgb {
                let new = v2::decompress_rgb_using(
                    &mut self.decoder,
                    &mut self.contexts[self.last_context_used].models,
                    last_item
                )?;
                new.pack_into(current_point);
                *last_item = new;
            } else {
                last_item.pack_into(current_point);
            }

            Ok(())
        }

        fn read_layers_sizes(&mut self, src: &mut R) -> std::io::Result<()> {
            self.layer_size = src.read_u32::<LittleEndian>()?;
            Ok(())
        }

        fn read_layers(&mut self, src: &mut R) -> std::io::Result<()> {
            self.changed_rgb = copy_bytes_into_decoder(
                self.requested_rgb,
                self.layer_size as usize,
                &mut self.decoder,
                src,
            )?;
            Ok(())
        }
    }

    pub struct LasRGBCompressor {
        encoder: ArithmeticEncoder<Cursor<Vec<u8>>>,
        rgb_has_changed: bool,
        contexts: [Option<v2::RGBModels>; 4],
        last_rgbs: [Option<RGB>; 4],
        last_context_used: usize,
    }

    impl Default for LasRGBCompressor {
        fn default() -> Self {
            Self {
                encoder: ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new())),
                rgb_has_changed: false,
                contexts: [None, None, None, None],
                last_rgbs: [None; 4],
                last_context_used: 0,
            }
        }
    }

    impl<R: Write> LayeredFieldCompressor<R> for LasRGBCompressor {
        fn size_of_field(&self) -> usize {
            RGB::SIZE
        }

        fn init_first_point(
            &mut self,
            dst: &mut R,
            first_point: &[u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            dst.write_all(first_point)?;
            self.contexts[*context] = Some(v2::RGBModels::default());
            self.last_rgbs[*context] = Some(RGB::unpack_from(first_point));
            self.last_context_used = *context;
            Ok(())
        }

        fn compress_field_with(&mut self, buf: &[u8], context: &mut usize) -> std::io::Result<()> {
            let current_point = RGB::unpack_from(buf);
            let mut last_rgb = self.last_rgbs[self.last_context_used]
                .as_mut()
                .expect("internal error: last value is not initialized");

            if self.last_context_used != *context {
                if self.contexts[*context].is_none() {
                    self.contexts[*context] = Some(v2::RGBModels::default());
                    self.last_rgbs[*context] = Some(*last_rgb);
                    last_rgb = self.last_rgbs[*context].as_mut().unwrap();
                }
                self.last_context_used = *context;
            }

            if *last_rgb != current_point {
                self.rgb_has_changed = true;
            }
            let models = self.contexts[self.last_context_used]
                .as_mut()
                .expect("internal error: context is not initialized");
            v2::compress_rgb_using(&mut self.encoder, models, &current_point, last_rgb)?;
            *last_rgb = current_point;
            Ok(())
        }

        fn write_layers_sizes(&mut self, dst: &mut R) -> std::io::Result<()> {
            if self.rgb_has_changed {
                self.encoder.done()?;
                dst.write_u32::<LittleEndian>(inner_buffer_len_of(&self.encoder) as u32)?;
            }
            Ok(())
        }

        fn write_layers(&mut self, dst: &mut R) -> std::io::Result<()> {
            if self.rgb_has_changed {
                copy_encoder_content_to(&mut self.encoder, dst)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lower_red_changed() {
        let a = RGB {
            red: 0,
            green: 0,
            blue: 0,
        };
        let b = RGB {
            red: 1,
            green: 0,
            blue: 0,
        };

        assert_eq!(ColorDiff::from_points(&a, &b).0, 0b00000001);
        assert_eq!(ColorDiff::from_points(&b, &a).0, 0b01000001);
    }

    #[test]
    fn upper_red_changed() {
        let a = RGB {
            red: 0,
            green: 0,
            blue: 0,
        };
        let b = RGB {
            red: 256,
            green: 0,
            blue: 0,
        };

        assert_eq!(ColorDiff::from_points(&a, &b).0, 0b00000010);
        assert_eq!(ColorDiff::from_points(&b, &a).0, 0b01000010);
    }

    #[test]
    fn lower_green_changes() {
        let a = RGB {
            red: 0,
            green: 0,
            blue: 0,
        };
        let b = RGB {
            red: 0,
            green: 1,
            blue: 0,
        };

        assert_eq!(ColorDiff::from_points(&a, &b).0, 0b00000100);
        assert_eq!(ColorDiff::from_points(&b, &a).0, 0b01000100);
    }

    #[test]
    fn upper_green_changes() {
        let a = RGB {
            red: 0,
            green: 0,
            blue: 0,
        };
        let b = RGB {
            red: 0,
            green: 256,
            blue: 0,
        };

        assert_eq!(ColorDiff::from_points(&a, &b).0, 0b00001000);
        assert_eq!(ColorDiff::from_points(&b, &a).0, 0b01001000);
    }

    #[test]
    fn lower_blue_changes() {
        let a = RGB {
            red: 0,
            green: 0,
            blue: 0,
        };
        let b = RGB {
            red: 0,
            green: 0,
            blue: 1,
        };

        assert_eq!(ColorDiff::from_points(&a, &b).0, 0b00010000);
        assert_eq!(ColorDiff::from_points(&b, &a).0, 0b01010000);
    }

    #[test]
    fn upper_blue_changes() {
        let a = RGB {
            red: 0,
            green: 0,
            blue: 0,
        };
        let b = RGB {
            red: 0,
            green: 0,
            blue: 256,
        };

        assert_eq!(ColorDiff::from_points(&a, &b).0, 0b00100000);
        assert_eq!(ColorDiff::from_points(&b, &a).0, 0b01100000);
    }

    #[test]
    fn test_nothing_changes() {
        let a = RGB::default();
        let b = RGB::default();

        assert_eq!(ColorDiff::from_points(&a, &b).0, 0b00000000);
        assert_eq!(ColorDiff::from_points(&b, &a).0, 0b00000000);
    }
}
