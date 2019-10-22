//! Defines the compressors and decompressors for the Near Infrared (Nir) data


pub trait LasNIR {
    fn nir(&self) -> u16;
    fn set_nir(&mut self, new_val: u16);
}

#[derive(Default, Copy, Clone, PartialOrd, PartialEq, Debug)]
pub struct Nir(u16);

impl LasNIR for Nir {
    fn nir(&self) -> u16 {
        self.0
    }

    fn set_nir(&mut self, new_val: u16) {
        self.0 = new_val;
    }
}

impl Nir {
    pub const SIZE: usize = 2;
}

pub mod v3 {
    use std::io::{Cursor, Read, Seek};

    use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

    use crate::decoders::ArithmeticDecoder;
    use crate::encoders::ArithmeticEncoder;
    use crate::las::nir::{LasNIR, Nir};
    use crate::las::utils::{
        copy_encoder_content_to, lower_byte, lower_byte_changed, read_and_unpack, upper_byte,
        upper_byte_changed,
    };
    use crate::las::utils::copy_bytes_into_decoder;
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;
    use crate::record::{LayeredFieldCompressor, LayeredFieldDecompressor};

    struct NirContext {
        bytes_used_model: ArithmeticModel,
        lower_byte_diff_model: ArithmeticModel,
        upper_byte_diff_model: ArithmeticModel,
        unused: bool,
    }

    impl Default for NirContext {
        fn default() -> Self {
            Self {
                bytes_used_model: ArithmeticModelBuilder::new(4).build(),
                lower_byte_diff_model: ArithmeticModelBuilder::new(256).build(),
                upper_byte_diff_model: ArithmeticModelBuilder::new(256).build(),
                unused: false,
            }
        }
    }

    //TODO Selective
    pub struct LasNIRDecompressor {
        decoder: ArithmeticDecoder<Cursor<Vec<u8>>>,
        changed_nir: bool,
        layer_size: u32,
        last_context_used: usize,
        // Last & contexts are separated for the same reasons as in v3::RGB
        contexts: [NirContext; 4],
        last_nirs: [u16; 4],
    }

    impl LasNIRDecompressor {
        pub fn new() -> Self {
            Self {
                decoder: ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new())),
                contexts: [
                    NirContext::default(),
                    NirContext::default(),
                    NirContext::default(),
                    NirContext::default(),
                ],
                changed_nir: false,
                layer_size: 0,
                last_context_used: 0,
                last_nirs: [0u16; 4],
            }
        }
    }

    impl<R: Read + Seek> LayeredFieldDecompressor<R> for LasNIRDecompressor {
        fn size_of_field(&self) -> usize {
            std::mem::size_of::<u16>()
        }

        fn init_first_point(
            &mut self,
            src: &mut R,
            first_point: &mut [u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            for ctx in &mut self.contexts {
                ctx.unused = true;
            }

            self.last_nirs[*context] = read_and_unpack::<_, u16>(src, first_point)?;
            self.contexts[*context].unused = false;
            self.last_context_used = *context;
            Ok(())
        }

        fn decompress_field_with(
            &mut self,
            current_point: &mut [u8],
            context: &mut usize,
        ) -> std::io::Result<()> {

            let mut last_nir = &mut self.last_nirs[self.last_context_used];
            if self.last_context_used != *context {
                self.last_context_used = *context;
                if self.contexts[*context].unused {
                    self.last_nirs[*context] = *last_nir;
                    self.contexts[*context].unused = false;
                    last_nir = &mut self.last_nirs[*context];
                }
            }

            let the_context = &mut self.contexts[self.last_context_used];
            if self.changed_nir {
                let mut new_nir: u16;
                let sym =self.decoder
                    .decode_symbol(
                    &mut the_context.bytes_used_model)?;

                if is_nth_bit_set!(sym, 0) {
                    let diff = self
                        .decoder
                        .decode_symbol(
                            &mut the_context.lower_byte_diff_model)? as u8;
                    new_nir = u16::from(diff.wrapping_add(lower_byte(*last_nir)));
                } else {
                    new_nir = *last_nir & 0x00FF;
                }

                if is_nth_bit_set!(sym, 1) {
                    let diff = self
                        .decoder
                        .decode_symbol(
                            &mut the_context.upper_byte_diff_model)? as u8;
                    let upper_byte = u16::from(diff.wrapping_add(upper_byte(*last_nir)));
                    new_nir |= (upper_byte << 8) & 0xFF00;
                } else {
                    new_nir |= *last_nir & 0xFF00;
                }
                *last_nir = new_nir;
            }
            last_nir.pack_into(current_point);
            Ok(())
        }

        fn read_layers_sizes(&mut self, src: &mut R) -> std::io::Result<()> {
            self.layer_size = src.read_u32::<LittleEndian>()?;
            Ok(())
        }

        fn read_layers(&mut self, src: &mut R) -> std::io::Result<()> {
            self.changed_nir = copy_bytes_into_decoder(
                true, //TODO
                self.layer_size as usize,
                &mut self.decoder,
                src,
            )?;
            Ok(())
        }
    }

    pub struct LasNIRCompressor {
        encoder: ArithmeticEncoder<Cursor<Vec<u8>>>,
        has_nir_changed: bool,
        last_context_used: usize,
        contexts: Vec<NirContext>,
        last_nirs: [u16; 4]
    }

    impl LasNIRCompressor {
        pub fn new() -> Self {
            Self {
                encoder: ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new())),
                contexts: vec![
                    NirContext::default(),
                    NirContext::default(),
                    NirContext::default(),
                    NirContext::default(),
                ],
                has_nir_changed: false,
                last_context_used: 0,
                last_nirs: [0u16; 4]
            }
        }
    }

    impl<R: std::io::Write> LayeredFieldCompressor<R> for LasNIRCompressor {
        fn size_of_field(&self) -> usize {
            std::mem::size_of::<u16>()
        }

        fn init_first_point(
            &mut self,
            dst: &mut R,
            first_point: &[u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            for ctx in &mut self.contexts {
                ctx.unused = true;
            }

            dst.write_all(first_point)?;
            self.last_nirs[*context] = u16::unpack_from(first_point);
            self.last_context_used = *context;
            Ok(())
        }

        fn compress_field_with(
            &mut self,
            current_point: &[u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            let current_point = Nir {
                0: u16::unpack_from(current_point),
            };

            if self.last_context_used != *context {
                if self.contexts[*context].unused {
                    self.last_nirs[*context] = self.last_nirs[self.last_context_used];
                    self.contexts[*context].unused = false;
                }
                self.last_context_used = *context;
            };
            let last_nir = &mut self.last_nirs[self.last_context_used];
            let the_context = &mut self.contexts[self.last_context_used];

            if current_point.nir() != *last_nir {
                self.has_nir_changed = true;
            }

            let sym = lower_byte_changed(current_point.nir(), *last_nir) as u8
                | (upper_byte_changed(current_point.nir(), *last_nir) as u8) << 1;

            if is_nth_bit_set!(sym, 0) {
                let corr = lower_byte(current_point.nir()).wrapping_sub(lower_byte(*last_nir));
                self.encoder
                    .encode_symbol(&mut the_context.lower_byte_diff_model, u32::from(corr))?;
            }

            if is_nth_bit_set!(sym, 1) {
                let corr = upper_byte(current_point.nir()).wrapping_sub(upper_byte(*last_nir));
                self.encoder
                    .encode_symbol(&mut the_context.upper_byte_diff_model, u32::from(corr))?;
            }
            *last_nir = current_point.0;
            Ok(())
        }

        fn write_layers_sizes(&mut self, dst: &mut R) -> std::io::Result<()> {
            self.encoder.done()?;
            dst.write_u32::<LittleEndian>(self.encoder.out_stream().get_ref().len() as u32)
        }

        fn write_layers(&mut self, dst: &mut R) -> std::io::Result<()> {
            copy_encoder_content_to(&mut self.encoder, dst)
        }
    }
}
