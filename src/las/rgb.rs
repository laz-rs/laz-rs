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

use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use num_traits::clamp;

use crate::las::utils::flag_diff;
use crate::packers::Packable;

fn u8_clamp(n: i32) -> u8 {
    clamp(n, i32::from(std::u8::MIN), i32::from(std::u8::MAX)) as u8
}

#[inline(always)]
fn lower_byte(n: u16) -> u8 {
    (n & 0x00_FF) as u8
}

#[inline(always)]
fn upper_byte(n: u16) -> u8 {
    (n >> 8) as u8
}

pub trait LasRGB {
    fn red(&self) -> u16;
    fn green(&self) -> u16;
    fn blue(&self) -> u16;

    fn set_red(&mut self, new_val: u16);
    fn set_green(&mut self, new_val: u16);
    fn set_blue(&mut self, new_val: u16);

    fn read_from<R: Read>(&mut self, src: &mut R) -> std::io::Result<()> {
        self.set_red(src.read_u16::<LittleEndian>()?);
        self.set_green(src.read_u16::<LittleEndian>()?);
        self.set_blue(src.read_u16::<LittleEndian>()?);
        Ok(())
    }

    fn write_to<W: Write>(&self, dst: &mut W) -> std::io::Result<()> {
        dst.write_u16::<LittleEndian>(self.red())?;
        dst.write_u16::<LittleEndian>(self.green())?;
        dst.write_u16::<LittleEndian>(self.blue())?;
        Ok(())
    }

    fn set_fields_from<P: LasRGB>(&mut self, other: &P) {
        self.set_red(other.red());
        self.set_green(other.green());
        self.set_blue(other.blue());
    }
}

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

pub(crate) struct RGBWrapper<'a> {
    slc: &'a mut [u8],
}

impl<'a> RGBWrapper<'a> {
    fn new(slc: &'a mut [u8]) -> Self {
        if slc.len() < 6 {
            panic!("RGB Wrapper expected a buffer a 6 bytes");
        } else {
            Self { slc }
        }
    }
}

impl<'a> LasRGB for RGBWrapper<'a> {
    fn red(&self) -> u16 {
        unsafe { u16::from_le_bytes([*self.slc.get_unchecked(0), *self.slc.get_unchecked(1)]) }
    }

    fn green(&self) -> u16 {
        unsafe { u16::from_le_bytes([*self.slc.get_unchecked(2), *self.slc.get_unchecked(3)]) }
    }

    fn blue(&self) -> u16 {
        unsafe { u16::from_le_bytes([*self.slc.get_unchecked(4), *self.slc.get_unchecked(5)]) }
    }

    fn set_red(&mut self, new_val: u16) {
        unsafe {
            self.slc
                .get_unchecked_mut(0..2)
                .copy_from_slice(&new_val.to_le_bytes());
        }
    }

    fn set_green(&mut self, new_val: u16) {
        unsafe {
            self.slc
                .get_unchecked_mut(2..4)
                .copy_from_slice(&new_val.to_le_bytes());
        }
    }

    fn set_blue(&mut self, new_val: u16) {
        unsafe {
            self.slc
                .get_unchecked_mut(4..6)
                .copy_from_slice(&new_val.to_le_bytes());
        }
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
    type Type = RGB;

    fn unpack_from(input: &[u8]) -> Self::Type {
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
    use std::io::{Read, Write};
    use std::mem::size_of;

    use crate::compressors::{IntegerCompressor, IntegerCompressorBuilder};
    use crate::decoders::ArithmeticDecoder;
    use crate::decompressors::{IntegerDecompressor, IntegerDecompressorBuilder};
    use crate::encoders::ArithmeticEncoder;
    use crate::las::rgb::{LasRGB, RGBWrapper};
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;
    use crate::record::{
        BufferFieldCompressor, BufferFieldDecompressor, PointFieldCompressor,
        PointFieldDecompressor,
    };

    use super::{lower_byte, upper_byte, ColorDiff, RGB};

    pub struct LasRGBDecompressor {
        last: RGB,
        byte_used_model: ArithmeticModel,
        ic_rgb: IntegerDecompressor,
    }

    impl LasRGBDecompressor {
        pub fn new() -> Self {
            Self {
                last: Default::default(),
                byte_used_model: ArithmeticModelBuilder::new(64).build(),
                ic_rgb: IntegerDecompressorBuilder::new()
                    .bits(8) // 8 bits, because we encode byte by byte
                    .contexts(6) // there are 6 bytes in a RGB component
                    .build_initialized(),
            }
        }
    }

    impl<R: Read, P: LasRGB> PointFieldDecompressor<R, P> for LasRGBDecompressor {
        fn init_first_point(
            &mut self,
            mut src: &mut R,
            first_point: &mut P,
        ) -> std::io::Result<()> {
            self.last.read_from(&mut src)?;
            first_point.set_fields_from(&self.last);
            Ok(())
        }
        //TODO mutate directly instead of using set methods
        fn decompress_field_with(
            &mut self,
            mut decoder: &mut ArithmeticDecoder<R>,
            current_point: &mut P,
        ) -> std::io::Result<()> {
            let color_diff =
                ColorDiff::new(decoder.decode_symbol(&mut self.byte_used_model)? as u8);

            if color_diff.lower_red_byte_changed() {
                self.last.set_red(self.ic_rgb.decompress(
                    &mut decoder,
                    lower_byte(self.last.red()) as i32,
                    0,
                )? as u16);
            }

            if color_diff.upper_red_byte_changed() {
                self.last.set_red(
                    self.last.red()
                        | self.ic_rgb.decompress(
                            &mut decoder,
                            upper_byte(self.last.red()) as i32,
                            1,
                        )? as u16,
                );
            }

            if color_diff.lower_green_byte_changed() {
                self.last.set_green(self.ic_rgb.decompress(
                    &mut decoder,
                    lower_byte(self.last.green()) as i32,
                    2,
                )? as u16);
            }

            if color_diff.upper_green_byte_changed() {
                self.last.set_green(
                    self.last.green()
                        | self.ic_rgb.decompress(
                            &mut decoder,
                            upper_byte(self.last.green()) as i32,
                            3,
                        )? as u16,
                );
            }

            if color_diff.lower_blue_byte_changed() {
                self.last.set_blue(self.ic_rgb.decompress(
                    &mut decoder,
                    lower_byte(self.last.blue()) as i32,
                    4,
                )? as u16);
            }

            if color_diff.upper_blue_byte_changed() {
                self.last.set_blue(
                    self.last.blue()
                        | self.ic_rgb.decompress(
                            &mut decoder,
                            upper_byte(self.last.blue()) as i32,
                            5,
                        )? as u16,
                );
            }
            current_point.set_fields_from(&self.last);
            Ok(())
        }
    }

    pub struct LasRGBCompressor {
        last: RGB,
        byte_used_model: ArithmeticModel,
        ic_rgb: IntegerCompressor,
    }

    impl LasRGBCompressor {
        pub fn new() -> Self {
            Self {
                last: Default::default(),
                byte_used_model: ArithmeticModelBuilder::new(64).build(),
                ic_rgb: IntegerCompressorBuilder::new()
                    .bits(8)
                    .contexts(6)
                    .build_initialized(),
            }
        }
    }

    impl<W: Write, P: LasRGB> PointFieldCompressor<W, P> for LasRGBCompressor {
        fn init_first_point(&mut self, mut dst: &mut W, first_point: &P) -> std::io::Result<()> {
            first_point.write_to(&mut dst)?;
            self.last.set_fields_from(first_point);
            Ok(())
        }

        fn compress_field_with(
            &mut self,
            mut encoder: &mut ArithmeticEncoder<W>,
            current_point: &P,
        ) -> std::io::Result<()> {
            let sym = ((lower_byte(self.last.red()) != lower_byte(current_point.red())) as u8) << 0
                | ((upper_byte(self.last.red()) != upper_byte(current_point.red())) as u8) << 1
                | ((lower_byte(self.last.green()) != lower_byte(current_point.green())) as u8) << 2
                | ((upper_byte(self.last.green()) != upper_byte(current_point.green())) as u8) << 3
                | ((lower_byte(self.last.blue()) != lower_byte(current_point.blue())) as u8) << 4
                | ((upper_byte(self.last.blue()) != upper_byte(current_point.blue())) as u8) << 5;

            encoder.encode_symbol(&mut self.byte_used_model, sym as u32)?;
            let color_diff = ColorDiff::new(sym);

            if color_diff.lower_red_byte_changed() {
                self.ic_rgb.compress(
                    &mut encoder,
                    lower_byte(self.last.red()) as i32,
                    lower_byte(current_point.red()) as i32,
                    0,
                )?;
            }

            if color_diff.upper_red_byte_changed() {
                self.ic_rgb.compress(
                    &mut encoder,
                    upper_byte(self.last.red()) as i32,
                    upper_byte(current_point.red()) as i32,
                    1,
                )?;
            }

            if color_diff.lower_green_byte_changed() {
                self.ic_rgb.compress(
                    &mut encoder,
                    lower_byte(self.last.green()) as i32,
                    lower_byte(current_point.green()) as i32,
                    2,
                )?;
            }

            if color_diff.upper_green_byte_changed() {
                self.ic_rgb.compress(
                    &mut encoder,
                    upper_byte(self.last.green()) as i32,
                    upper_byte(current_point.green()) as i32,
                    3,
                )?;
            }

            if color_diff.lower_blue_byte_changed() {
                self.ic_rgb.compress(
                    &mut encoder,
                    lower_byte(self.last.blue()) as i32,
                    lower_byte(current_point.blue()) as i32,
                    4,
                )?;
            }

            if color_diff.upper_blue_byte_changed() {
                self.ic_rgb.compress(
                    &mut encoder,
                    upper_byte(self.last.blue()) as i32,
                    upper_byte(current_point.blue()) as i32,
                    5,
                )?;
            }
            self.last.set_fields_from(current_point);
            Ok(())
        }
    }

    impl<R: Read> BufferFieldDecompressor<R> for LasRGBDecompressor {
        fn size_of_field(&self) -> usize {
            3 * size_of::<u16>()
        }

        fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()> {
            let mut current = RGBWrapper { slc: first_point };
            self.init_first_point(src, &mut current)?;
            Ok(())
        }

        fn decompress_with(
            &mut self,
            mut decoder: &mut ArithmeticDecoder<R>,
            buf: &mut [u8],
        ) -> std::io::Result<()> {
            let mut current = RGBWrapper { slc: buf };
            self.decompress_field_with(&mut decoder, &mut current)?;
            Ok(())
        }
    }

    impl<W: Write> BufferFieldCompressor<W> for LasRGBCompressor {
        fn size_of_field(&self) -> usize {
            6
        }

        fn compress_first(&mut self, mut dst: &mut W, buf: &[u8]) -> std::io::Result<()> {
            let current = RGB::unpack_from(buf);
            self.init_first_point(&mut dst, &current)
        }

        fn compress_with(
            &mut self,
            mut encoder: &mut ArithmeticEncoder<W>,
            buf: &[u8],
        ) -> std::io::Result<()> {
            let current = RGB::unpack_from(buf);
            self.compress_field_with(&mut encoder, &current)?;
            Ok(())
        }
    }
}

pub mod v2 {
    use std::io::{Read, Write};

    use crate::decoders::ArithmeticDecoder;
    use crate::encoders::ArithmeticEncoder;
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;
    use crate::record::{
        BufferFieldCompressor, BufferFieldDecompressor, PointFieldCompressor,
        PointFieldDecompressor,
    };

    use super::{lower_byte, u8_clamp, upper_byte, ColorDiff, RGB};
    use crate::las::rgb::{LasRGB, RGBWrapper};

    pub struct LasRGBCompressor {
        last: RGB,
        byte_used: ArithmeticModel,
        rgb_diff_0: ArithmeticModel,
        rgb_diff_1: ArithmeticModel,
        rgb_diff_2: ArithmeticModel,
        rgb_diff_3: ArithmeticModel,
        rgb_diff_4: ArithmeticModel,
        rgb_diff_5: ArithmeticModel,
    }

    impl LasRGBCompressor {
        pub fn new() -> Self {
            Self {
                last: Default::default(),
                byte_used: ArithmeticModelBuilder::new(128).build(),
                rgb_diff_0: ArithmeticModelBuilder::new(256).build(),
                rgb_diff_1: ArithmeticModelBuilder::new(256).build(),
                rgb_diff_2: ArithmeticModelBuilder::new(256).build(),
                rgb_diff_3: ArithmeticModelBuilder::new(256).build(),
                rgb_diff_4: ArithmeticModelBuilder::new(256).build(),
                rgb_diff_5: ArithmeticModelBuilder::new(256).build(),
            }
        }
    }

    impl<W: Write, P: LasRGB> PointFieldCompressor<W, P> for LasRGBCompressor {
        fn init_first_point(&mut self, mut dst: &mut W, first_point: &P) -> std::io::Result<()> {
            first_point.write_to(&mut dst)?;
            self.last.set_fields_from(first_point);
            Ok(())
        }

        fn compress_field_with(
            &mut self,
            encoder: &mut ArithmeticEncoder<W>,
            current_point: &P,
        ) -> std::io::Result<()> {
            let mut diff_l = 0i32;
            let mut diff_h = 0i32;
            let mut corr;

            let color_diff = ColorDiff::from_points(current_point, &self.last);
            encoder.encode_symbol(&mut self.byte_used, color_diff.0 as u32)?;

            if color_diff.lower_red_byte_changed() {
                diff_l = lower_byte(current_point.red()) as i32 - lower_byte(self.last.red) as i32;
                encoder.encode_symbol(&mut self.rgb_diff_0, diff_l as u8 as u32)?;
            }

            if color_diff.upper_red_byte_changed() {
                diff_h = upper_byte(current_point.red()) as i32 - upper_byte(self.last.red) as i32;
                encoder.encode_symbol(&mut self.rgb_diff_1, diff_h as u8 as u32)?;
            }
            if (color_diff.0 & (1 << 6)) != 0 {
                if color_diff.lower_green_byte_changed() {
                    corr = lower_byte(current_point.green()) as i32
                        - u8_clamp(diff_l + lower_byte(self.last.green) as i32) as i32;
                    encoder.encode_symbol(&mut self.rgb_diff_2, corr as u8 as u32)?;
                }

                if color_diff.lower_blue_byte_changed() {
                    diff_l = (diff_l + lower_byte(current_point.green()) as i32
                        - lower_byte(self.last.green) as i32)
                        / 2;
                    corr = lower_byte(current_point.blue()) as i32
                        - u8_clamp(diff_l + lower_byte(self.last.blue) as i32) as i32;
                    encoder.encode_symbol(&mut self.rgb_diff_4, corr as u8 as u32)?;
                }

                if color_diff.upper_green_byte_changed() {
                    corr = upper_byte(current_point.green()) as i32
                        - u8_clamp(diff_h + upper_byte(self.last.green) as i32) as i32;
                    encoder.encode_symbol(&mut self.rgb_diff_3, corr as u8 as u32)?;
                }

                if color_diff.upper_blue_byte_changed() {
                    diff_h = (diff_h + upper_byte(current_point.green()) as i32
                        - upper_byte(self.last.green) as i32)
                        / 2;
                    corr = upper_byte(current_point.blue()) as i32
                        - u8_clamp(diff_h + upper_byte(self.last.blue) as i32) as i32;
                    encoder.encode_symbol(&mut self.rgb_diff_5, corr as u8 as u32)?;
                }
            }
            self.last.set_fields_from(current_point);
            Ok(())
        }
    }

    pub struct LasRGBDecompressor {
        pub(crate) last: RGB,
        pub(crate) byte_used: ArithmeticModel,
        pub(crate) rgb_diff_0: ArithmeticModel,
        pub(crate) rgb_diff_1: ArithmeticModel,
        pub(crate) rgb_diff_2: ArithmeticModel,
        pub(crate) rgb_diff_3: ArithmeticModel,
        pub(crate) rgb_diff_4: ArithmeticModel,
        pub(crate) rgb_diff_5: ArithmeticModel,
    }

    impl LasRGBDecompressor {
        pub fn new() -> Self {
            Self {
                last: Default::default(),
                byte_used: ArithmeticModelBuilder::new(128).build(),
                rgb_diff_0: ArithmeticModelBuilder::new(256).build(),
                rgb_diff_1: ArithmeticModelBuilder::new(256).build(),
                rgb_diff_2: ArithmeticModelBuilder::new(256).build(),
                rgb_diff_3: ArithmeticModelBuilder::new(256).build(),
                rgb_diff_4: ArithmeticModelBuilder::new(256).build(),
                rgb_diff_5: ArithmeticModelBuilder::new(256).build(),
            }
        }
    }

    impl<R: Read, P: LasRGB> PointFieldDecompressor<R, P> for LasRGBDecompressor {
        fn init_first_point(
            &mut self,
            mut src: &mut R,
            first_point: &mut P,
        ) -> std::io::Result<()> {
            first_point.read_from(&mut src)?;
            self.last.set_fields_from(first_point);
            Ok(())
        }

        fn decompress_field_with(
            &mut self,
            decoder: &mut ArithmeticDecoder<R>,
            current_point: &mut P,
        ) -> std::io::Result<()> {
            let sym = decoder.decode_symbol(&mut self.byte_used)?;
            let color_diff = ColorDiff { 0: sym as u8 };

            let mut this_val = RGB::default();
            let mut corr;
            let mut diff;

            if color_diff.lower_red_byte_changed() {
                corr = decoder.decode_symbol(&mut self.rgb_diff_0)? as u8;
                this_val.red = corr.wrapping_add(lower_byte(self.last.red())) as u16;
            } else {
                this_val.red = self.last.red() & 0xFF;
            }

            if color_diff.upper_red_byte_changed() {
                corr = decoder.decode_symbol(&mut self.rgb_diff_1)? as u8;
                this_val.red |= (corr.wrapping_add(upper_byte(self.last.red())) as u16) << 8;
            } else {
                this_val.red |= self.last.red() & 0xFF00;
            }

            if (sym & (1 << 6)) != 0 {
                diff = lower_byte(this_val.red) as i32 - lower_byte(self.last.red()) as i32;

                if color_diff.lower_green_byte_changed() {
                    corr = decoder.decode_symbol(&mut self.rgb_diff_2)? as u8;
                    this_val.green = corr
                        .wrapping_add(u8_clamp(diff + lower_byte(self.last.green()) as i32) as u8)
                        as u16;
                } else {
                    this_val.green = self.last.green() & 0x00FF;
                }

                if color_diff.lower_blue_byte_changed() {
                    corr = decoder.decode_symbol(&mut self.rgb_diff_4)? as u8;
                    diff = (diff + lower_byte(this_val.green) as i32
                        - lower_byte(self.last.green()) as i32)
                        / 2;
                    this_val.blue = (corr
                        .wrapping_add(u8_clamp(diff + lower_byte(self.last.blue) as i32) as u8))
                        as u16;
                } else {
                    this_val.blue = self.last.blue() & 0x00FF;
                }

                diff = upper_byte(this_val.red) as i32 - upper_byte(self.last.red) as i32;
                if color_diff.upper_green_byte_changed() {
                    corr = decoder.decode_symbol(&mut self.rgb_diff_3)? as u8;
                    this_val.green |= (corr
                        .wrapping_add(u8_clamp(diff + upper_byte(self.last.green) as i32))
                        as u16)
                        << 8;
                } else {
                    this_val.green |= self.last.green() & 0xFF00;
                }

                if color_diff.upper_blue_byte_changed() {
                    corr = decoder.decode_symbol(&mut self.rgb_diff_5)? as u8;
                    diff = (diff + upper_byte(this_val.green) as i32
                        - upper_byte(self.last.green) as i32)
                        / 2;

                    this_val.blue |= ((corr
                        .wrapping_add(u8_clamp(diff + upper_byte(self.last.blue) as i32)))
                        as u16)
                        << 8;
                } else {
                    this_val.blue |= self.last.blue & 0xFF00;
                }
            } else {
                this_val.green = this_val.red;
                this_val.blue = this_val.red;
            }

            current_point.set_fields_from(&this_val);
            self.last = this_val;
            Ok(())
        }
    }

    impl<W: Write> BufferFieldCompressor<W> for LasRGBCompressor {
        fn size_of_field(&self) -> usize {
            3 * std::mem::size_of::<u16>()
        }

        fn compress_first(&mut self, mut dst: &mut W, buf: &[u8]) -> std::io::Result<()> {
            let this_val = super::RGB::unpack_from(&buf);
            self.init_first_point(&mut dst, &this_val)
        }

        fn compress_with(
            &mut self,
            mut encoder: &mut ArithmeticEncoder<W>,
            buf: &[u8],
        ) -> std::io::Result<()> {
            let this_val = super::RGB::unpack_from(&buf);
            self.compress_field_with(&mut encoder, &this_val)
        }
    }

    impl<R: Read> BufferFieldDecompressor<R> for LasRGBDecompressor {
        fn size_of_field(&self) -> usize {
            6
        }

        fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()> {
            let mut current = RGBWrapper::new(first_point);
            self.init_first_point(src, &mut current)?;
            Ok(())
        }

        fn decompress_with(
            &mut self,
            mut decoder: &mut ArithmeticDecoder<R>,
            buf: &mut [u8],
        ) -> std::io::Result<()> {
            let mut current = RGBWrapper::new(buf);
            self.decompress_field_with(&mut decoder, &mut current)?;
            Ok(())
        }
    }
}

pub mod v3 {
    //! Version 3 of the compression / decompression algorithm
    //! is the same as the version 2, but with the support for the contexts system
    use super::v2::LasRGBDecompressor as LasRGBDecompressorV2;
    use crate::decoders::ArithmeticDecoder;
    use crate::las::rgb::{LasRGB, RGB};
    use crate::las::utils::copy_bytes_into_decoder;
    use crate::record::{
        BufferLayeredFieldDecompressor, LayeredPointFieldDecompressor, PointFieldDecompressor,
    };
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::{Cursor, Read, Seek};

    struct LasContextRGB {
        decompressor: LasRGBDecompressorV2,
        unused: bool,
    }

    impl LasContextRGB {
        fn from_rgb(rgb: &RGB) -> Self {
            let mut me = Self {
                decompressor: LasRGBDecompressorV2::new(),
                unused: false,
            };
            me.decompressor.last = *rgb;
            me
        }
    }

    pub struct LasRGBDecompressor {
        pub(crate) decoder: ArithmeticDecoder<Cursor<Vec<u8>>>,
        pub(crate) changed_rgb: bool,
        pub(crate) requested_rgb: bool,
        layer_size: u32,
        // 4
        contexts: Vec<LasContextRGB>,
        last_context_used: usize,
    }

    impl LasRGBDecompressor {
        pub fn new() -> Self {
            let rgb = RGB::default();
            Self {
                decoder: ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new())),
                changed_rgb: false,
                requested_rgb: true,
                layer_size: 0,
                contexts: (0..4)
                    .into_iter()
                    .map(|_i| LasContextRGB::from_rgb(&rgb))
                    .collect(),
                last_context_used: 0,
            }
        }
    }

    impl<R: Read + Seek, P: LasRGB> LayeredPointFieldDecompressor<R, P> for LasRGBDecompressor {
        fn init_first_point(
            &mut self,
            src: &mut R,
            first_point: &mut P,
            context: &mut usize,
        ) -> std::io::Result<()> {
            for rgb_context in &mut self.contexts {
                rgb_context.unused = true;
            }

            let rgb = &mut self.contexts[*context].decompressor.last;
            rgb.read_from(src)?;
            first_point.set_fields_from(rgb);
            Ok(())
        }

        fn decompress_field_with(
            &mut self,
            current_point: &mut P,
            context: &mut usize,
        ) -> std::io::Result<()> {
            // If the context changed we may have to do an initialization
            if self.last_context_used != *context {
                if self.contexts[*context].unused {
                    self.contexts[*context] = LasContextRGB::from_rgb(
                        &self.contexts[self.last_context_used].decompressor.last,
                    )
                }
            }

            let the_context = &mut self.contexts[*context];
            if self.changed_rgb {
                the_context
                    .decompressor
                    .decompress_field_with(&mut self.decoder, current_point)?;
            }

            self.last_context_used = *context;
            current_point.set_fields_from(&the_context.decompressor.last);
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

    impl_buffer_decompressor_for_typed_decompressor!(LasRGBDecompressor, RGB);
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
