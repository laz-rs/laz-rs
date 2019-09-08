use crate::las::nir::LasNIR;
use crate::las::rgb::LasRGB;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

#[derive(Default, Copy, Clone)]
pub struct RGBNIR {
    red: u16,
    green: u16,
    blue: u16,
    nir: u16,
}

impl RGBNIR {
    pub const SIZE: usize = 8;

    pub fn read_from<R: Read>(&mut self, src: &mut R) -> std::io::Result<()> {
        self.red = src.read_u16::<LittleEndian>()?;
        self.green = src.read_u16::<LittleEndian>()?;
        self.blue = src.read_u16::<LittleEndian>()?;
        self.nir = src.read_u16::<LittleEndian>()?;
        Ok(())
    }

    pub fn write_to<W: Write>(&self, dst: &mut W) -> std::io::Result<()> {
        dst.write_u16::<LittleEndian>(self.red)?;
        dst.write_u16::<LittleEndian>(self.green)?;
        dst.write_u16::<LittleEndian>(self.blue)?;
        dst.write_u16::<LittleEndian>(self.nir)?;
        Ok(())
    }
}

impl LasNIR for RGBNIR {
    fn nir(&self) -> u16 {
        self.nir
    }

    fn set_nir(&mut self, new_val: u16) {
        self.nir = new_val;
    }
}

impl LasRGB for RGBNIR {
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
        self.green = new_val;
    }

    fn set_blue(&mut self, new_val: u16) {
        self.blue = new_val;
    }
}

pub mod v3 {
    use crate::las::nir::v3::LasNIRDecompressor;
    use crate::las::nir::LasNIR;
    use crate::las::rgb::v3::LasRGBDecompressor;
    use crate::las::rgb::LasRGB;

    use super::RGBNIR;
    use crate::las::utils::copy_bytes_into_decoder;
    use crate::record::{BufferLayeredFieldDecompressor, LayeredPointFieldDecompressor};
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::{Cursor, Read, Seek};

    pub struct LasRGBNIRDecompressor {
        rgb_layer_size: u32,
        nir_layer_size: u32,
        rgb_decompressor: LasRGBDecompressor,
        nir_decompressor: LasNIRDecompressor,
    }

    impl LasRGBNIRDecompressor {
        pub fn new() -> Self {
            Self {
                rgb_layer_size: 0,
                nir_layer_size: 0,
                rgb_decompressor: LasRGBDecompressor::new(),
                nir_decompressor: LasNIRDecompressor::new(),
            }
        }
    }

    impl<R: Read + Seek, P: LasRGB + LasNIR> LayeredPointFieldDecompressor<R, P>
        for LasRGBNIRDecompressor
    {
        fn init_first_point(
            &mut self,
            src: &mut R,
            first_point: &mut P,
            context: &mut usize,
        ) -> std::io::Result<()> {
            <LasRGBDecompressor as LayeredPointFieldDecompressor<R, P>>::init_first_point(
                &mut self.rgb_decompressor,
                src,
                first_point,
                context,
            )?;
            <LasNIRDecompressor as LayeredPointFieldDecompressor<R, P>>::init_first_point(
                &mut self.nir_decompressor,
                src,
                first_point,
                context,
            )
        }

        fn decompress_field_with(
            &mut self,
            current_point: &mut P,
            context: &mut usize,
        ) -> std::io::Result<()> {
            <LasRGBDecompressor as LayeredPointFieldDecompressor<R, P>>::decompress_field_with(
                &mut self.rgb_decompressor,
                current_point,
                context,
            )?;
            <LasNIRDecompressor as LayeredPointFieldDecompressor<R, P>>::decompress_field_with(
                &mut self.nir_decompressor,
                current_point,
                context,
            )
        }

        fn read_layers_sizes(&mut self, src: &mut R) -> std::io::Result<()> {
            self.rgb_layer_size = src.read_u32::<LittleEndian>()?;
            self.nir_layer_size = src.read_u32::<LittleEndian>()?;
            Ok(())
        }

        fn read_layers(&mut self, src: &mut R) -> std::io::Result<()> {
            self.rgb_decompressor.changed_rgb = copy_bytes_into_decoder(
                true, //TODO
                self.rgb_layer_size as usize,
                &mut self.rgb_decompressor.decoder,
                src,
            )?;

            self.nir_decompressor.changed_nir = copy_bytes_into_decoder(
                true, //TODO
                self.nir_layer_size as usize,
                &mut self.nir_decompressor.decoder,
                src,
            )?;

            Ok(())
        }
    }

    impl_buffer_decompressor_for_typed_decompressor!(LasRGBNIRDecompressor, RGBNIR);

}
