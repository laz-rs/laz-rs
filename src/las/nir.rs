use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

pub trait LasNIR {
    fn nir(&self) -> u16;
    fn set_nir(&mut self, new_val: u16);

    fn read_from<R: Read>(&mut self, src: &mut R) -> std::io::Result<()> {
        self.set_nir(src.read_u16::<LittleEndian>()?);
        Ok(())
    }

    fn write_to<W: Write>(&self, dst: &mut W) -> std::io::Result<()> {
        dst.write_u16::<LittleEndian>(self.nir())?;
        Ok(())
    }
}

#[derive(Default, Copy, Clone, PartialOrd, PartialEq)]
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
    use crate::decoders::ArithmeticDecoder;
    use crate::las::nir::{LasNIR, Nir};
    use crate::las::utils::copy_bytes_into_decoder;
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::record::{BufferLayeredFieldDecompressor, LayeredPointFieldDecompressor};
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::{Cursor, Read, Seek};

    struct NirContext {
        last_nir: u16,
        nir_bytes_used_model: ArithmeticModel,
        nir_diff_0_model: ArithmeticModel,
        nir_diff_1_model: ArithmeticModel,
        unused: bool,
    }

    impl NirContext {
        fn from_last(last_val: u16) -> Self {
            Self {
                last_nir: last_val,
                nir_bytes_used_model: ArithmeticModelBuilder::new(4).build(),
                nir_diff_0_model: ArithmeticModelBuilder::new(256).build(),
                nir_diff_1_model: ArithmeticModelBuilder::new(256).build(),
                unused: false,
            }
        }

        fn new() -> Self {
            Self::from_last(0)
        }
    }

    //TODO Selective
    pub struct LasNIRDecompressor {
        pub(crate) decoder: ArithmeticDecoder<Cursor<Vec<u8>>>,
        pub(crate) changed_nir: bool,
        layer_size: u32,
        last_context_used: usize,
        contexts: Vec<NirContext>,
    }

    impl LasNIRDecompressor {
        pub fn new() -> Self {
            Self {
                decoder: ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new())),
                contexts: vec![
                    NirContext::new(),
                    NirContext::new(),
                    NirContext::new(),
                    NirContext::new(),
                ],
                changed_nir: false,
                layer_size: 0,
                last_context_used: 0,
            }
        }
    }

    impl<R: Read + Seek, P: LasNIR> LayeredPointFieldDecompressor<R, P> for LasNIRDecompressor {
        fn init_first_point(
            &mut self,
            src: &mut R,
            first_point: &mut P,
            context: &mut usize,
        ) -> std::io::Result<()> {
            for ctx in &mut self.contexts {
                ctx.unused = true;
            }

            let nir = src.read_u16::<LittleEndian>()?;
            self.contexts[*context].last_nir = nir;
            first_point.set_nir(nir);

            self.last_context_used = *context;
            Ok(())
        }

        fn decompress_field_with(
            &mut self,
            current_point: &mut P,
            context: &mut usize,
        ) -> std::io::Result<()> {
            if self.last_context_used != *context {
                if self.contexts[*context].unused {
                    let last_nir = self.contexts[self.last_context_used].last_nir;
                    self.contexts[*context] = NirContext::from_last(last_nir);
                }
            }

            let the_context = &mut self.contexts[*context];

            if self.changed_nir {
                let mut new_nir: u16;
                let sym = self
                    .decoder
                    .decode_symbol(&mut the_context.nir_bytes_used_model)?;

                if (sym & (1 << 0)) != 0 {
                    let corr = self
                        .decoder
                        .decode_symbol(&mut the_context.nir_diff_0_model)?
                        as u16;
                    new_nir = corr + (the_context.last_nir & 0x00FF);
                } else {
                    new_nir = the_context.last_nir & 0x00FF;
                }

                if (sym & (1 << 1)) != 0 {
                    let corr = self
                        .decoder
                        .decode_symbol(&mut the_context.nir_diff_1_model)?
                        as u16;
                    let upper_byte = corr + the_context.last_nir >> 8;
                    new_nir |= (upper_byte << 8) & 0xFF00;
                } else {
                    new_nir |= the_context.last_nir & 0xFF00;
                }
                the_context.last_nir = new_nir;
            }

            current_point.set_nir(the_context.last_nir);
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

    impl_buffer_decompressor_for_typed_decompressor!(LasNIRDecompressor, Nir);
}
