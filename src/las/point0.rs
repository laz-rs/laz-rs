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

//! Defines the Point Format 0 ands different version of compressors and decompressors

use crate::packers::Packable;

pub trait LasPoint0 {
    // Non mutable accessors
    fn x(&self) -> i32;
    fn y(&self) -> i32;
    fn z(&self) -> i32;
    fn intensity(&self) -> u16;

    fn bit_fields(&self) -> u8;
    fn number_of_returns_of_given_pulse(&self) -> u8;
    fn scan_direction_flag(&self) -> bool;
    fn edge_of_flight_line(&self) -> bool;
    fn return_number(&self) -> u8;

    fn classification(&self) -> u8;
    fn scan_angle_rank(&self) -> i8;
    fn user_data(&self) -> u8;
    fn point_source_id(&self) -> u16;

    // Mutable accessors

    fn set_x(&mut self, new_val: i32);
    fn set_y(&mut self, new_val: i32);
    fn set_z(&mut self, new_val: i32);
    fn set_intensity(&mut self, new_val: u16);

    fn set_bit_fields(&mut self, new_val: u8);

    fn set_classification(&mut self, new_val: u8);
    fn set_scan_angle_rank(&mut self, new_val: i8);
    fn set_user_data(&mut self, new_val: u8);
    fn set_point_source_id(&mut self, new_val: u16);
}

#[derive(Default, Copy, Clone, PartialEq, Debug)]
pub struct Point0 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub intensity: u16,

    // 3 bits
    pub number_of_returns_of_given_pulse: u8,
    // 3 bits
    pub scan_direction_flag: bool,
    // 1 bit
    pub edge_of_flight_line: bool,
    // 1 bit
    pub return_number: u8,

    // 5 bits for classification the rest are bit flags
    pub classification: u8,

    pub scan_angle_rank: i8,
    pub user_data: u8,
    pub point_source_id: u16,
}

impl Point0 {
    pub const SIZE: usize = 20;

    pub fn populate_bit_fields_from(&mut self, byte: u8) {
        self.return_number = byte & 0x7;
        self.number_of_returns_of_given_pulse = (byte >> 3) & 0x7;
        self.scan_direction_flag = ((byte >> 6) & 0x1) != 0;
        self.edge_of_flight_line = ((byte >> 7) & 0x1) != 0;
    }

    pub fn bit_fields_to_byte(&self) -> u8 {
        let a = self.return_number;
        let b = self.number_of_returns_of_given_pulse;
        let c = self.scan_direction_flag as u8;
        let d = self.edge_of_flight_line as u8;

        ((d & 0x1) << 7) | (c & 0x1) << 6 | (b & 0x7) << 3 | (a & 0x7)
    }

    pub fn copy_from<P: LasPoint0 + ?Sized>(&mut self, point: &P) {
        todo!()
    }
}

impl LasPoint0 for Point0 {
    fn x(&self) -> i32 {
        self.x
    }

    fn y(&self) -> i32 {
        self.y
    }

    fn z(&self) -> i32 {
        self.z
    }

    fn intensity(&self) -> u16 {
        self.intensity
    }

    fn bit_fields(&self) -> u8 {
        self.bit_fields_to_byte()
    }

    fn number_of_returns_of_given_pulse(&self) -> u8 {
        self.number_of_returns_of_given_pulse
    }

    fn scan_direction_flag(&self) -> bool {
        self.scan_direction_flag
    }

    fn edge_of_flight_line(&self) -> bool {
        self.edge_of_flight_line
    }

    fn return_number(&self) -> u8 {
        self.return_number
    }

    fn classification(&self) -> u8 {
        self.classification
    }

    fn scan_angle_rank(&self) -> i8 {
        self.scan_angle_rank
    }

    fn user_data(&self) -> u8 {
        self.user_data
    }

    fn point_source_id(&self) -> u16 {
        self.point_source_id
    }

    fn set_x(&mut self, new_val: i32) {
        self.x = new_val;
    }

    fn set_y(&mut self, new_val: i32) {
        self.y = new_val;
    }

    fn set_z(&mut self, new_val: i32) {
        self.z = new_val;
    }

    fn set_intensity(&mut self, new_val: u16) {
        self.intensity = new_val
    }

    fn set_bit_fields(&mut self, new_val: u8) {
        self.populate_bit_fields_from(new_val);
    }

    fn set_classification(&mut self, new_val: u8) {
        self.classification = new_val;
    }

    fn set_scan_angle_rank(&mut self, new_val: i8) {
        self.scan_angle_rank = new_val;
    }

    fn set_user_data(&mut self, new_val: u8) {
        self.user_data = new_val;
    }

    fn set_point_source_id(&mut self, new_val: u16) {
        self.point_source_id = new_val;
    }
}

impl Packable for Point0 {
    fn unpack_from(input: &[u8]) -> Self {
        assert!(
            input.len() >= 20,
            "Point10::unpack_from expected buffer of 20 bytes"
        );
        unsafe { Self::unpack_from_unchecked(input) }
    }

    fn pack_into(&self, output: &mut [u8]) {
        assert!(
            output.len() >= 20,
            "Point10::pack_into expected buffer of 20 bytes"
        );
        unsafe { self.pack_into_unchecked(output) }
    }

    unsafe fn unpack_from_unchecked(input: &[u8]) -> Self {
        debug_assert!(
            input.len() >= 20,
            "Point10::unpack_from expected buffer of 20 bytes"
        );
        let mut point = Self {
            x: i32::unpack_from_unchecked(input.get_unchecked(..4)),
            y: i32::unpack_from_unchecked(input.get_unchecked(4..8)),
            z: i32::unpack_from_unchecked(input.get_unchecked(8..12)),
            intensity: u16::unpack_from_unchecked(input.get_unchecked(12..14)),
            number_of_returns_of_given_pulse: 0,
            scan_direction_flag: false,
            edge_of_flight_line: false,
            return_number: 0,
            classification: *input.get_unchecked(15),
            scan_angle_rank: *input.get_unchecked(16) as i8,
            user_data: *input.get_unchecked(17),
            point_source_id: u16::unpack_from_unchecked(input.get_unchecked(18..20)),
        };
        point.set_bit_fields(*input.get_unchecked(14));
        point
    }

    unsafe fn pack_into_unchecked(&self, output: &mut [u8]) {
        debug_assert!(
            output.len() >= 20,
            "Point10::pack_into expected buffer of 20 bytes"
        );
        i32::pack_into_unchecked(&self.x, output.get_unchecked_mut(0..4));
        i32::pack_into_unchecked(&self.y, output.get_unchecked_mut(4..8));
        i32::pack_into_unchecked(&self.z, output.get_unchecked_mut(8..12));

        u16::pack_into_unchecked(&self.intensity, output.get_unchecked_mut(12..14));

        *output.get_unchecked_mut(14) = self.bit_fields_to_byte();
        *output.get_unchecked_mut(15) = self.classification;
        *output.get_unchecked_mut(16) = self.scan_angle_rank as u8;
        *output.get_unchecked_mut(17) = self.user_data;
        u16::pack_into_unchecked(&self.point_source_id, output.get_unchecked_mut(18..20));
    }
}

pub mod v1 {
    use std::io::{Read, Write};

    use crate::compressors::{
        IntegerCompressor, IntegerCompressorBuilder, DEFAULT_COMPRESS_CONTEXTS,
    };
    use crate::decoders::ArithmeticDecoder;
    use crate::decompressors::{
        IntegerDecompressor, IntegerDecompressorBuilder, DEFAULT_DECOMPRESS_CONTEXTS,
    };
    use crate::encoders::ArithmeticEncoder;
    use crate::las::point0::LasPoint0;
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;
    use crate::record::{FieldCompressor, FieldDecompressor};

    use super::Point0;

    /// find median difference from 3 preceding differences
    fn median_diff(diff_array: &[i32; 3]) -> i32 {
        unsafe {
            if diff_array.get_unchecked(0) < diff_array.get_unchecked(1) {
                if diff_array.get_unchecked(1) < diff_array.get_unchecked(2) {
                    *diff_array.get_unchecked(1)
                } else if diff_array.get_unchecked(0) < diff_array.get_unchecked(2) {
                    *diff_array.get_unchecked(2)
                } else {
                    *diff_array.get_unchecked(0)
                }
            } else if diff_array.get_unchecked(0) < diff_array.get_unchecked(2) {
                *diff_array.get_unchecked(0)
            } else if diff_array.get_unchecked(1) < diff_array.get_unchecked(2) {
                *diff_array.get_unchecked(2)
            } else {
                *diff_array.get_unchecked(1)
            }
        }
    }

    pub struct LasPoint0Decompressor {
        last_point: Point0,
        last_x_diffs: [i32; 3],
        last_y_diffs: [i32; 3],
        last_incr: usize,

        ic_dx: IntegerDecompressor,
        ic_dy: IntegerDecompressor,
        ic_dz: IntegerDecompressor,
        ic_intensity: IntegerDecompressor,
        ic_scan_angle_rank: IntegerDecompressor,
        ic_point_source_id: IntegerDecompressor,

        changed_values_model: ArithmeticModel,
        // All theses vec have 256 elements
        // all the associated dimensions have 256 elements: [0..255]
        bit_byte_models: Vec<Option<ArithmeticModel>>,
        classification_models: Vec<Option<ArithmeticModel>>,
        user_data_models: Vec<Option<ArithmeticModel>>,
    }

    impl Default for LasPoint0Decompressor {
        fn default() -> Self {
            Self {
                last_point: Default::default(),
                last_x_diffs: [0i32; 3],
                last_y_diffs: [0i32; 3],
                last_incr: 0,
                ic_dx: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .build_initialized(),
                ic_dy: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(20)
                    .build_initialized(),
                ic_dz: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(20)
                    .build_initialized(),
                ic_intensity: IntegerDecompressorBuilder::new()
                    .bits(16)
                    .build_initialized(),
                ic_scan_angle_rank: IntegerDecompressorBuilder::new()
                    .bits(8)
                    .contexts(2)
                    .build_initialized(),
                ic_point_source_id: IntegerDecompressorBuilder::new()
                    .bits(16)
                    .build_initialized(),
                changed_values_model: ArithmeticModelBuilder::new(64).build(),
                bit_byte_models: (0..256).map(|_| None).collect(),
                classification_models: (0..256).map(|_| None).collect(),
                user_data_models: (0..256).map(|_| None).collect(),
            }
        }
    }

    impl LasPoint0Decompressor {
        fn median_x_diff(&self) -> i32 {
            median_diff(&self.last_x_diffs)
        }

        fn median_y_diff(&self) -> i32 {
            median_diff(&self.last_y_diffs)
        }
    }

    pub struct LasPoint0Compressor {
        last_point: Point0,
        last_x_diffs: [i32; 3],
        last_y_diffs: [i32; 3],
        last_incr: usize,

        ic_dx: IntegerCompressor,
        ic_dy: IntegerCompressor,
        ic_dz: IntegerCompressor,
        ic_intensity: IntegerCompressor,
        ic_scan_angle_rank: IntegerCompressor,
        ic_point_source_id: IntegerCompressor,

        changed_values_model: ArithmeticModel,
        // All theses vec have 256 elements
        // all the associated dimensions have 256 elements: [0..255]
        bit_byte_models: Vec<Option<ArithmeticModel>>,
        classification_models: Vec<Option<ArithmeticModel>>,
        user_data_models: Vec<Option<ArithmeticModel>>,
    }

    impl Default for LasPoint0Compressor {
        fn default() -> Self {
            Self {
                last_point: Default::default(),
                last_x_diffs: [0i32; 3],
                last_y_diffs: [0i32; 3],
                last_incr: 0,
                ic_dx: IntegerCompressorBuilder::new().bits(32).build_initialized(),
                ic_dy: IntegerCompressorBuilder::new()
                    .bits(32)
                    .contexts(20)
                    .build_initialized(),
                ic_dz: IntegerCompressorBuilder::new()
                    .bits(32)
                    .contexts(20)
                    .build_initialized(),
                ic_intensity: IntegerCompressorBuilder::new().bits(16).build_initialized(),
                ic_scan_angle_rank: IntegerCompressorBuilder::new()
                    .bits(8)
                    .contexts(2)
                    .build_initialized(),
                ic_point_source_id: IntegerCompressorBuilder::new().bits(16).build_initialized(),
                changed_values_model: ArithmeticModelBuilder::new(64).build(),
                bit_byte_models: (0..256).map(|_| None).collect(),
                classification_models: (0..256).map(|_| None).collect(),
                user_data_models: (0..256).map(|_| None).collect(),
            }
        }
    }

    impl<P: ?Sized + LasPoint0, W: Write> FieldCompressor<P, W> for LasPoint0Compressor {
        fn size_of_field(&self) -> usize {
            20
        }

        fn compress_first(&mut self, dst: &mut W, point: &P) -> std::io::Result<()> {
            self.last_point.copy_from(point);
            let mut buf = [0u8; Point0::SIZE];
            self.last_point.pack_into(&mut buf);
            dst.write_all(&buf)?;
            Ok(())
        }

        fn compress_with(
            &mut self,
            encoder: &mut ArithmeticEncoder<W>,
            point: &P,
        ) -> std::io::Result<()> {
            let mut current_point = Point0::default();
            current_point.copy_from(point);
            let median_x = median_diff(&self.last_x_diffs);
            let median_y = median_diff(&self.last_y_diffs);

            let x_diff = current_point.x() - self.last_point.x();
            let y_diff = current_point.y() - self.last_point.y();

            self.ic_dx
                .compress(encoder, median_x, x_diff, DEFAULT_COMPRESS_CONTEXTS)?;
            let k_bits = self.ic_dx.k();
            self.ic_dy.compress(
                encoder,
                median_y,
                y_diff,
                if k_bits < 19 { k_bits } else { 19 },
            )?;

            let k_bits = (k_bits + self.ic_dy.k()) / 2;
            self.ic_dz.compress(
                encoder,
                self.last_point.z,
                current_point.z,
                if k_bits < 19 { k_bits } else { 19 },
            )?;

            let changed_values: u8 = ((self.last_point.intensity() != current_point.intensity())
                as u8)
                << 5
                | ((self.last_point.bit_fields() != current_point.bit_fields()) as u8) << 4
                | ((self.last_point.classification() != current_point.classification()) as u8) << 3
                | ((self.last_point.scan_angle_rank() != current_point.scan_angle_rank()) as u8)
                    << 2
                | ((self.last_point.user_data() != current_point.user_data()) as u8) << 1
                | (self.last_point.point_source_id() != current_point.point_source_id()) as u8;

            encoder.encode_symbol(&mut self.changed_values_model, u32::from(changed_values))?;

            if changed_values != 0 {
                if (changed_values & 32) != 0 {
                    self.ic_intensity.compress(
                        encoder,
                        i32::from(self.last_point.intensity),
                        i32::from(current_point.intensity),
                        DEFAULT_COMPRESS_CONTEXTS,
                    )?;
                }

                if (changed_values & 16) != 0 {
                    let model = &mut self.bit_byte_models[self.last_point.bit_fields() as usize]
                        .get_or_insert(ArithmeticModelBuilder::new(256).build());
                    encoder.encode_symbol(model, u32::from(current_point.bit_fields()))?;
                }

                if (changed_values & 8) != 0 {
                    let model = &mut self.classification_models
                        [self.last_point.classification() as usize]
                        .get_or_insert(ArithmeticModelBuilder::new(256).build());
                    encoder.encode_symbol(model, u32::from(current_point.classification))?;
                }

                if (changed_values & 4) != 0 {
                    self.ic_scan_angle_rank.compress(
                        encoder,
                        i32::from(self.last_point.scan_angle_rank),
                        i32::from(current_point.scan_angle_rank),
                        (k_bits < 3) as u32,
                    )?;
                }

                if (changed_values & 2) != 0 {
                    let model = self.user_data_models[self.last_point.user_data() as usize]
                        .get_or_insert(ArithmeticModelBuilder::new(256).build());
                    encoder.encode_symbol(model, u32::from(current_point.user_data))?;
                }

                if (changed_values & 1) != 0 {
                    self.ic_point_source_id.compress(
                        encoder,
                        i32::from(self.last_point.point_source_id),
                        i32::from(current_point.point_source_id),
                        DEFAULT_COMPRESS_CONTEXTS,
                    )?;
                }
            }
            self.last_x_diffs[self.last_incr] = x_diff;
            self.last_y_diffs[self.last_incr] = y_diff;
            self.last_incr += 1;
            if self.last_incr > 2 {
                self.last_incr = 0;
            }
            self.last_point = current_point;
            Ok(())
        }
    }

    impl<R: Read> FieldDecompressor<R> for LasPoint0Decompressor {
        fn size_of_field(&self) -> usize {
            20
        }

        fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()> {
            src.read_exact(first_point)?;
            self.last_point = Point0::unpack_from(first_point);
            Ok(())
        }

        fn decompress_with(
            &mut self,
            mut decoder: &mut ArithmeticDecoder<R>,
            buf: &mut [u8],
        ) -> std::io::Result<()> {
            // Decompress x, y, z
            let median_x = self.median_x_diff();
            let median_y = self.median_y_diff();

            let x_diff =
                self.ic_dx
                    .decompress(&mut decoder, median_x, DEFAULT_DECOMPRESS_CONTEXTS)?;
            self.last_point.x += x_diff;
            // we use the number k of bits corrector bits to switch contexts
            let k_bits = self.ic_dx.k();
            let y_diff = self.ic_dy.decompress(
                &mut decoder,
                median_y,
                if k_bits < 19 { k_bits } else { 19 },
            )?;
            self.last_point.y += y_diff;
            let k_bits = (k_bits + self.ic_dy.k()) / 2;
            self.last_point.z = self.ic_dz.decompress(
                &mut decoder,
                self.last_point.z,
                if k_bits < 19 { k_bits } else { 19 },
            )?;

            let changed_value = decoder.decode_symbol(&mut self.changed_values_model)? as i32;
            if changed_value != 0 {
                if (changed_value & 32) != 0 {
                    self.last_point.intensity = self.ic_intensity.decompress(
                        &mut decoder,
                        self.last_point.intensity as i32,
                        DEFAULT_DECOMPRESS_CONTEXTS,
                    )? as u16;
                }

                if (changed_value & 16) != 0 {
                    let model = self.bit_byte_models[self.last_point.bit_fields() as usize]
                        .get_or_insert_with(|| ArithmeticModelBuilder::new(256).build());
                    self.last_point
                        .set_bit_fields(decoder.decode_symbol(model)? as u8);
                }

                if (changed_value & 8) != 0 {
                    let model = self.classification_models[self.last_point.classification as usize]
                        .get_or_insert_with(|| ArithmeticModelBuilder::new(256).build());
                    self.last_point
                        .set_classification(decoder.decode_symbol(model)? as u8);
                }

                if (changed_value & 4) != 0 {
                    self.last_point
                        .set_scan_angle_rank(self.ic_scan_angle_rank.decompress(
                            &mut decoder,
                            i32::from(self.last_point.scan_angle_rank),
                            (k_bits < 3) as u32,
                        )? as i8);
                }

                if (changed_value & 2) != 0 {
                    let model = self.user_data_models[self.last_point.user_data as usize]
                        .get_or_insert_with(|| ArithmeticModelBuilder::new(256).build());
                    self.last_point
                        .set_user_data(decoder.decode_symbol(model)? as u8);
                }

                if (changed_value & 1) != 0 {
                    self.last_point
                        .set_point_source_id(self.ic_point_source_id.decompress(
                            &mut decoder,
                            i32::from(self.last_point.point_source_id),
                            DEFAULT_DECOMPRESS_CONTEXTS,
                        )? as u16);
                }
            }

            // record the differences
            self.last_x_diffs[self.last_incr] = x_diff;
            self.last_y_diffs[self.last_incr] = y_diff;
            self.last_incr += 1;
            if self.last_incr > 2 {
                self.last_incr = 0;
            }
            self.last_point.pack_into(buf);
            Ok(())
        }
    }

    #[cfg(test)]
    mod test {
        use super::median_diff;

        #[test]
        fn median_diff_test_1_elem() {
            let a = [1, 0, 0];
            assert_eq!(median_diff(&a), 0);

            let a = [-1, 0, 0];
            assert_eq!(median_diff(&a), 0);
        }

        #[test]
        fn median_diff_test_2_elem() {
            let a = [3, 1, 0];
            assert_eq!(median_diff(&a), 1);

            let a = [-3, 1, 0];
            assert_eq!(median_diff(&a), 0);
        }

        #[test]
        fn median_diff_test_3_elem() {
            let a = [3, 1, 4];
            assert_eq!(median_diff(&a), 3);

            let a = [-3, 1, -5];
            assert_eq!(median_diff(&a), -3);
        }
    }
}

pub mod v2 {
    use std::io::{Read, Write};

    use crate::compressors::{IntegerCompressor, IntegerCompressorBuilder};
    use crate::decoders::ArithmeticDecoder;
    use crate::decompressors::{IntegerDecompressor, IntegerDecompressorBuilder};
    use crate::encoders::ArithmeticEncoder;
    use crate::las::point0::LasPoint0;
    use crate::las::utils;
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;
    use crate::record::{FieldCompressor, FieldDecompressor};

    use super::Point0;

    struct Point10ChangedValues {
        value: i32,
    }

    /// Only valid for version 2 of the compression / decompression
    /// Compared to version 1, the flag bit used for the intensity & bit_fields
    /// have been swapped
    impl Point10ChangedValues {
        pub fn from_points<P: LasPoint0, OP: LasPoint0>(
            current: &P,
            last: &OP,
            last_intensity: u16,
        ) -> Self {
            // This logic here constructs a 5-bit changed value which is basically a bit map of what has changed
            // since the last point, not considering the x, y and z values

            let bit_fields_changed = ((last.return_number() ^ current.return_number()) != 0)
                | ((last.number_of_returns_of_given_pulse()
                    ^ current.number_of_returns_of_given_pulse())
                    != 0)
                | (last.scan_direction_flag() ^ current.scan_direction_flag())
                | (last.edge_of_flight_line() ^ current.edge_of_flight_line());

            let intensity_changed = (last_intensity ^ current.intensity()) != 0;
            let classification_changed = (last.classification() ^ current.classification()) != 0;
            let scan_angle_rank_changed = (last.scan_angle_rank() ^ current.scan_angle_rank()) != 0;
            let user_data_changed = (last.user_data() ^ current.user_data()) != 0;
            let point_source_id_changed = (last.point_source_id() ^ current.point_source_id()) != 0;
            Point10ChangedValues {
                value: (bit_fields_changed as i32) << 5
                    | (intensity_changed as i32) << 4
                    | (classification_changed as i32) << 3
                    | (scan_angle_rank_changed as i32) << 2
                    | (user_data_changed as i32) << 1
                    | (point_source_id_changed as i32),
            }
        }

        pub fn bit_fields_changed(&self) -> bool {
            (self.value & (1 << 5)) != 0
        }

        pub fn intensity_changed(&self) -> bool {
            (self.value & (1 << 4)) != 0
        }

        pub fn classification_changed(&self) -> bool {
            (self.value & (1 << 3)) != 0
        }

        pub fn scan_angle_rank_changed(&self) -> bool {
            (self.value & (1 << 2)) != 0
        }

        pub fn user_data_changed(&self) -> bool {
            (self.value & (1 << 1)) != 0
        }

        pub fn point_source_id_changed(&self) -> bool {
            (self.value & 1) != 0
        }
    }

    // TODO Do just like Point6Compressor & Point6Decompressor: have a Point0Models, Point0 compressors
    // All the things we need to compress a point, group them into structs
    // so we don't have too many names flying around
    struct Common {
        last_intensity: [u16; 16],

        // can't have arrays as StreamingMedian is not a copy type
        // 16 elements both
        last_x_diff_median: Vec<utils::StreamingMedian<i32>>,
        last_y_diff_median: Vec<utils::StreamingMedian<i32>>,

        last_height: [i32; 8],

        changed_values: ArithmeticModel,

        // can't have arrays as ArithmeticModel is not a copy type
        scan_angle_rank: Vec<ArithmeticModel>,
        // 2
        bit_byte: Vec<ArithmeticModel>,
        // 256
        classification: Vec<ArithmeticModel>,
        //256
        user_data: Vec<ArithmeticModel>, //256
    }

    impl Common {
        pub fn new() -> Self {
            Self {
                last_intensity: [0u16; 16],
                last_x_diff_median: (0..16)
                    .map(|_i| utils::StreamingMedian::<i32>::new())
                    .collect(),
                last_y_diff_median: (0..16)
                    .map(|_i| utils::StreamingMedian::<i32>::new())
                    .collect(),
                last_height: [0i32; 8],
                changed_values: ArithmeticModelBuilder::new(64).build(),
                scan_angle_rank: (0..2)
                    .map(|_i| ArithmeticModelBuilder::new(256).build())
                    .collect(),
                bit_byte: (0..256)
                    .map(|_i| ArithmeticModelBuilder::new(256).build())
                    .collect(),
                classification: (0..256)
                    .map(|_i| ArithmeticModelBuilder::new(256).build())
                    .collect(),
                user_data: (0..256)
                    .map(|_i| ArithmeticModelBuilder::new(256).build())
                    .collect(),
            }
        }
    }

    pub struct LasPoint0Compressor {
        last_point: Point0,
        ic_intensity: IntegerCompressor,
        ic_point_source_id: IntegerCompressor,
        ic_dx: IntegerCompressor,
        ic_dy: IntegerCompressor,
        ic_z: IntegerCompressor,
        common: Common,
    }

    impl Default for LasPoint0Compressor {
        fn default() -> Self {
            Self {
                last_point: Default::default(),
                ic_intensity: IntegerCompressorBuilder::new()
                    .bits(16)
                    .contexts(4)
                    .build_initialized(),
                ic_point_source_id: IntegerCompressorBuilder::new().bits(16).build_initialized(),
                ic_dx: IntegerCompressorBuilder::new()
                    .bits(32)
                    .contexts(2)
                    .build_initialized(),
                ic_dy: IntegerCompressorBuilder::new()
                    .bits(32)
                    .contexts(22)
                    .build_initialized(),
                ic_z: IntegerCompressorBuilder::new()
                    .bits(32)
                    .contexts(20)
                    .build_initialized(),
                common: Common::new(),
            }
        }
    }

    impl<P: ?Sized + LasPoint0, W: Write> FieldCompressor<P, W> for LasPoint0Compressor {
        fn size_of_field(&self) -> usize {
            20
        }

        fn compress_first(&mut self, dst: &mut W, point: &P) -> std::io::Result<()> {
            self.last_point.copy_from(point);
            let mut buf = [0u8; Point0::SIZE];
            self.last_point.pack_into(&mut buf);
            dst.write_all(&buf)
        }

        fn compress_with(
            &mut self,
            mut encoder: &mut ArithmeticEncoder<W>,
            point: &P,
        ) -> std::io::Result<()> {
            let mut current_point = Point0::default();
            current_point.copy_from(point);
            let r = current_point.return_number();
            let n = current_point.number_of_returns_of_given_pulse();
            // According to table  m is in range 0..16
            let m = utils::NUMBER_RETURN_MAP[n as usize][r as usize];
            // According to table l is in range 0..8
            let l = utils::NUMBER_RETURN_LEVEL[n as usize][r as usize];

            let changed_values =
                Point10ChangedValues::from_points(&current_point, &self.last_point, *unsafe {
                    self.common.last_intensity.get_unchecked(m as usize)
                });

            // compress which other values have changed

            encoder.encode_symbol(&mut self.common.changed_values, changed_values.value as u32)?;

            if changed_values.bit_fields_changed() {
                let b = current_point.bit_fields();
                let last_b = self.last_point.bit_fields();
                encoder.encode_symbol(
                    unsafe { self.common.bit_byte.get_unchecked_mut(last_b as usize) },
                    u32::from(b),
                )?;
            }

            if changed_values.intensity_changed() {
                self.ic_intensity.compress(
                    &mut encoder,
                    i32::from(self.common.last_intensity[m as usize]),
                    i32::from(current_point.intensity),
                    if m < 3 { u32::from(m) } else { 3 },
                )?;
                self.common.last_intensity[m as usize] = current_point.intensity;
            }

            if changed_values.classification_changed() {
                encoder.encode_symbol(
                    unsafe {
                        self.common
                            .classification
                            .get_unchecked_mut(self.last_point.classification as usize)
                    },
                    u32::from(current_point.classification),
                )?;
            }

            if changed_values.scan_angle_rank_changed() {
                // the "as u8" before "as u32" is vital
                encoder.encode_symbol(
                    unsafe {
                        self.common
                            .scan_angle_rank
                            .get_unchecked_mut(current_point.scan_direction_flag() as usize)
                    },
                    (current_point
                        .scan_angle_rank
                        .wrapping_sub(self.last_point.scan_angle_rank)) as u8
                        as u32,
                )?;
            }

            if changed_values.user_data_changed() {
                encoder.encode_symbol(
                    unsafe {
                        self.common
                            .user_data
                            .get_unchecked_mut(self.last_point.user_data as usize)
                    },
                    u32::from(current_point.user_data),
                )?;
            }

            if changed_values.point_source_id_changed() {
                self.ic_point_source_id.compress(
                    &mut encoder,
                    i32::from(self.last_point.point_source_id),
                    i32::from(current_point.point_source_id),
                    0,
                )?;
            }

            //compress x coordinates
            let median = unsafe { self.common.last_x_diff_median.get_unchecked(m as usize) }.get();
            let diff = current_point.x.wrapping_sub(self.last_point.x);
            self.ic_dx
                .compress(&mut encoder, median, diff, (n == 1) as u32)?;
            unsafe { self.common.last_x_diff_median.get_unchecked_mut(m as usize) }.add(diff);

            //compress y coordinates
            let k_bits = self.ic_dx.k();
            let median = unsafe {
                self.common
                    .last_y_diff_median
                    .get_unchecked(m as usize)
                    .get()
            };
            let diff = current_point.y.wrapping_sub(self.last_point.y);
            let context = (n == 1) as u32
                + if k_bits < 20 {
                    utils::u32_zero_bit(k_bits)
                } else {
                    20
                };
            self.ic_dy.compress(&mut encoder, median, diff, context)?;
            unsafe {
                self.common
                    .last_y_diff_median
                    .get_unchecked_mut(m as usize)
                    .add(diff);
            }

            //compress z coordinates
            let k_bits = (self.ic_dx.k() + self.ic_dy.k()) / 2;
            let context = (n == 1) as u32
                + if k_bits < 18 {
                    utils::u32_zero_bit(k_bits)
                } else {
                    18
                };
            self.ic_z.compress(
                &mut encoder,
                *unsafe { self.common.last_height.get_unchecked(l as usize) },
                current_point.z(),
                context,
            )?;
            unsafe { *self.common.last_height.get_unchecked_mut(l as usize) = current_point.z() };
            self.last_point = current_point;
            Ok(())
        }
    }

    pub struct LasPoint0Decompressor {
        last_point: Point0,
        ic_intensity: IntegerDecompressor,
        ic_point_source_id: IntegerDecompressor,
        ic_dx: IntegerDecompressor,
        ic_dy: IntegerDecompressor,
        ic_z: IntegerDecompressor,

        common: Common,
    }

    impl Default for LasPoint0Decompressor {
        fn default() -> Self {
            Self {
                last_point: Default::default(),
                ic_intensity: IntegerDecompressorBuilder::new()
                    .bits(16)
                    .contexts(4)
                    .build_initialized(),
                ic_point_source_id: IntegerDecompressorBuilder::new()
                    .bits(16)
                    .build_initialized(),
                ic_dx: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(2)
                    .build_initialized(),
                ic_dy: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(22)
                    .build_initialized(),
                ic_z: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(20)
                    .build_initialized(),
                common: Common::new(),
            }
        }
    }

    impl<R: Read> FieldDecompressor<R> for LasPoint0Decompressor {
        fn size_of_field(&self) -> usize {
            20
        }

        fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()> {
            src.read_exact(first_point)?;
            self.last_point = Point0::unpack_from(first_point);
            self.last_point.intensity = 0;
            Ok(())
        }

        fn decompress_with(
            &mut self,
            mut decoder: &mut ArithmeticDecoder<R>,
            buf: &mut [u8],
        ) -> std::io::Result<()> {
            let changed_value = Point10ChangedValues {
                value: decoder.decode_symbol(&mut self.common.changed_values)? as i32,
            };

            let r;
            let n;
            let m;
            let l;

            unsafe {
                if changed_value.value != 0 {
                    // there was some change in one of the fields (other than x, y and z)

                    if changed_value.bit_fields_changed() {
                        let mut b = self.last_point.bit_fields();
                        b = decoder
                            .decode_symbol(self.common.bit_byte.get_unchecked_mut(b as usize))?
                            as u8;
                        self.last_point.set_bit_fields(b);
                    }

                    r = self.last_point.return_number();
                    n = self.last_point.number_of_returns_of_given_pulse();
                    // According to table m is in range 0..16
                    m = utils::NUMBER_RETURN_MAP[n as usize][r as usize];
                    // According to table l is in range 0..8
                    l = utils::NUMBER_RETURN_LEVEL[n as usize][r as usize];

                    if changed_value.intensity_changed() {
                        self.last_point.intensity = self.ic_intensity.decompress(
                            &mut decoder,
                            i32::from(*self.common.last_intensity.get_unchecked(m as usize)),
                            if m < 3 { u32::from(m) } else { 3 },
                        )? as u16;
                        *self.common.last_intensity.get_unchecked_mut(m as usize) =
                            self.last_point.intensity;
                    } else {
                        self.last_point.intensity =
                            *self.common.last_intensity.get_unchecked(m as usize);
                    }

                    if changed_value.classification_changed() {
                        self.last_point.set_classification(
                            decoder.decode_symbol(
                                self.common
                                    .classification
                                    .get_unchecked_mut(self.last_point.classification as usize),
                            )? as u8,
                        );
                    }

                    if changed_value.scan_angle_rank_changed() {
                        let val = decoder.decode_symbol(
                            self.common
                                .scan_angle_rank
                                .get_unchecked_mut(self.last_point.scan_direction_flag as usize),
                        )? as i8;
                        self.last_point.scan_angle_rank =
                            self.last_point.scan_angle_rank.wrapping_add(val);
                    }

                    if changed_value.user_data_changed() {
                        self.last_point.set_user_data(
                            decoder.decode_symbol(
                                self.common
                                    .user_data
                                    .get_unchecked_mut(self.last_point.user_data as usize),
                            )? as u8,
                        );
                    }

                    if changed_value.point_source_id_changed() {
                        self.last_point
                            .set_point_source_id(self.ic_point_source_id.decompress(
                                &mut decoder,
                                i32::from(self.last_point.point_source_id),
                                0,
                            )? as u16);
                    }
                } else {
                    r = self.last_point.return_number();
                    n = self.last_point.number_of_returns_of_given_pulse();
                    m = utils::NUMBER_RETURN_MAP[n as usize][r as usize];
                    l = utils::NUMBER_RETURN_LEVEL[n as usize][r as usize];
                }

                // decompress x
                let median = self
                    .common
                    .last_x_diff_median
                    .get_unchecked(m as usize)
                    .get();
                let diff = self
                    .ic_dx
                    .decompress(&mut decoder, median, (n == 1) as u32)?;
                self.last_point.x = self.last_point.x.wrapping_add(diff);
                self.common
                    .last_x_diff_median
                    .get_unchecked_mut(m as usize)
                    .add(diff);

                // decompress y
                let median = self
                    .common
                    .last_y_diff_median
                    .get_unchecked(m as usize)
                    .get();
                let k_bits = self.ic_dx.k();
                let context = (n == 1) as u32
                    + if k_bits < 20 {
                        utils::u32_zero_bit(k_bits)
                    } else {
                        20
                    };
                let diff = self.ic_dy.decompress(&mut decoder, median, context)?;
                self.last_point.y = self.last_point.y.wrapping_add(diff);
                self.common
                    .last_y_diff_median
                    .get_unchecked_mut(m as usize)
                    .add(diff);

                // decompress z coordinate
                let k_bits = (self.ic_dx.k() + self.ic_dy.k()) / 2;
                let context = (n == 1) as u32
                    + if k_bits < 18 {
                        utils::u32_zero_bit(k_bits)
                    } else {
                        18
                    };
                self.last_point.z = self.ic_z.decompress(
                    &mut decoder,
                    *self.common.last_height.get_unchecked(l as usize),
                    context,
                )?;
                *self.common.last_height.get_unchecked_mut(l as usize) = self.last_point.z();
            }
            self.last_point.pack_into(buf);
            Ok(())
        }
    }
}
