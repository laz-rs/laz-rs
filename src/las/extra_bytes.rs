/*
===============================================================================

  PROGRAMMERS:

    martin.isenburg@rapidlasso.com  -  http://rapidlasso.com
    uday.karan@gmail.com - Hobu, Inc.
    andrew.bell.ia@gmail.com - Hobu Inc.

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
//! Defines the different version of compressors and decompressors for ExtraBytes contained in points

/// Just re-export v1 as v2 as they are both the same implementation
pub use v1 as v2;

pub trait LasExtraBytes {
    fn extra_bytes(&self) -> &[u8];
    fn set_extra_bytes(&mut self, extra_bytes: &[u8]);
}

//FIXME the trait for extra bytes should be implemented for Vec<u8>
// to avoid having a struct wrapping it
pub struct ExtraBytes {
    bytes: Vec<u8>,
}

impl ExtraBytes {
    pub fn new(count: usize) -> Self {
        Self {
            bytes: vec![0u8; count],
        }
    }
}

impl LasExtraBytes for ExtraBytes {
    fn extra_bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn set_extra_bytes(&mut self, extra_bytes: &[u8]) {
        self.bytes.copy_from_slice(extra_bytes);
    }
}

pub mod v1 {
    //! The Algorithm is simple:
    //! encode the difference between byte for each extra bytes
    use std::io::{Read, Write};

    use crate::decoders::ArithmeticDecoder;
    use crate::encoders::ArithmeticEncoder;
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::record::{FieldCompressor, FieldDecompressor};

    pub struct LasExtraByteCompressor {
        last_bytes: Vec<u8>,
        count: usize,
        diffs: Vec<u8>,
        models: Vec<ArithmeticModel>,
    }

    impl LasExtraByteCompressor {
        pub fn new(count: usize) -> Self {
            Self {
                last_bytes: vec![0u8; count],
                count,
                diffs: vec![0u8; count],
                models: (0..count)
                    .into_iter()
                    .map(|_i| ArithmeticModelBuilder::new(256).build())
                    .collect(),
            }
        }
    }

    impl<W: Write> FieldCompressor<W> for LasExtraByteCompressor {
        fn size_of_field(&self) -> usize {
            self.count
        }

        fn compress_first(&mut self, dst: &mut W, buf: &[u8]) -> std::io::Result<()> {
            self.last_bytes.copy_from_slice(buf);
            dst.write_all(buf)
        }

        fn compress_with(
            &mut self,
            encoder: &mut ArithmeticEncoder<W>,
            buf: &[u8],
        ) -> std::io::Result<()> {
            let current_bytes = buf;
            for i in 0..self.count {
                self.diffs[i] = (current_bytes[i]).wrapping_sub(self.last_bytes[i]);
                self.last_bytes[i] = current_bytes[i];
            }
            for (diff, mut model) in self.diffs.iter().zip(self.models.iter_mut()) {
                encoder.encode_symbol(&mut model, u32::from(*diff))?;
            }
            Ok(())
        }
    }

    pub struct LasExtraByteDecompressor {
        last_bytes: Vec<u8>,
        count: usize,
        diffs: Vec<u8>,
        models: Vec<ArithmeticModel>,
    }

    impl LasExtraByteDecompressor {
        pub fn new(count: usize) -> Self {
            Self {
                last_bytes: vec![0u8; count],
                count,
                diffs: vec![0u8; count],
                models: (0..count)
                    .map(|_i| ArithmeticModelBuilder::new(256).build())
                    .collect(),
            }
        }
    }

    impl<R: Read> FieldDecompressor<R> for LasExtraByteDecompressor {
        fn size_of_field(&self) -> usize {
            self.count
        }

        fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()> {
            src.read_exact(first_point)?;
            self.last_bytes.copy_from_slice(first_point);
            Ok(())
        }

        //TODO this can probably be 'obtimized'
        fn decompress_with(
            &mut self,
            decoder: &mut ArithmeticDecoder<R>,
            buf: &mut [u8],
        ) -> std::io::Result<()> {
            for i in 0..self.count {
                let sym = decoder.decode_symbol(&mut self.models[i])? as u8;
                self.diffs[i] = self.last_bytes[i].wrapping_add(sym);
            }
            self.last_bytes.copy_from_slice(&self.diffs);
            buf.copy_from_slice(&self.last_bytes);
            Ok(())
        }
    }
}

pub mod v3 {
    //! The algorithm is similar to v1 (& v2), the changes are
    //! that compressor / decompressor uses contexts (4)
    //! and each byte of the extra bytes is encoded in its own layer
    //! with its own encoder
    use std::io::{Cursor, Read, Seek, Write};

    use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

    use crate::decoders::ArithmeticDecoder;
    use crate::encoders::ArithmeticEncoder;
    use crate::las::extra_bytes::ExtraBytes;
    use crate::las::utils::{copy_bytes_into_decoder, copy_encoder_content_to};
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::record::{LayeredFieldCompressor, LayeredFieldDecompressor};

    struct ExtraBytesContext {
        last_bytes: ExtraBytes,
        models: Vec<ArithmeticModel>,
        unused: bool,
    }

    impl ExtraBytesContext {
        pub fn new(count: usize) -> Self {
            Self {
                last_bytes: ExtraBytes::new(count),
                models: (0..count)
                    .map(|_i| ArithmeticModelBuilder::new(256).build())
                    .collect(),
                unused: true,
            }
        }
    }

    pub struct LasExtraByteDecompressor {
        // Each extra bytes has is own layer, thus its own decoder
        decoders: Vec<ArithmeticDecoder<Cursor<Vec<u8>>>>,
        num_bytes_per_layer: Vec<u32>,
        has_byte_changed: Vec<bool>,
        contexts: Vec<ExtraBytesContext>,
        num_extra_bytes: usize,
        last_context_used: usize,
    }

    impl LasExtraByteDecompressor {
        pub fn new(count: usize) -> Self {
            Self {
                decoders: (0..count)
                    .map(|_i| ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new())))
                    .collect(),
                num_bytes_per_layer: vec![0; count],
                has_byte_changed: vec![false; count],
                contexts: (0..4).map(|_i| ExtraBytesContext::new(count)).collect(),
                num_extra_bytes: count,
                last_context_used: 0,
            }
        }
    }

    impl<R: Read + Seek> LayeredFieldDecompressor<R> for LasExtraByteDecompressor {
        fn size_of_field(&self) -> usize {
            self.num_extra_bytes
        }

        fn init_first_point(
            &mut self,
            src: &mut R,
            first_point: &mut [u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            for eb_context in &mut self.contexts {
                eb_context.unused = true;
            }

            let the_context = &mut self.contexts[*context];
            src.read_exact(first_point)?;
            the_context.last_bytes.bytes.copy_from_slice(first_point);

            self.last_context_used = *context;
            the_context.unused = false;
            Ok(())
        }

        fn decompress_field_with(
            &mut self,
            current_point: &mut [u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            if self.last_context_used != *context {
                if self.contexts[*context].unused {
                    let mut new_context = ExtraBytesContext::new(self.num_extra_bytes);
                    new_context
                        .last_bytes
                        .bytes
                        .copy_from_slice(&self.contexts[self.last_context_used].last_bytes.bytes);
                    self.contexts[*context] = new_context;
                }
            }

            let the_context = &mut self.contexts[*context];
            for i in 0..self.num_extra_bytes {
                if self.has_byte_changed[i] {
                    let last_value = &mut the_context.last_bytes.bytes[i];
                    let new_value = u32::from(*last_value)
                        + self.decoders[i].decode_symbol(&mut the_context.models[i])?;
                    *last_value = new_value as u8;
                }
            }
            current_point.copy_from_slice(&the_context.last_bytes.bytes);
            self.last_context_used = *context;
            Ok(())
        }

        fn read_layers_sizes(&mut self, src: &mut R) -> std::io::Result<()> {
            for layer_size in &mut self.num_bytes_per_layer {
                *layer_size = src.read_u32::<LittleEndian>()?;
            }
            Ok(())
        }

        fn read_layers(&mut self, src: &mut R) -> std::io::Result<()> {
            for i in 0..self.num_extra_bytes {
                self.has_byte_changed[i] = copy_bytes_into_decoder(
                    true, // TODO requested bytes
                    self.num_bytes_per_layer[i] as usize,
                    &mut self.decoders[i],
                    src,
                )?;
            }
            Ok(())
        }
    }

    pub struct LasExtraByteCompressor {
        // Each extra bytes has is own layer, thus its own decoder
        encoders: Vec<ArithmeticEncoder<Cursor<Vec<u8>>>>,
        has_byte_changed: Vec<bool>,
        contexts: Vec<ExtraBytesContext>,
        num_extra_bytes: usize,
        last_context_used: usize,
    }

    impl LasExtraByteCompressor {
        pub fn new(count: usize) -> Self {
            Self {
                encoders: (0..count)
                    .map(|_i| ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new())))
                    .collect(),
                has_byte_changed: vec![false; count],
                contexts: (0..4).map(|_i| ExtraBytesContext::new(count)).collect(),
                num_extra_bytes: count,
                last_context_used: 0,
            }
        }
    }

    impl<W: Write> LayeredFieldCompressor<W> for LasExtraByteCompressor {
        fn size_of_field(&self) -> usize {
            self.num_extra_bytes
        }

        fn init_first_point(
            &mut self,
            dst: &mut W,
            first_point: &[u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            for eb_context in &mut self.contexts {
                eb_context.unused = true;
            }

            dst.write_all(first_point)?;
            let the_context = &mut self.contexts[*context];
            the_context.last_bytes.bytes.copy_from_slice(first_point);
            self.last_context_used = *context;
            the_context.unused = false;
            Ok(())
        }

        fn compress_field_with(
            &mut self,
            current_point: &[u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            if self.last_context_used != *context {
                if self.contexts[*context].unused {
                    let mut new_context = ExtraBytesContext::new(self.num_extra_bytes);
                    new_context
                        .last_bytes
                        .bytes
                        .copy_from_slice(&self.contexts[self.last_context_used].last_bytes.bytes);
                    self.contexts[*context] = new_context;
                }
            }
            let the_context = &mut self.contexts[*context];

            for i in 0..self.num_extra_bytes {
                let diff = current_point[i] - the_context.last_bytes.bytes[i];
                self.encoders[i].encode_symbol(&mut the_context.models[i], u32::from(diff))?;
                if diff != 0 {
                    self.has_byte_changed[i] = true;
                    the_context.last_bytes.bytes[i] = current_point[i];
                }
            }

            self.last_context_used = *context;
            Ok(())
        }

        fn write_layers_sizes(&mut self, dst: &mut W) -> std::io::Result<()> {
            for encoder in &mut self.encoders {
                encoder.done()?;
                dst.write_u32::<LittleEndian>(encoder.out_stream().get_ref().len() as u32)?;
            }
            Ok(())
        }

        fn write_layers(&mut self, dst: &mut W) -> std::io::Result<()> {
            for encoder in &mut self.encoders {
                copy_encoder_content_to(encoder, dst)?;
            }
            Ok(())
        }
    }
}
