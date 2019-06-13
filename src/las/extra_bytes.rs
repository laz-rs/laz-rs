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

pub mod v1 {
    use std::io::{Read, Write};

    use crate::decoders::ArithmeticDecoder;
    use crate::encoders::ArithmeticEncoder;
    use crate::formats::{FieldCompressor, FieldDecompressor};
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};

    pub struct ExtraBytesCompressor {
        have_last: bool,
        count: usize,
        lasts: Vec<u8>,
        diffs: Vec<u8>,
        models: Vec<ArithmeticModel>,
    }

    impl ExtraBytesCompressor {
        pub fn new(count: usize) -> Self {
            Self {
                have_last: false,
                count,
                lasts: vec![0u8; count],
                diffs: vec![0u8; count],
                models: (0..count)
                    .into_iter()
                    .map(|_i| ArithmeticModelBuilder::new(256).build())
                    .collect(),
            }
        }
    }

    impl<W: Write> FieldCompressor<W> for ExtraBytesCompressor {
        fn size_of_field(&self) -> usize {
            self.count
        }

        fn compress_with(
            &mut self,
            encoder: &mut ArithmeticEncoder<W>,
            buf: &[u8],
        ) -> std::io::Result<()> {
            for i in 0..self.count {
                let current_byte = &buf[i];
                let last = &mut self.lasts[i];

                self.diffs[i] = (*current_byte).wrapping_sub(*last);
                *last = *current_byte;
            }

            if !self.have_last {
                encoder.out_stream().write_all(&self.lasts)?;
                self.have_last = true;
            } else {
                for (diff, mut model) in self.diffs.iter().zip(self.models.iter_mut()) {
                    encoder.encode_symbol(&mut model, *diff as u32)?;
                }
            }
            Ok(())
        }
    }

    pub type ExtraBytesDecompressor = ExtraBytesCompressor;

    impl<R: Read> FieldDecompressor<R> for ExtraBytesDecompressor {
        fn size_of_field(&self) -> usize {
            self.count
        }

        fn decompress_with(
            &mut self,
            decoder: &mut ArithmeticDecoder<R>,
            mut buf: &mut [u8],
        ) -> std::io::Result<()> {
            if !self.have_last {
                decoder.in_stream().read_exact(&mut buf)?;
                self.lasts.copy_from_slice(buf);
                self.have_last = true;
            } else {
                for i in 0..self.count {
                    let diff = &mut self.diffs[i];
                    let last = &mut self.lasts[i];

                    let sym = decoder.decode_symbol(&mut self.models[i])? as u8;

                    *diff = (*last).wrapping_add(sym);
                    buf[i] = *diff;
                    *last = *diff;
                }
            }
            Ok(())
        }
    }
}

// Just re-export v1 as v2 as they are both the same implementation
pub use v1 as v2;
