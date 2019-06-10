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
use std::mem::size_of;

use crate::compressors::{IntegerCompressor, IntegerCompressorBuilder};
use crate::decoders::ArithmeticDecoder;
use crate::decompressors::{IntegerDecompressor, IntegerDecompressorBuilder};
use crate::encoders::ArithmeticEncoder;
use crate::formats::{FieldCompressor, FieldDecompressor};
use crate::las::utils;
use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
use crate::packers::Packable;

struct Point10ChangedValues {
    value: i32,
}

impl Point10ChangedValues {
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

#[derive(Default, Copy, Clone, PartialEq, Debug)]
pub struct Point10 {
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

impl Point10 {
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

    fn changed_values(&self, last: &Self, last_intensity: u16) -> Point10ChangedValues {
        // This logic here constructs a 5-bit changed value which is basically a bit map of what has changed
        // since the last point, not considering the x, y and z values

        let bit_fields_changed = ((last.return_number ^ self.return_number) != 0)
            | ((last.number_of_returns_of_given_pulse ^ self.number_of_returns_of_given_pulse)
                != 0)
            | (last.scan_direction_flag ^ self.scan_direction_flag)
            | (last.edge_of_flight_line ^ self.edge_of_flight_line);

        let intensity_changed = (last_intensity ^ self.intensity) != 0;
        let classification_changed = (last.classification ^ self.classification) != 0;
        let scan_angle_rank_changed = (last.scan_angle_rank ^ self.scan_angle_rank) != 0;
        let user_data_changed = (last.user_data ^ self.user_data) != 0;
        let point_source_id_changed = (last.point_source_id ^ self.point_source_id) != 0;

        Point10ChangedValues {
            value: (bit_fields_changed as i32) << 5
                | (intensity_changed as i32) << 4
                | (classification_changed as i32) << 3
                | (scan_angle_rank_changed as i32) << 2
                | (user_data_changed as i32) << 1
                | (point_source_id_changed as i32),
        }
    }
}

impl Packable for Point10 {
    type Type = Point10;

    fn unpack(input: &[u8]) -> Self::Type {
        let mut point = Point10::default();

        let mut start = 0;
        let mut end = size_of::<i32>();
        point.x = i32::unpack(&input[start..end]);
        start += size_of::<i32>();
        end += size_of::<i32>();
        point.y = i32::unpack(&input[start..end]);
        start += size_of::<i32>();
        end += size_of::<i32>();
        point.z = i32::unpack(&input[start..end]);

        start = end;
        end += size_of::<u16>();
        point.intensity = u16::unpack(&input[start..end]);

        start = end;
        end += size_of::<u8>();
        let bitfields = u8::unpack(&input[start..end]);
        point.populate_bit_fields_from(bitfields);

        start = end;
        end += size_of::<u8>();
        point.classification = u8::unpack(&input[start..end]);

        start = end;
        end += size_of::<i8>();
        point.scan_angle_rank = i8::unpack(&input[start..end]);

        start = end;
        end += size_of::<i8>();
        point.user_data = u8::unpack(&input[start..end]);

        start = end;
        end += size_of::<u16>();
        point.point_source_id = u16::unpack(&input[start..end]);

        point
    }

    fn pack(value: Self::Type, output: &mut [u8]) {
        let mut start = 0;
        let mut end = size_of::<i32>();

        i32::pack(value.x, &mut output[start..end]);
        start += size_of::<i32>();
        end += size_of::<i32>();
        i32::pack(value.y, &mut output[start..end]);
        start += size_of::<i32>();
        end += size_of::<i32>();
        i32::pack(value.z, &mut output[start..end]);

        start = end;
        end += size_of::<u16>();
        u16::pack(value.intensity, &mut output[start..end]);

        start = end;
        end += size_of::<u8>();
        u8::pack(value.bit_fields_to_byte(), &mut output[start..end]);

        start = end;
        end += size_of::<u8>();
        u8::pack(value.classification, &mut output[start..end]);

        start = end;
        end += size_of::<i8>();
        i8::pack(value.scan_angle_rank, &mut output[start..end]);

        start = end;
        end += size_of::<i8>();
        u8::pack(value.user_data, &mut output[start..end]);

        start = end;
        end += size_of::<u16>();
        u16::pack(value.point_source_id, &mut output[start..end]);
    }
}

// All the things we need to compress a point, group them into structs
// so we don't have too many names flying around
struct Common {
    last_point: Point10,
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

    have_last: bool,
}

impl Common {
    pub fn new() -> Self {
        Self {
            last_point: Default::default(),
            last_intensity: [0u16; 16],
            last_x_diff_median: (0..16)
                .into_iter()
                .map(|_i| utils::StreamingMedian::<i32>::new())
                .collect(),
            last_y_diff_median: (0..16)
                .into_iter()
                .map(|_i| utils::StreamingMedian::<i32>::new())
                .collect(),
            last_height: [0i32; 8],
            changed_values: ArithmeticModelBuilder::new(64).build(),
            scan_angle_rank: (0..2)
                .into_iter()
                .map(|_i| ArithmeticModelBuilder::new(256).build())
                .collect(),
            bit_byte: (0..256)
                .into_iter()
                .map(|_i| ArithmeticModelBuilder::new(256).build())
                .collect(),
            classification: (0..256)
                .into_iter()
                .map(|_i| ArithmeticModelBuilder::new(256).build())
                .collect(),
            user_data: (0..256)
                .into_iter()
                .map(|_i| ArithmeticModelBuilder::new(256).build())
                .collect(),
            have_last: false,
        }
    }
}

pub struct Point10Compressor {
    ic_intensity: IntegerCompressor,
    ic_point_source_id: IntegerCompressor,
    ic_dx: IntegerCompressor,
    ic_dy: IntegerCompressor,
    ic_z: IntegerCompressor,

    common: Common,
    compressor_inited: bool,
}

impl Point10Compressor {
    pub fn new() -> Self {
        Self {
            ic_intensity: IntegerCompressorBuilder::new().bits(16).contexts(4).build(),
            ic_point_source_id: IntegerCompressorBuilder::new().bits(16).build(),
            ic_dx: IntegerCompressorBuilder::new().bits(32).contexts(2).build(),
            ic_dy: IntegerCompressorBuilder::new()
                .bits(32)
                .contexts(22)
                .build(),
            ic_z: IntegerCompressorBuilder::new()
                .bits(32)
                .contexts(20)
                .build(),
            common: Common::new(),
            compressor_inited: false,
        }
    }
}

impl<W: Write> FieldCompressor<W> for Point10Compressor {
    fn size_of_field(&self) -> usize {
        20
    }

    fn compress_with(&mut self, mut encoder: &mut ArithmeticEncoder<W>, buf: &[u8]) {
        let this_val = Point10::unpack(&buf);

        if !self.compressor_inited {
            self.ic_intensity.init();
            self.ic_point_source_id.init();
            self.ic_dx.init();
            self.ic_dy.init();
            self.ic_z.init();
            self.compressor_inited = true;
        }

        if !self.common.have_last {
            // don't have the first data yet, just push it to our have last stuff and move on
            self.common.have_last = true;
            self.common.last_point = this_val;

            encoder.out_stream().write_all(&buf).unwrap();
        } else {
            let r = this_val.return_number;
            let n = this_val.number_of_returns_of_given_pulse;
            let m = utils::NUMBER_RETURN_MAP[n as usize][r as usize];
            let l = utils::NUMBER_RETURN_LEVEL[n as usize][r as usize];

            let changed_values = this_val.changed_values(
                &self.common.last_point,
                self.common.last_intensity[m as usize],
            );
            // compress which other values have changed

            encoder.encode_symbol(&mut self.common.changed_values, changed_values.value as u32);

            if changed_values.bit_fields_changed() {
                println!("-> bit fields");
                let b = this_val.bit_fields_to_byte();
                let last_b = self.common.last_point.bit_fields_to_byte();
                encoder.encode_symbol(&mut self.common.bit_byte[last_b as usize], b as u32);
            }

            if changed_values.intensity_changed() {
                println!("-> Intensity");
                self.ic_intensity.compress(
                    &mut encoder,
                    self.common.last_intensity[m as usize] as i32,
                    this_val.intensity as i32,
                    if m < 3 { m as u32 } else { 3 },
                );
                self.common.last_intensity[m as usize] = this_val.intensity;
            }

            if changed_values.classification_changed() {
                println!("-> Classification");
                encoder.encode_symbol(
                    &mut self.common.classification[self.common.last_point.classification as usize],
                    this_val.classification as u32,
                );
            }

            if changed_values.scan_angle_rank_changed() {
                println!("-> Scan angle rank");
                // the "as u8" before "as u32" is vital
                encoder.encode_symbol(
                    &mut self.common.scan_angle_rank[this_val.scan_direction_flag as usize],
                    (this_val.scan_angle_rank - self.common.last_point.scan_angle_rank) as u8
                        as u32,
                );
            }

            if changed_values.user_data_changed() {
                println!("-> user_data");
                encoder.encode_symbol(
                    &mut self.common.user_data[self.common.last_point.user_data as usize],
                    this_val.user_data as u32,
                );
            }

            if changed_values.point_source_id_changed() {
                println!("-> Point source ID");
                self.ic_point_source_id.compress(
                    &mut encoder,
                    self.common.last_point.point_source_id as i32,
                    this_val.point_source_id as i32,
                    0,
                );
            }

            println!("-> X");
            //compress x coordinates
            let median = self.common.last_x_diff_median[m as usize].get();
            let diff = this_val.x - self.common.last_point.x;
            self.ic_dx
                .compress(&mut encoder, median, diff, (n == 1) as u32);
            self.common.last_x_diff_median[m as usize].add(diff);

            println!("-> Y");
            //compress y coordinates
            let k_bits = self.ic_dx.k();
            let median = self.common.last_y_diff_median[m as usize].get();
            let diff = this_val.y - self.common.last_point.y;
            let context = (n == 1) as u32
                + if k_bits < 20 {
                    utils::u32_zero_bit(k_bits)
                } else {
                    20
                };
            self.ic_dy.compress(&mut encoder, median, diff, context);
            self.common.last_y_diff_median[m as usize].add(diff);

            println!("-> Z");
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
                self.common.last_height[l as usize],
                this_val.z,
                context,
            );
            self.common.last_height[l as usize] = this_val.z;

            self.common.last_point = this_val;
        }
    }
}

pub struct Point10Decompressor {
    ic_intensity: IntegerDecompressor,
    ic_point_source_id: IntegerDecompressor,
    ic_dx: IntegerDecompressor,
    ic_dy: IntegerDecompressor,
    ic_z: IntegerDecompressor,

    common: Common,
    decompressor_inited: bool,
}

impl Point10Decompressor {
    pub fn new() -> Self {
        Self {
            ic_intensity: IntegerDecompressorBuilder::new()
                .bits(16)
                .contexts(4)
                .build(),
            ic_point_source_id: IntegerDecompressorBuilder::new().bits(16).build(),
            ic_dx: IntegerDecompressorBuilder::new()
                .bits(32)
                .contexts(2)
                .build(),
            ic_dy: IntegerDecompressorBuilder::new()
                .bits(32)
                .contexts(22)
                .build(),
            ic_z: IntegerDecompressorBuilder::new()
                .bits(32)
                .contexts(20)
                .build(),
            common: Common::new(),
            decompressor_inited: false,
        }
    }
}

impl<R: Read> FieldDecompressor<R> for Point10Decompressor {
    fn size_of_field(&self) -> usize {
        20
    }

    fn decompress_with(&mut self, mut decoder: &mut ArithmeticDecoder<R>, mut buf: &mut [u8]) {
        if !self.decompressor_inited {
            self.ic_intensity.init();
            self.ic_point_source_id.init();
            self.ic_dx.init();
            self.ic_dy.init();
            self.ic_z.init();
            self.decompressor_inited = true;
        }

        if !self.common.have_last {
            decoder.in_stream().read_exact(&mut buf).unwrap();
            self.common.last_point = Point10::unpack(&buf);
            self.common.have_last = true;
        } else {
            let changed_value = Point10ChangedValues {
                value: decoder.decode_symbol(&mut self.common.changed_values) as i32,
            };

            let r;
            let n;
            let m;
            let l;

            if changed_value.value != 0 {
                // there was some change in one of the fields (other than x, y and z)

                if changed_value.bit_fields_changed() {
                    println!("bit_fields");
                    let mut b = self.common.last_point.bit_fields_to_byte();
                    b = decoder.decode_symbol(&mut self.common.bit_byte[b as usize]) as u8;
                    self.common.last_point.populate_bit_fields_from(b);
                }

                r = self.common.last_point.return_number;
                n = self.common.last_point.number_of_returns_of_given_pulse;
                m = utils::NUMBER_RETURN_MAP[n as usize][r as usize];
                l = utils::NUMBER_RETURN_LEVEL[n as usize][r as usize];

                if changed_value.intensity_changed() {
                    println!(
                        "Intensity -> m: {}, last intensity: {}",
                        m, self.common.last_intensity[m as usize]
                    );
                    self.common.last_point.intensity = self.ic_intensity.decompress(
                        &mut decoder,
                        self.common.last_intensity[m as usize] as i32,
                        if m < 3 { m as u32 } else { 3 },
                    ) as u16;
                    self.common.last_intensity[m as usize] = self.common.last_point.intensity;
                } else {
                    self.common.last_point.intensity = self.common.last_intensity[m as usize];
                }

                if changed_value.classification_changed() {
                    println!("Classification");
                    self.common.last_point.classification = decoder.decode_symbol(
                        &mut self.common.classification
                            [self.common.last_point.classification as usize],
                    ) as u8;
                }

                if changed_value.scan_angle_rank_changed() {
                    println!("Scna angle rank");
                    let val = decoder.decode_symbol(
                        &mut self.common.scan_angle_rank
                            [self.common.last_point.scan_direction_flag as usize],
                    ) as i8;
                    self.common.last_point.scan_angle_rank =
                        val + self.common.last_point.scan_angle_rank;
                }

                if changed_value.user_data_changed() {
                    println!("user_data");
                    self.common.last_point.user_data = decoder.decode_symbol(
                        &mut self.common.user_data[self.common.last_point.user_data as usize],
                    ) as u8;
                }

                if changed_value.point_source_id_changed() {
                    println!("point_source_id");
                    self.common.last_point.point_source_id = self.ic_point_source_id.decompress(
                        &mut decoder,
                        self.common.last_point.point_source_id as i32,
                        0,
                    ) as u16;
                }
            } else {
                r = self.common.last_point.return_number;
                n = self.common.last_point.number_of_returns_of_given_pulse;
                m = utils::NUMBER_RETURN_MAP[n as usize][r as usize];
                l = utils::NUMBER_RETURN_LEVEL[n as usize][r as usize];
            }

            println!("-> X");
            // decompress x
            let median = self.common.last_x_diff_median[m as usize].get();
            let diff = self.ic_dx.decompress(&mut decoder, median, (n == 1) as u32);
            self.common.last_point.x += diff;
            self.common.last_x_diff_median[m as usize].add(diff);

            println!("-> Y");
            // decompress y
            let median = self.common.last_y_diff_median[m as usize].get();
            let k_bits = self.ic_dx.k();
            let context = (n == 1) as u32
                + if k_bits < 20 {
                    utils::u32_zero_bit(k_bits)
                } else {
                    20
                };
            let diff = self.ic_dy.decompress(&mut decoder, median, context);
            self.common.last_point.y += diff;
            self.common.last_y_diff_median[m as usize].add(diff);

            println!("-> Z");
            // decompress z coordinate
            let k_bits = (self.ic_dx.k() + self.ic_dy.k()) / 2;
            let context = (n == 1) as u32
                + if k_bits < 18 {
                    utils::u32_zero_bit(k_bits)
                } else {
                    18
                };
            self.common.last_point.z =
                self.ic_z
                    .decompress(&mut decoder, self.common.last_height[l as usize], context);
            self.common.last_height[l as usize] = self.common.last_point.z;

            //TODO here last point is not taken by ref, but by copy, this may cause perf problems
            Point10::pack(self.common.last_point, &mut buf);
        }
    }
}
