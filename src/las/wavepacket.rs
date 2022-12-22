use crate::packers::Packable;

/// ASPRS definition of wavepacket data.
#[derive(Debug, Default, Copy, Clone, PartialEq, PartialOrd)]
pub struct LasWavepacket {
    descriptor_index: u8,
    // offset in bytes to waveform data
    offset: u64,
    // in bytes
    size: u32,
    return_point: f32,
    dx: f32,
    dy: f32,
    dz: f32,
}

impl LasWavepacket {
    pub(crate) const SIZE: usize = 29;
}

impl Packable for LasWavepacket {
    fn unpack_from(input: &[u8]) -> Self {
        assert!(
            input.len() >= LasWavepacket::SIZE,
            "LasWavepacket::unpack_from expected buffer of {} bytes",
            LasWavepacket::SIZE
        );
        unsafe {
            Self::unpack_from_unchecked(input)
        }
    }

    fn pack_into(&self, output: &mut [u8]) {
        assert!(
            output.len() >= LasWavepacket::SIZE,
            "LasWavepacket::pack_into expected buffer of {} bytes",
            LasWavepacket::SIZE
        );
        unsafe {
            self.pack_into_unchecked(output)
        }
    }

    unsafe fn unpack_from_unchecked(input: &[u8]) -> Self {
        debug_assert!(
            input.len() >= LasWavepacket::SIZE,
            "LasWavepacket::unpack_from_unchecked expected buffer of {} bytes",
            LasWavepacket::SIZE
        );
        Self {
            descriptor_index: u8::unpack_from_unchecked(input.get_unchecked(0..1)),
            offset: u64::unpack_from_unchecked(input.get_unchecked(1..9)),
            size: u32::unpack_from_unchecked(input.get_unchecked(9..13)),
            return_point: f32::unpack_from_unchecked(input.get_unchecked(13..17)),
            dx: f32::unpack_from_unchecked(input.get_unchecked(17..21)),
            dy: f32::unpack_from_unchecked(input.get_unchecked(21..25)),
            dz: f32::unpack_from_unchecked(input.get_unchecked(25..29)),
        }
    }

    unsafe fn pack_into_unchecked(&self, output: &mut [u8]) {
        debug_assert!(
            output.len() >= LasWavepacket::SIZE,
            "LasWavepacket::pack_into_unchecked expected buffer of {} bytes",
            LasWavepacket::SIZE
        );
        u8::pack_into_unchecked(&self.descriptor_index, output.get_unchecked_mut(0..1));
        u64::pack_into_unchecked(&self.offset, output.get_unchecked_mut(1..9));
        u32::pack_into_unchecked(&self.size, output.get_unchecked_mut(9..13));
        f32::pack_into_unchecked(&self.return_point, output.get_unchecked_mut(13..17));
        f32::pack_into_unchecked(&self.dx, output.get_unchecked_mut(17..21));
        f32::pack_into_unchecked(&self.dy, output.get_unchecked_mut(21..25));
        f32::pack_into_unchecked(&self.dz, output.get_unchecked_mut(25..29));
    }
}

pub mod v1 {
    use super::LasWavepacket;
    use crate::compressors::{IntegerCompressor, IntegerCompressorBuilder};
    use crate::decoders::ArithmeticDecoder;
    use crate::decompressors::{IntegerDecompressor, IntegerDecompressorBuilder};
    use crate::encoders::ArithmeticEncoder;
    use crate::las::utils::read_and_unpack;
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;
    use crate::record::{FieldCompressor, FieldDecompressor};
    use std::io::{Read, Write};

    const DX_CONTEXT: u32 = 0;
    const DY_CONTEXT: u32 = 1;
    const DZ_CONTEXT: u32 = 2;


    pub struct LasWavepacketDecompressor {
        last_wavepacket: LasWavepacket,

        last_offset_diff: i32,
        last_sym_offset_diff: u32,

        packet_index_model: ArithmeticModel,
        offset_diff_model: [ArithmeticModel; 4],

        idc_offset_diff: IntegerDecompressor,
        idc_packet_size: IntegerDecompressor,
        idc_return_point: IntegerDecompressor,
        idc_xyz: IntegerDecompressor,
    }

    impl Default for LasWavepacketDecompressor {
        fn default() -> Self {
            Self {
                last_wavepacket: LasWavepacket::default(),
                last_offset_diff: 0,
                last_sym_offset_diff: 0,
                packet_index_model: ArithmeticModelBuilder::new(256).build(),
                offset_diff_model: [
                    ArithmeticModelBuilder::new(4).build(),
                    ArithmeticModelBuilder::new(4).build(),
                    ArithmeticModelBuilder::new(4).build(),
                    ArithmeticModelBuilder::new(4).build(),
                ],
                idc_offset_diff: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .build_initialized(),
                idc_packet_size: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .build_initialized(),
                idc_return_point: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .build_initialized(),
                // 3 contexts as this is used to decompress dx, dy, dz
                idc_xyz: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(3)
                    .build_initialized(),
            }
        }
    }

    impl<R> FieldDecompressor<R> for LasWavepacketDecompressor
    where
        R: Read,
    {
        fn size_of_field(&self) -> usize {
            LasWavepacket::SIZE
        }

        fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()> {
            self.last_wavepacket = read_and_unpack(src, first_point)?;
            Ok(())
        }

        fn decompress_with(
            &mut self,
            decoder: &mut ArithmeticDecoder<R>,
            buf: &mut [u8],
        ) -> std::io::Result<()> {
            let mut current_wavepacket = LasWavepacket::default();

            current_wavepacket.descriptor_index =
                decoder.decode_symbol(&mut self.packet_index_model)? as u8;

            self.last_sym_offset_diff = decoder
                .decode_symbol(&mut self.offset_diff_model[self.last_sym_offset_diff as usize])?;

            match self.last_sym_offset_diff {
                0 => {
                    current_wavepacket.offset = self.last_wavepacket.offset;
                }
                1 => {
                    current_wavepacket.offset =
                        self.last_wavepacket.offset + u64::from(self.last_wavepacket.size);
                }
                2 => {
                    self.last_offset_diff =
                        self.idc_offset_diff
                            .decompress(decoder, self.last_offset_diff, 0)?;
                    current_wavepacket.offset =
                        self.last_wavepacket.offset.wrapping_add(self.last_offset_diff as u64);
                }
                _ => {
                    current_wavepacket.offset = decoder.read_int_64()?;
                }
            }

            current_wavepacket.size =
                self.idc_packet_size
                    .decompress(decoder, self.last_wavepacket.size as i32, 0)?
                    as u32;

            // return_point
            let pred = i32::from_le_bytes(self.last_wavepacket.return_point.to_le_bytes());
            let tmp_out = self.idc_return_point.decompress(decoder, pred, 0)?;
            current_wavepacket.return_point = f32::from_le_bytes(tmp_out.to_le_bytes());

            // x
            let pred = i32::from_le_bytes(self.last_wavepacket.dx.to_le_bytes());
            let tmp_out = self.idc_xyz.decompress(decoder, pred, DX_CONTEXT)?;
            current_wavepacket.dx = f32::from_le_bytes(tmp_out.to_le_bytes());

            // y
            let pred = i32::from_le_bytes(self.last_wavepacket.dy.to_le_bytes());
            let tmp_out = self.idc_xyz.decompress(decoder, pred, DY_CONTEXT)?;
            current_wavepacket.dy = f32::from_le_bytes(tmp_out.to_le_bytes());

            // z
            let pred = i32::from_le_bytes(self.last_wavepacket.dz.to_le_bytes());
            let tmp_out = self.idc_xyz.decompress(decoder, pred, DZ_CONTEXT)?;
            current_wavepacket.dz = f32::from_le_bytes(tmp_out.to_le_bytes());

            current_wavepacket.pack_into(buf);
            self.last_wavepacket = current_wavepacket;

            Ok(())
        }
    }

    pub struct LasWavepacketCompressor {
        last_wavepacket: LasWavepacket,

        last_offset_diff: i32,
        last_sym_offset_diff: u32,

        packet_index_model: ArithmeticModel,
        offset_diff_model: [ArithmeticModel; 4],

        ic_offset_diff: IntegerCompressor,
        ic_packet_size: IntegerCompressor,
        ic_return_point: IntegerCompressor,
        ic_xyz: IntegerCompressor,
    }

    impl Default for LasWavepacketCompressor {
        fn default() -> Self {
            Self {
                last_wavepacket: LasWavepacket::default(),
                last_offset_diff: 0,
                last_sym_offset_diff: 0,
                packet_index_model: ArithmeticModelBuilder::new(256).build(),
                offset_diff_model: [
                    ArithmeticModelBuilder::new(4).build(),
                    ArithmeticModelBuilder::new(4).build(),
                    ArithmeticModelBuilder::new(4).build(),
                    ArithmeticModelBuilder::new(4).build(),
                ],
                ic_offset_diff: IntegerCompressorBuilder::new().bits(32).build_initialized(),
                ic_packet_size: IntegerCompressorBuilder::new().bits(32).build_initialized(),
                ic_return_point: IntegerCompressorBuilder::new().bits(32).build_initialized(),
                // 3 contexts as this is used to decompress dx, dy, dz
                ic_xyz: IntegerCompressorBuilder::new()
                    .bits(32)
                    .contexts(3)
                    .build_initialized(),
            }
        }
    }

    impl<W> FieldCompressor<W> for LasWavepacketCompressor
    where
        W: Write,
    {
        fn size_of_field(&self) -> usize {
            LasWavepacket::SIZE
        }

        fn compress_first(&mut self, dst: &mut W, buf: &[u8]) -> std::io::Result<()> {
            self.last_wavepacket = LasWavepacket::unpack_from(buf);

            dst.write_all(buf)
        }

        fn compress_with(
            &mut self,
            encoder: &mut ArithmeticEncoder<W>,
            buf: &[u8],
        ) -> std::io::Result<()> {
            let current_item = LasWavepacket::unpack_from(buf);
            encoder.encode_symbol(
                &mut self.packet_index_model,
                u32::from(current_item.descriptor_index),
            )?;

            let offset_diff_64 = current_item.offset as i64 - self.last_wavepacket.offset as i64;
            let offset_diff_32 = offset_diff_64 as i32;

            if offset_diff_64 == offset_diff_32 as i64 {
                // Difference can be represented on 32bits

                let new_syn_offset_diff = if offset_diff_32 == 0 {
                    0
                } else if offset_diff_32 == self.last_wavepacket.size as i32 {
                    1
                } else {
                    2
                };

                encoder.encode_symbol(
                    &mut self.offset_diff_model[self.last_sym_offset_diff as usize],
                    new_syn_offset_diff,
                )?;

                if new_syn_offset_diff == 2 {
                    self.ic_offset_diff.compress(
                        encoder,
                        self.last_offset_diff,
                        offset_diff_32,
                        0,
                    )?;
                    self.last_offset_diff = offset_diff_32;
                }

                self.last_sym_offset_diff = new_syn_offset_diff;
            } else {
                encoder.encode_symbol(
                    &mut self.offset_diff_model[self.last_sym_offset_diff as usize],
                    3,
                )?;
                self.last_sym_offset_diff = 3;
                encoder.write_int64(current_item.offset)?;
            }

            self.ic_packet_size.compress(
                encoder,
                self.last_wavepacket.size as i32,
                current_item.size as i32,
                0,
            )?;

            // return_point
            let pred = i32::from_le_bytes(self.last_wavepacket.return_point.to_le_bytes());
            let real = i32::from_le_bytes(current_item.return_point.to_le_bytes());
            self.ic_return_point.compress(encoder, pred, real, 0)?;

            // x
            let pred = i32::from_le_bytes(self.last_wavepacket.dx.to_le_bytes());
            let real = i32::from_le_bytes(current_item.dx.to_le_bytes());
            self.ic_xyz.compress(encoder, pred, real, DX_CONTEXT)?;

            // y
            let pred = i32::from_le_bytes(self.last_wavepacket.dy.to_le_bytes());
            let real = i32::from_le_bytes(current_item.dy.to_le_bytes());
            self.ic_xyz.compress(encoder, pred, real, DY_CONTEXT)?;

            // z
            let pred = i32::from_le_bytes(self.last_wavepacket.dz.to_le_bytes());
            let real = i32::from_le_bytes(current_item.dz.to_le_bytes());
            self.ic_xyz.compress(encoder, pred, real, DZ_CONTEXT)?;

            self.last_wavepacket = current_item;

            Ok(())
        }
    }
}

/// Just re-export v1 as v2 as they are both the same implementation
pub use v1 as v2;
