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

pub(crate) struct ExtraByteWrapper<'a> {
    slc: &'a mut [u8],
}

impl<'a> LasExtraBytes for ExtraByteWrapper<'a> {
    fn extra_bytes(&self) -> &[u8] {
        &self.slc
    }

    fn set_extra_bytes(&mut self, extra_bytes: &[u8]) {
        self.slc.copy_from_slice(extra_bytes);
    }
}

pub mod v1 {
    use std::io::{Read, Write};

    use crate::decoders::ArithmeticDecoder;
    use crate::encoders::ArithmeticEncoder;
    use crate::las::extra_bytes::{ExtraByteWrapper, ExtraBytes, LasExtraBytes};
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::record::{
        BufferFieldCompressor, BufferFieldDecompressor, PointFieldCompressor,
        PointFieldDecompressor,
    };

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

    impl<W: Write, P: LasExtraBytes> PointFieldCompressor<W, P> for LasExtraByteCompressor {
        fn init_first_point(&mut self, dst: &mut W, first_point: &P) -> std::io::Result<()> {
            dst.write_all(first_point.extra_bytes())?;
            self.last_bytes.copy_from_slice(first_point.extra_bytes());
            Ok(())
        }

        fn compress_field_with(
            &mut self,
            encoder: &mut ArithmeticEncoder<W>,
            current_point: &P,
        ) -> std::io::Result<()> {
            let current_bytes = current_point.extra_bytes();
            for i in 0..self.count {
                self.diffs[i] = (current_bytes[i]).wrapping_sub(self.last_bytes[i]);
                self.last_bytes[i] = current_bytes[i];
            }
            for (diff, mut model) in self.diffs.iter().zip(self.models.iter_mut()) {
                encoder.encode_symbol(&mut model, *diff as u32)?;
            }
            Ok(())
        }
    }

    impl<W: Write> BufferFieldCompressor<W> for LasExtraByteCompressor {
        fn size_of_field(&self) -> usize {
            self.count
        }

        fn compress_first(&mut self, mut dst: &mut W, buf: &[u8]) -> std::io::Result<()> {
            let bytes = vec![0u8; self.count];
            let mut current = ExtraBytes { bytes };
            current.set_extra_bytes(buf);
            self.init_first_point(&mut dst, &current)
        }

        fn compress_with(
            &mut self,
            mut encoder: &mut ArithmeticEncoder<W>,
            buf: &[u8],
        ) -> std::io::Result<()> {
            let bytes = vec![0u8; self.count];
            let mut current = ExtraBytes { bytes };
            current.set_extra_bytes(buf);
            self.compress_field_with(&mut encoder, &current)?;
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
                    .into_iter()
                    .map(|_i| ArithmeticModelBuilder::new(256).build())
                    .collect(),
            }
        }
    }

    impl<R: Read, P: LasExtraBytes> PointFieldDecompressor<R, P> for LasExtraByteDecompressor {
        fn init_first_point(&mut self, src: &mut R, first_point: &mut P) -> std::io::Result<()> {
            src.read_exact(&mut self.last_bytes)?;
            first_point.set_extra_bytes(&self.last_bytes);
            Ok(())
        }

        fn decompress_field_with(
            &mut self,
            decoder: &mut ArithmeticDecoder<R>,
            current_point: &mut P,
        ) -> std::io::Result<()> {
            for i in 0..self.count {
                let sym = decoder.decode_symbol(&mut self.models[i])? as u8;
                self.diffs[i] = self.last_bytes[i].wrapping_add(sym);
            }
            current_point.set_extra_bytes(&self.diffs);
            self.last_bytes.clone_from(&self.diffs);
            Ok(())
        }
    }

    impl<R: Read> BufferFieldDecompressor<R> for LasExtraByteDecompressor {
        fn size_of_field(&self) -> usize {
            self.count
        }

        fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()> {
            let mut eb = ExtraByteWrapper { slc: first_point };
            self.init_first_point(src, &mut eb)?;
            Ok(())
        }

        fn decompress_with(
            &mut self,
            mut decoder: &mut ArithmeticDecoder<R>,
            buf: &mut [u8],
        ) -> std::io::Result<()> {
            let mut eb = ExtraByteWrapper { slc: buf };
            self.decompress_field_with(&mut decoder, &mut eb)?;
            Ok(())
        }
    }

}

// Just re-export v1 as v2 as they are both the same implementation
pub use v1 as v2;

pub mod v3 {
    use super::LasExtraBytes;
    use crate::decoders::ArithmeticDecoder;
    use std::io::{Cursor, Read, Seek, Write};

    use crate::las::extra_bytes::ExtraBytes;
    use crate::las::utils::copy_bytes_into_decoder;
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::record::{BufferLayeredFieldDecompressor, LayeredPointFieldDecompressor};
    use byteorder::{LittleEndian, ReadBytesExt};

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

    impl<R: Read + Seek, P: LasExtraBytes> LayeredPointFieldDecompressor<R, P>
        for LasExtraByteDecompressor
    {
        fn init_first_point(
            &mut self,
            src: &mut R,
            first_point: &mut P,
            context: &mut usize,
        ) -> std::io::Result<()> {
            for eb_context in &mut self.contexts {
                eb_context.unused = true;
            }

            let the_context = &mut self.contexts[*context];
            for i in 0..self.num_extra_bytes {
                the_context.last_bytes.bytes[i] = src.read_u8()?
            }

            self.last_context_used = *context;
            the_context.unused = false;
            first_point.set_extra_bytes(the_context.last_bytes.extra_bytes());
            Ok(())
        }

        fn decompress_field_with(
            &mut self,
            current_point: &mut P,
            context: &mut usize,
        ) -> std::io::Result<()> {
            if self.last_context_used != *context {
                if self.contexts[*context].unused {
                    let last_item = self.contexts[self.last_context_used]
                        .last_bytes
                        .bytes
                        .clone();
                    self.contexts[*context]
                        .last_bytes
                        .bytes
                        .copy_from_slice(&last_item);
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
            current_point.set_extra_bytes(&the_context.last_bytes.bytes);
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

    impl<R: Read + Seek> BufferLayeredFieldDecompressor<R> for LasExtraByteDecompressor {
        fn size_of_field(&self) -> usize {
            self.num_extra_bytes
        }

        fn init_first_point(
            &mut self,
            src: &mut R,
            first_point: &mut [u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            //FIXME creating a vec each time seems inefficient
            let mut point = ExtraBytes::new(self.num_extra_bytes);
            <Self as LayeredPointFieldDecompressor<R, ExtraBytes>>::init_first_point(
                self, src, &mut point, context,
            )?;
            let mut cursor = Cursor::new(first_point);
            cursor.write_all(&point.bytes)?;
            Ok(())
        }

        fn decompress_field_with(
            &mut self,
            current_point: &mut [u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            //FIXME creating a vec each time seems inefficient
            let mut point = ExtraBytes::new(self.num_extra_bytes);
            <Self as LayeredPointFieldDecompressor<R, ExtraBytes>>::decompress_field_with(
                self, &mut point, context,
            )?;
            let mut cursor = Cursor::new(current_point);
            cursor.write_all(&point.bytes)?;
            Ok(())
        }

        fn read_layers_sizes(&mut self, src: &mut R) -> std::io::Result<()> {
            <Self as LayeredPointFieldDecompressor<R, ExtraBytes>>::read_layers_sizes(self, src)
        }

        fn read_layers(&mut self, src: &mut R) -> std::io::Result<()> {
            <Self as LayeredPointFieldDecompressor<R, ExtraBytes>>::read_layers(self, src)
        }
    }
}
