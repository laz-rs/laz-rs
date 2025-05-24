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
    terms of the Apache Public License 2.0 published by the Apache Software
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

impl From<Vec<u8>> for ExtraBytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self { bytes }
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
    use crate::las::selective::DecompressionSelection;
    use crate::las::utils::{copy_bytes_into_decoder, copy_encoder_content_to};
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::record::{LayeredFieldCompressor, LayeredFieldDecompressor};

    struct ExtraBytesContext {
        models: Vec<ArithmeticModel>,
        unused: bool,
    }

    impl ExtraBytesContext {
        pub fn new(count: usize) -> Self {
            Self {
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
        is_requested: Vec<bool>,
        should_load_bytes: Vec<bool>,
        contexts: Vec<ExtraBytesContext>,
        // Last & contexts are separated for the same reasons as in v3::RGB
        last_bytes: Vec<ExtraBytes>,
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
                is_requested: vec![true; count],
                should_load_bytes: vec![true; count],
                contexts: (0..4).map(|_i| ExtraBytesContext::new(count)).collect(),
                last_bytes: (0..4).map(|_| ExtraBytes::new(count)).collect(),
                num_extra_bytes: count,
                last_context_used: 0,
            }
        }
    }

    impl<R: Read + Seek> LayeredFieldDecompressor<R> for LasExtraByteDecompressor {
        fn size_of_field(&self) -> usize {
            self.num_extra_bytes
        }

        fn set_selection(&mut self, selection: DecompressionSelection) {
            self.is_requested
                .fill(selection.should_decompress_extra_bytes());
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

            src.read_exact(first_point)?;
            self.last_bytes[*context].bytes.copy_from_slice(first_point);

            self.last_context_used = *context;
            self.contexts[*context].unused = false;
            Ok(())
        }

        fn decompress_field_with(
            &mut self,
            current_point: &mut [u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            let mut last_bytes_ptr =
                &mut self.last_bytes[self.last_context_used] as *mut ExtraBytes;
            if self.last_context_used != *context {
                self.last_context_used = *context;
                if self.contexts[*context].unused {
                    let last_bytes = unsafe { &mut *last_bytes_ptr };
                    let new_context = ExtraBytesContext::new(last_bytes.bytes.len());
                    self.contexts[*context] = new_context;
                    self.contexts[*context].unused = false;
                    self.last_bytes[*context]
                        .bytes
                        .copy_from_slice(&last_bytes.bytes);
                    last_bytes_ptr = &mut self.last_bytes[*context] as &mut _;
                }
            }

            let last_bytes = unsafe { &mut *last_bytes_ptr };
            let the_context = &mut self.contexts[*context];
            for i in 0..self.num_extra_bytes {
                if self.should_load_bytes[i] {
                    let last_value = &mut last_bytes.bytes[i];
                    let diff = self.decoders[i].decode_symbol(&mut the_context.models[i])?;
                    let new_value = last_value.wrapping_add(diff as u8);
                    *last_value = new_value as u8;
                }
            }
            current_point.copy_from_slice(&last_bytes.bytes);
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
                self.should_load_bytes[i] = copy_bytes_into_decoder(
                    self.is_requested[i],
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
        // Last & contexts are separated for the same reasons as in v3::RGB
        last_bytes: Vec<ExtraBytes>,
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
                last_bytes: (0..4).map(|_i| ExtraBytes::new(count)).collect(),
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
            self.last_bytes[*context].bytes.copy_from_slice(first_point);
            self.last_context_used = *context;
            self.contexts[*context].unused = false;
            Ok(())
        }

        fn compress_field_with(
            &mut self,
            current_point: &[u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            let mut last_bytes_ptr =
                &mut self.last_bytes[self.last_context_used] as *mut ExtraBytes;
            if self.last_context_used != *context {
                self.last_context_used = *context;
                if self.contexts[*context].unused {
                    let last_bytes = unsafe { &mut *last_bytes_ptr };
                    let new_context = ExtraBytesContext::new(last_bytes.bytes.len());
                    self.contexts[*context] = new_context;
                    self.contexts[*context].unused = false;
                    self.last_bytes[*context]
                        .bytes
                        .copy_from_slice(&last_bytes.bytes);
                    last_bytes_ptr = &mut self.last_bytes[*context] as &mut _;
                }
            }

            let last_bytes = unsafe { &mut *last_bytes_ptr };
            let the_context = &mut self.contexts[*context];
            for i in 0..self.num_extra_bytes {
                let diff = current_point[i].wrapping_sub(last_bytes.bytes[i]);
                self.encoders[i].encode_symbol(&mut the_context.models[i], u32::from(diff))?;
                if diff != 0 {
                    self.has_byte_changed[i] = true;
                    last_bytes.bytes[i] = current_point[i];
                }
            }

            self.last_context_used = *context;
            Ok(())
        }

        fn write_layers_sizes(&mut self, dst: &mut W) -> std::io::Result<()> {
            for (encoder, has_changed) in self
                .encoders
                .iter_mut()
                .zip(self.has_byte_changed.iter().copied())
            {
                encoder.done()?;
                let num_bytes = if has_changed {
                    encoder.get_mut().get_ref().len() as u32
                } else {
                    0
                };
                dst.write_u32::<LittleEndian>(num_bytes)?;
            }

            Ok(())
        }

        fn write_layers(&mut self, dst: &mut W) -> std::io::Result<()> {
            for (encoder, has_changed) in self
                .encoders
                .iter_mut()
                .zip(self.has_byte_changed.iter().copied())
            {
                if has_changed {
                    copy_encoder_content_to(encoder, dst)?;
                }
            }
            Ok(())
        }
    }
}
