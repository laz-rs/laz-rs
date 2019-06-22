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

use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};

use crate::packers::Packable;

const LASZIP_GPS_TIME_MULTI: i32 = 500;
const LASZIP_GPS_TIME_MULTI_MINUS: i32 = -10;
const LASZIP_GPS_TIME_MULTI_UNCHANGED: i32 =
    (LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 1);
const LASZIP_GPS_TIME_MULTI_CODE_FULL: i32 =
    (LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 2);

const LASZIP_GPS_TIME_MULTI_TOTAL: i32 = (LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 6);

#[inline]
fn i32_quantize(n: f32) -> i32 {
    if n >= 0.0f32 {
        (n + 0.5f32) as i32
    } else {
        (n - 0.5f32) as i32
    }
}

pub trait LasGpsTime {
    fn gps_time(&self) -> f64;
    fn set_gps_time(&mut self, new_value: f64);

    fn read_from<R: Read>(&mut self, src: &mut R) -> std::io::Result<()> {
        self.set_gps_time(src.read_f64::<LittleEndian>()?);
        Ok(())
    }
}

#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct GpsTime {
    pub value: i64,
}

impl LasGpsTime for GpsTime {
    fn gps_time(&self) -> f64 {
        f64::from_bits(self.value as u64)
    }

    fn set_gps_time(&mut self, new_value: f64) {
        self.value = new_value.to_bits() as i64;
    }
}

impl Packable for GpsTime {
    type Type = GpsTime;

    fn unpack_from(input: &[u8]) -> Self::Type {
        let lower = u32::unpack_from(&input[0..std::mem::size_of::<u32>()]);
        let upper =
            u32::unpack_from(&input[std::mem::size_of::<u32>()..(2 * std::mem::size_of::<u32>())]);

        GpsTime {
            value: (upper as i64) << 32 | lower as i64,
        }
    }

    fn pack_into(&self, output: &mut [u8]) {
        u32::pack_into(
            &((self.value & 0xFFFFFFFF) as u32),
            &mut output[0..std::mem::size_of::<u32>()],
        );
        u32::pack_into(
            &((self.value >> 32) as u32),
            &mut output[std::mem::size_of::<u32>()..(2 * std::mem::size_of::<u32>())],
        );
    }
}

pub mod v1 {
    use std::io::{Read, Write};

    use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
    use num_traits::clamp;

    use crate::compressors::{
        IntegerCompressor, IntegerCompressorBuilder, DEFAULT_COMPRESS_CONTEXTS,
    };
    use crate::decoders::ArithmeticDecoder;
    use crate::decompressors::{IntegerDecompressor, IntegerDecompressorBuilder};
    use crate::encoders::ArithmeticEncoder;
    use crate::las::gps::LasGpsTime;
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;
    use crate::record::{
        BufferFieldCompressor, BufferFieldDecompressor, PointFieldCompressor,
        PointFieldDecompressor,
    };

    use super::GpsTime;

    const LASZIP_GPS_TIME_MULTI_MAX: u32 = 512;

    pub struct LasGpsTimeDecompressor {
        last_gps: i64,
        gps_time_multi_model: ArithmeticModel,
        gps_time_0_diff_model: ArithmeticModel,
        ic_gps_time: IntegerDecompressor,
        multi_extreme_counter: i32,
        last_gps_time_diff: i32,
    }

    impl LasGpsTimeDecompressor {
        pub fn new() -> Self {
            Self {
                last_gps: 0,
                gps_time_multi_model: ArithmeticModelBuilder::new(LASZIP_GPS_TIME_MULTI_MAX)
                    .build(),
                gps_time_0_diff_model: ArithmeticModelBuilder::new(3).build(),
                ic_gps_time: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(6)
                    .build_initialized(),
                multi_extreme_counter: 0,
                last_gps_time_diff: 0,
            }
        }
    }

    impl<R: Read, P: LasGpsTime> PointFieldDecompressor<R, P> for LasGpsTimeDecompressor {
        fn init_first_point(&mut self, src: &mut R, first_point: &mut P) -> std::io::Result<()> {
            let gps_value = src.read_f64::<LittleEndian>()?;
            first_point.set_gps_time(gps_value);
            self.last_gps = gps_value.to_bits() as i64;
            Ok(())
        }

        fn decompress_field_with(
            &mut self,
            mut decoder: &mut ArithmeticDecoder<R>,
            current_point: &mut P,
        ) -> std::io::Result<()> {
            let multi;
            if self.last_gps_time_diff == 0 {
                multi = decoder.decode_symbol(&mut self.gps_time_0_diff_model)?;
                if multi == 1 {
                    // the difference can be represented with 32 bits
                    self.last_gps_time_diff = self.ic_gps_time.decompress(&mut decoder, 0, 0)?;
                    self.last_gps += self.last_gps_time_diff as i64;
                } else if multi == 2 {
                    // the difference is huge,
                    // the gps was written as is
                    self.last_gps = decoder.read_int_64()? as i64;
                }
            } else {
                multi = decoder.decode_symbol(&mut self.gps_time_multi_model)?;

                if multi < LASZIP_GPS_TIME_MULTI_MAX - 2 {
                    let gps_time_diff: i32;
                    if multi == 1 {
                        gps_time_diff = self.ic_gps_time.decompress(
                            &mut decoder,
                            self.last_gps_time_diff,
                            1,
                        )?;
                        self.last_gps_time_diff = gps_time_diff;
                        self.multi_extreme_counter = 0;
                    } else if multi == 0 {
                        gps_time_diff = self.ic_gps_time.decompress(
                            &mut decoder,
                            self.last_gps_time_diff / 4,
                            2,
                        )?;
                        self.multi_extreme_counter += 1;
                        if self.multi_extreme_counter > 3 {
                            self.last_gps_time_diff = gps_time_diff;
                            self.multi_extreme_counter = 0;
                        }
                    } else {
                        let context = if multi < 10 {
                            3
                        } else if multi < 50 {
                            4
                        } else {
                            5
                        };

                        gps_time_diff = self.ic_gps_time.decompress(
                            &mut decoder,
                            self.last_gps_time_diff * multi as i32,
                            context,
                        )?;

                        if multi == LASZIP_GPS_TIME_MULTI_MAX - 3 {
                            self.multi_extreme_counter += 1;
                            if self.multi_extreme_counter > 3 {
                                self.last_gps_time_diff = gps_time_diff;
                                self.multi_extreme_counter = 0;
                            }
                        }
                    }
                    self.last_gps += gps_time_diff as i64;
                } else if multi < LASZIP_GPS_TIME_MULTI_MAX - 1 {
                    self.last_gps = decoder.read_int_64()? as i64;
                }
            }
            current_point.set_gps_time(f64::from_bits(self.last_gps as u64));
            Ok(())
        }
    }

    pub struct LasGpsTimeCompressor {
        last_gps: i64,
        gps_time_multi_model: ArithmeticModel,
        gps_time_0_diff_model: ArithmeticModel,
        ic_gps_time: IntegerCompressor,
        multi_extreme_counter: i32,
        last_gps_time_diff: i32,
    }

    impl LasGpsTimeCompressor {
        pub fn new() -> Self {
            Self {
                last_gps: 0,
                gps_time_multi_model: ArithmeticModelBuilder::new(LASZIP_GPS_TIME_MULTI_MAX)
                    .build(),
                gps_time_0_diff_model: ArithmeticModelBuilder::new(3).build(),
                ic_gps_time: IntegerCompressorBuilder::new()
                    .bits(32)
                    .contexts(6)
                    .build_initialized(),
                multi_extreme_counter: 0,
                last_gps_time_diff: 0,
            }
        }
    }

    impl<W: Write, P: LasGpsTime> PointFieldCompressor<W, P> for LasGpsTimeCompressor {
        fn init_first_point(&mut self, dst: &mut W, first_point: &P) -> std::io::Result<()> {
            dst.write_f64::<LittleEndian>(first_point.gps_time())?;
            self.last_gps = first_point.gps_time().to_bits() as i64;
            Ok(())
        }

        fn compress_field_with(
            &mut self,
            mut encoder: &mut ArithmeticEncoder<W>,
            current_point: &P,
        ) -> std::io::Result<()> {
            let current_gps_time_value = current_point.gps_time().to_bits() as i64;

            if self.last_gps_time_diff == 0 {
                if current_gps_time_value == self.last_gps {
                    encoder.encode_symbol(&mut self.gps_time_0_diff_model, 0)?;
                } else {
                    let current_gps_time_diff_64 = current_gps_time_value - self.last_gps;
                    let current_gps_time_diff_32 = current_gps_time_diff_64 as i32;

                    if current_gps_time_diff_64 == current_gps_time_diff_32 as i64 {
                        // this difference can be represented with 32 bits
                        encoder.encode_symbol(&mut self.gps_time_0_diff_model, 1)?;
                        self.ic_gps_time.compress(
                            &mut encoder,
                            0,
                            current_gps_time_diff_32,
                            DEFAULT_COMPRESS_CONTEXTS,
                        )?;
                        self.last_gps_time_diff = current_gps_time_diff_32;
                    } else {
                        encoder.encode_symbol(&mut self.gps_time_0_diff_model, 2)?; // the difference is huge
                        encoder.write_int64(current_gps_time_value as u64)?;
                    }
                    self.last_gps = current_gps_time_value;
                }
            } else {
                //difference was not zero
                if current_gps_time_value == self.last_gps {
                    // if the doubles have not changed use a special symbol
                    encoder.encode_symbol(
                        &mut self.gps_time_multi_model,
                        LASZIP_GPS_TIME_MULTI_MAX - 1,
                    )?;
                } else {
                    let current_gps_time_diff_64 = current_gps_time_value - self.last_gps;
                    let current_gps_time_diff_32 = current_gps_time_diff_64 as i32;

                    if current_gps_time_diff_64 == current_gps_time_diff_32 as i64 {
                        // compute multiplier between current and last integer difference
                        let mut multi = ((current_gps_time_diff_32 as f32
                            / (self.last_gps_time_diff as f32))
                            + 0.5f32) as i32;

                        // limit the multiplier into some bounds
                        multi = clamp(multi, 0, (LASZIP_GPS_TIME_MULTI_MAX - 3) as i32);
                        // compress this multiplier
                        encoder.encode_symbol(&mut self.gps_time_multi_model, multi as u32)?;
                        // compress the residual curr_gpstime_diff in dependance on the multiplier
                        if multi == 1 {
                            self.ic_gps_time.compress(
                                &mut encoder,
                                self.last_gps_time_diff,
                                current_gps_time_diff_32,
                                1,
                            )?;
                            self.last_gps_time_diff = current_gps_time_diff_32;
                            self.multi_extreme_counter = 0;
                        } else if multi == 0 {
                            self.ic_gps_time.compress(
                                &mut encoder,
                                self.last_gps_time_diff / 4,
                                current_gps_time_diff_32,
                                2,
                            )?;
                            self.multi_extreme_counter += 1;
                            if self.multi_extreme_counter > 3 {
                                self.last_gps_time_diff = current_gps_time_diff_32;
                                self.multi_extreme_counter = 0;
                            }
                        } else if multi < 10 {
                            //TODO simplify following if elses ?
                            self.ic_gps_time.compress(
                                &mut encoder,
                                self.last_gps_time_diff * multi,
                                current_gps_time_diff_32,
                                3,
                            )?;
                        } else if multi < 50 {
                            self.ic_gps_time.compress(
                                &mut encoder,
                                self.last_gps_time_diff * multi,
                                current_gps_time_diff_32,
                                4,
                            )?;
                        } else {
                            self.ic_gps_time.compress(
                                &mut encoder,
                                self.last_gps_time_diff * multi,
                                current_gps_time_diff_32,
                                5,
                            )?;
                            if multi == (LASZIP_GPS_TIME_MULTI_MAX - 3) as i32 {
                                self.multi_extreme_counter += 1;
                                if self.multi_extreme_counter > 3 {
                                    self.last_gps_time_diff = current_gps_time_diff_32;
                                    self.multi_extreme_counter = 0;
                                }
                            }
                        }
                    } else {
                        // Note orignal code says '-2' but looking at the decompress
                        // shouldn't it be '-1' (or vice versa) ?
                        // if difference is so huge ... we simply write the double
                        encoder.encode_symbol(
                            &mut self.gps_time_multi_model,
                            LASZIP_GPS_TIME_MULTI_MAX - 2,
                        )?;
                        encoder.write_int64(current_gps_time_value as u64)?;
                    }
                }
            }
            self.last_gps = current_gps_time_value;
            Ok(())
        }
    }

    impl<W: Write> BufferFieldCompressor<W> for LasGpsTimeCompressor {
        fn size_of_field(&self) -> usize {
            std::mem::size_of::<f64>()
        }

        fn compress_first(&mut self, mut dst: &mut W, buf: &[u8]) -> std::io::Result<()> {
            let current = GpsTime::unpack_from(buf);
            self.init_first_point(&mut dst, &current)?;
            Ok(())
        }

        fn compress_with(
            &mut self,
            mut encoder: &mut ArithmeticEncoder<W>,
            buf: &[u8],
        ) -> std::io::Result<()> {
            let current = GpsTime::unpack_from(buf);
            self.compress_field_with(&mut encoder, &current)?;
            Ok(())
        }
    }

    impl<R: Read> BufferFieldDecompressor<R> for LasGpsTimeDecompressor {
        fn size_of_field(&self) -> usize {
            std::mem::size_of::<f64>()
        }

        fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()> {
            let mut current_value = GpsTime::default();
            self.init_first_point(src, &mut current_value)?;
            current_value.pack_into(first_point);
            Ok(())
        }

        fn decompress_with(
            &mut self,
            mut decoder: &mut ArithmeticDecoder<R>,
            buf: &mut [u8],
        ) -> std::io::Result<()> {
            let mut current_value = GpsTime::default();
            self.decompress_field_with(&mut decoder, &mut current_value)?;
            current_value.pack_into(buf);
            Ok(())
        }
    }

}

pub mod v2 {
    use std::io::{Read, Write};

    use byteorder::{LittleEndian, WriteBytesExt};

    use crate::compressors::{IntegerCompressor, IntegerCompressorBuilder};
    use crate::decoders::ArithmeticDecoder;
    use crate::decompressors::{IntegerDecompressor, IntegerDecompressorBuilder};
    use crate::encoders::ArithmeticEncoder;
    use crate::las::gps::LasGpsTime;
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;
    use crate::record::{
        BufferFieldCompressor, BufferFieldDecompressor, PointFieldCompressor,
        PointFieldDecompressor,
    };

    use super::{
        i32_quantize, GpsTime, LASZIP_GPS_TIME_MULTI, LASZIP_GPS_TIME_MULTI_CODE_FULL,
        LASZIP_GPS_TIME_MULTI_MINUS, LASZIP_GPS_TIME_MULTI_TOTAL, LASZIP_GPS_TIME_MULTI_UNCHANGED,
    };

    // Common parts for both a compressor and decompressor go here
    struct Common {
        gps_time_multi: ArithmeticModel,
        gps_time_0_diff: ArithmeticModel,
        last: usize,
        next: usize,
        last_gps_times: [GpsTime; 4],
        last_gps_time_diffs: [i32; 4],
        multi_extreme_counters: [i32; 4],
    }

    impl Common {
        pub fn new() -> Self {
            Self {
                gps_time_multi: ArithmeticModelBuilder::new(LASZIP_GPS_TIME_MULTI_TOTAL as u32)
                    .build(),
                gps_time_0_diff: ArithmeticModelBuilder::new(6).build(),
                last: 0,
                next: 0,
                last_gps_times: [GpsTime::default(); 4],
                last_gps_time_diffs: [0i32; 4],
                multi_extreme_counters: [0i32; 4],
            }
        }
    }

    pub struct GpsTimeCompressor {
        ic_gps_time: IntegerCompressor,
        common: Common,
    }

    impl GpsTimeCompressor {
        pub fn new() -> Self {
            Self {
                ic_gps_time: IntegerCompressorBuilder::new()
                    .bits(32)
                    .contexts(9)
                    .build_initialized(),
                common: Common::new(),
            }
        }
    }

    impl<W: Write, P: LasGpsTime> PointFieldCompressor<W, P> for GpsTimeCompressor {
        fn init_first_point(&mut self, dst: &mut W, first_point: &P) -> std::io::Result<()> {
            self.common.last_gps_times[0].value = first_point.gps_time().to_bits() as i64;
            dst.write_f64::<LittleEndian>(first_point.gps_time())
        }

        fn compress_field_with(
            &mut self,
            mut encoder: &mut ArithmeticEncoder<W>,
            current_point: &P,
        ) -> std::io::Result<()> {
            let this_val = GpsTime {
                value: current_point.gps_time().to_bits() as i64,
            };
            if self.common.last_gps_time_diffs[self.common.last] == 0 {
                if this_val.value == self.common.last_gps_times[self.common.last].value {
                    encoder.encode_symbol(&mut self.common.gps_time_0_diff, 0)?;
                } else {
                    // calculate the difference between the two doubles as an integer
                    let curr_gps_time_diff_64 =
                        this_val.value - self.common.last_gps_times[self.common.last].value;
                    let curr_gps_time_diff_32 = curr_gps_time_diff_64 as i32;

                    if curr_gps_time_diff_64 == curr_gps_time_diff_32 as i64 {
                        // this difference is small enough to be represented with 32 bits
                        encoder.encode_symbol(&mut self.common.gps_time_0_diff, 1)?;
                        self.ic_gps_time
                            .compress(&mut encoder, 0, curr_gps_time_diff_32, 0)?;
                        self.common.last_gps_time_diffs[self.common.last] = curr_gps_time_diff_32;
                        self.common.multi_extreme_counters[self.common.last] = 0;
                    } else {
                        // the difference is huge
                        // maybe the double belongs to another time sequence
                        for i in 1..4 {
                            let other_gps_time_diff_64 = this_val.value
                                - self.common.last_gps_times[((self.common.last + i) & 3)].value;
                            let other_gps_time_diff_32 = other_gps_time_diff_64 as i32;

                            if other_gps_time_diff_64 == other_gps_time_diff_32 as i64 {
                                // it belongs to another sequence
                                encoder.encode_symbol(
                                    &mut self.common.gps_time_0_diff,
                                    (i + 2) as u32,
                                )?;
                                self.common.last = (self.common.last + i) & 3;
                                return self.compress_field_with(&mut encoder, current_point);
                            }
                        }
                        // no other sequence found. start new sequence.
                        encoder.encode_symbol(&mut self.common.gps_time_0_diff, 2)?;
                        self.ic_gps_time.compress(
                            &mut encoder,
                            (self.common.last_gps_times[self.common.last].value >> 32) as i32,
                            (this_val.value >> 32) as i32,
                            8,
                        )?;

                        encoder.write_int(this_val.value as u32)?;

                        self.common.next = (self.common.next + 1) & 3;
                        self.common.last = self.common.next;
                        self.common.last_gps_time_diffs[self.common.last] = 0;
                        self.common.multi_extreme_counters[self.common.last] = 0;
                    }
                    self.common.last_gps_times[self.common.last] = this_val;
                }
            } else {
                //the last integer difference was *not* zero
                if this_val.value == self.common.last_gps_times[self.common.last as usize].value {
                    // if the doubles have not changed use a special symbol
                    encoder.encode_symbol(
                        &mut self.common.gps_time_multi,
                        LASZIP_GPS_TIME_MULTI_UNCHANGED as u32,
                    )?;
                } else {
                    // the last integer difference was *not* zero
                    let curr_gps_time_diff_64 =
                        this_val.value - self.common.last_gps_times[self.common.last].value;
                    let curr_gps_time_diff_32 = curr_gps_time_diff_64 as i32;

                    // if the current gps time difference can be represented with 32 bits
                    if curr_gps_time_diff_64 == curr_gps_time_diff_32 as i64 {
                        // compute multiplier between current and last integer difference
                        let multi_f = curr_gps_time_diff_32 as f32
                            / self.common.last_gps_time_diffs[self.common.last] as f32;
                        let multi = i32_quantize(multi_f);

                        // compress the residual curr_gps_time_diff in dependance on the multiplier
                        if multi == 1 {
                            // this is the case we assume we get most often for regular spaced pulses
                            encoder.encode_symbol(&mut self.common.gps_time_multi, 1)?;
                            self.ic_gps_time.compress(
                                &mut encoder,
                                self.common.last_gps_time_diffs[self.common.last],
                                curr_gps_time_diff_32,
                                1,
                            )?;
                            self.common.multi_extreme_counters[self.common.last] = 0;
                        } else if multi > 0 {
                            if multi < LASZIP_GPS_TIME_MULTI {
                                // positive multipliers up to LASZIP_GPSTIME_MULTI are compressed directly
                                encoder
                                    .encode_symbol(&mut self.common.gps_time_multi, multi as u32)?;
                                let context = if multi < 10 { 2u32 } else { 3u32 };
                                self.ic_gps_time.compress(
                                    &mut encoder,
                                    multi.wrapping_mul(
                                        self.common.last_gps_time_diffs[self.common.last],
                                    ),
                                    curr_gps_time_diff_32,
                                    context,
                                )?;
                            } else {
                                encoder.encode_symbol(
                                    &mut self.common.gps_time_multi,
                                    LASZIP_GPS_TIME_MULTI as u32,
                                )?;
                                self.ic_gps_time.compress(
                                    &mut encoder,
                                    LASZIP_GPS_TIME_MULTI.wrapping_mul(
                                        self.common.last_gps_time_diffs[self.common.last],
                                    ),
                                    curr_gps_time_diff_32,
                                    4,
                                )?;

                                let multi_extreme_counter = &mut self.common.multi_extreme_counters[self.common.last];
                                *multi_extreme_counter +=1 ;
                                if *multi_extreme_counter > 3 {
                                    self.common.last_gps_time_diffs[self.common.last] =
                                        curr_gps_time_diff_32;
                                    *multi_extreme_counter = 0;
                                }
                            }
                        } else if multi < 0 {
                            if multi > LASZIP_GPS_TIME_MULTI_MINUS {
                                // negative multipliers larger than LASZIP_GPSTIME_MULTI_MINUS are compressed directly
                                encoder.encode_symbol(
                                    &mut self.common.gps_time_multi,
                                    (LASZIP_GPS_TIME_MULTI - multi) as u32,
                                )?;
                                self.ic_gps_time.compress(
                                    &mut encoder,
                                    multi.wrapping_mul(
                                        self.common.last_gps_time_diffs[self.common.last],
                                    ),
                                    curr_gps_time_diff_32,
                                    5,
                                )?;
                            } else {
                                encoder.encode_symbol(
                                    &mut self.common.gps_time_multi,
                                    (LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS) as u32,
                                )?;
                                self.ic_gps_time.compress(
                                    &mut encoder,
                                    LASZIP_GPS_TIME_MULTI_MINUS.wrapping_mul(
                                        self.common.last_gps_time_diffs[self.common.last],
                                    ),
                                    curr_gps_time_diff_32,
                                    6,
                                )?;
                                let multi_extreme_counter = &mut self.common.multi_extreme_counters[self.common.last];
                                *multi_extreme_counter +=1 ;
                                if *multi_extreme_counter > 3 {
                                    self.common.last_gps_time_diffs[self.common.last] =
                                        curr_gps_time_diff_32;
                                    *multi_extreme_counter = 0;
                                }
                            }
                        } else {
                            encoder.encode_symbol(&mut self.common.gps_time_multi, 0)?;
                            self.ic_gps_time
                                .compress(&mut encoder, 0, curr_gps_time_diff_32, 7)?;
                            let multi_extreme_counter = &mut self.common.multi_extreme_counters[self.common.last];
                            *multi_extreme_counter +=1 ;
                            if *multi_extreme_counter > 3 {
                                self.common.last_gps_time_diffs[self.common.last] =
                                    curr_gps_time_diff_32;
                                *multi_extreme_counter = 0;
                            }
                        }
                    } else {
                        // the difference is huge
                        // maybe the double belongs to another time sequence
                        for i in 1..4 {
                            let other_gps_time_diff_64 = this_val.value
                                - self.common.last_gps_times[((self.common.last + i) & 3)].value;
                            let other_gps_time_diff_32 = other_gps_time_diff_64 as i32;

                            if other_gps_time_diff_64 == other_gps_time_diff_32 as i64 {
                                // it belongs to this sequence
                                encoder.encode_symbol(
                                    &mut self.common.gps_time_multi,
                                    (LASZIP_GPS_TIME_MULTI_CODE_FULL + i as i32) as u32,
                                )?;
                                self.common.last = (self.common.last + i) & 3;
                                return self.compress_field_with(&mut encoder, current_point);
                            }
                        }

                        // no other sequence found start a new one
                        encoder.encode_symbol(
                            &mut self.common.gps_time_multi,
                            LASZIP_GPS_TIME_MULTI_CODE_FULL as u32,
                        )?;
                        self.ic_gps_time.compress(
                            &mut encoder,
                            (self.common.last_gps_times[self.common.last as usize].value >> 32)
                                as i32,
                            (this_val.value >> 32) as i32,
                            8,
                        )?;

                        encoder.write_int(this_val.value as u32)?;
                        self.common.next = (self.common.next + 1) & 3;
                        self.common.last = self.common.next;
                        self.common.last_gps_time_diffs[self.common.last] = 0;
                        self.common.multi_extreme_counters[self.common.last] = 0;
                    }
                    self.common.last_gps_times[self.common.last] = this_val;
                }
            }
            Ok(())
        }
    }

    impl<W: Write> BufferFieldCompressor<W> for GpsTimeCompressor {
        fn size_of_field(&self) -> usize {
            std::mem::size_of::<i64>()
        }

        fn compress_first(&mut self, mut dst: &mut W, buf: &[u8]) -> std::io::Result<()> {
            let this_val = GpsTime::unpack_from(&buf);
            self.init_first_point(&mut dst, &this_val)
        }

        fn compress_with(
            &mut self,
            mut encoder: &mut ArithmeticEncoder<W>,
            buf: &[u8],
        ) -> std::io::Result<()> {
            let this_val = GpsTime::unpack_from(&buf);
            self.compress_field_with(&mut encoder, &this_val)?;
            Ok(())
        }
    }

    pub struct GpsTimeDecompressor {
        common: Common,
        ic_gps_time: IntegerDecompressor,
    }

    impl GpsTimeDecompressor {
        pub fn new() -> Self {
            Self {
                common: Common::new(),
                ic_gps_time: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(9)
                    .build_initialized(),
            }
        }
    }

    impl<R: Read, P: LasGpsTime> PointFieldDecompressor<R, P> for GpsTimeDecompressor {
        fn init_first_point(
            &mut self,
            mut src: &mut R,
            first_point: &mut P,
        ) -> std::io::Result<()> {
            self.common.last_gps_times[0].read_from(&mut src)?;
            first_point.set_gps_time(f64::from_bits(self.common.last_gps_times[0].value as u64));
            Ok(())
        }

        fn decompress_field_with(
            &mut self,
            mut decoder: &mut ArithmeticDecoder<R>,
            last_point: &mut P,
        ) -> std::io::Result<()> {
            let mut multi: i32;

            if self.common.last_gps_time_diffs[self.common.last as usize] == 0 {
                // it the last integer difference was zero
                multi = decoder.decode_symbol(&mut self.common.gps_time_0_diff)? as i32;

                if multi == 1 {
                    // the difference can be represented with 32 bits
                    self.common.last_gps_time_diffs[self.common.last as usize] =
                        self.ic_gps_time.decompress(&mut decoder, 0, 0)?;
                    self.common.last_gps_times[self.common.last as usize].value +=
                        self.common.last_gps_time_diffs[self.common.last as usize] as i64;
                    self.common.multi_extreme_counters[self.common.last as usize] = 0;
                } else if multi == 2 {
                    // the difference is huge
                    self.common.next = (self.common.next + 1) & 3;
                    self.common.last_gps_times[self.common.next as usize].value =
                        self.ic_gps_time.decompress(
                            &mut decoder,
                            (self.common.last_gps_times[self.common.last as usize].value >> 32)
                                as i32,
                            8,
                        )? as i64;
                    self.common.last_gps_times[self.common.next as usize].value <<= 32;
                    self.common.last_gps_times[self.common.next as usize].value |=
                        decoder.read_int()? as i64;
                    self.common.last = self.common.next;
                    self.common.last_gps_time_diffs[self.common.last as usize] = 0;
                    self.common.multi_extreme_counters[self.common.last as usize] = 0;
                } else if multi > 2 {
                    // we switch to another sequence
                    self.common.last = (self.common.last + multi as usize - 2) & 3;
                    self.decompress_field_with(&mut decoder, last_point)?;
                }
            } else {
                multi = decoder.decode_symbol(&mut self.common.gps_time_multi)? as i32;

                if multi == 1 {
                    self.common.last_gps_times[self.common.last as usize].value +=
                        self.ic_gps_time.decompress(
                            &mut decoder,
                            self.common.last_gps_time_diffs[self.common.last as usize],
                            1,
                        )? as i64;
                    self.common.multi_extreme_counters[self.common.last as usize] = 0;
                } else if multi < LASZIP_GPS_TIME_MULTI_UNCHANGED {
                    let gps_time_diff: i32;
                    if multi == 0 {
                        gps_time_diff = self.ic_gps_time.decompress(&mut decoder, 0, 7)?;
                        self.common.multi_extreme_counters[self.common.last as usize] += 1;
                        if self.common.multi_extreme_counters[self.common.last as usize] > 3 {
                            self.common.last_gps_time_diffs[self.common.last as usize] =
                                gps_time_diff;
                            self.common.multi_extreme_counters[self.common.last as usize] = 0;
                        }
                    } else if multi < LASZIP_GPS_TIME_MULTI {
                        // TODO this can be made shorter, the if only changes the context param
                        if multi < 10 {
                            gps_time_diff = self.ic_gps_time.decompress(
                                &mut decoder,
                                multi.wrapping_mul(
                                    self.common.last_gps_time_diffs[self.common.last as usize],
                                ),
                                2,
                            )?;
                        } else {
                            gps_time_diff = self.ic_gps_time.decompress(
                                &mut decoder,
                                multi.wrapping_mul(
                                    self.common.last_gps_time_diffs[self.common.last as usize],
                                ),
                                3,
                            )?;
                        }
                    }
                    // < LASZIP_GPS_TIME_MULTI
                    else if multi == LASZIP_GPS_TIME_MULTI {
                        gps_time_diff = self.ic_gps_time.decompress(
                            &mut decoder,
                            multi.wrapping_mul(
                                self.common.last_gps_time_diffs[self.common.last as usize],
                            ),
                            4,
                        )?;
                        self.common.multi_extreme_counters[self.common.last as usize] += 1;
                        if self.common.multi_extreme_counters[self.common.last as usize] > 3 {
                            self.common.last_gps_time_diffs[self.common.last as usize] =
                                gps_time_diff;
                            self.common.multi_extreme_counters[self.common.last as usize] = 0;
                        }
                    } else {
                        multi = LASZIP_GPS_TIME_MULTI - multi;
                        if multi > LASZIP_GPS_TIME_MULTI_MINUS {
                            gps_time_diff = self.ic_gps_time.decompress(
                                &mut decoder,
                                multi.wrapping_mul(
                                    self.common.last_gps_time_diffs[self.common.last as usize],
                                ),
                                5,
                            )?;
                        } else {
                            gps_time_diff = self.ic_gps_time.decompress(
                                &mut decoder,
                                LASZIP_GPS_TIME_MULTI_MINUS.wrapping_mul(
                                    self.common.last_gps_time_diffs[self.common.last as usize],
                                ),
                                6,
                            )?;
                            self.common.multi_extreme_counters[self.common.last as usize] += 1;
                            if self.common.multi_extreme_counters[self.common.last as usize] > 3 {
                                self.common.last_gps_time_diffs[self.common.last as usize] =
                                    gps_time_diff;
                                self.common.multi_extreme_counters[self.common.last as usize] = 0;
                            }
                        }
                    }
                    self.common.last_gps_times[self.common.last as usize].value +=
                        gps_time_diff as i64;
                } else if multi == LASZIP_GPS_TIME_MULTI_CODE_FULL {
                    self.common.next = (self.common.next + 1) & 3;
                    self.common.last_gps_times[self.common.next as usize].value =
                        self.ic_gps_time.decompress(
                            &mut decoder,
                            (self.common.last_gps_times[self.common.last as usize].value >> 32)
                                as i32,
                            8,
                        )? as i64;
                    self.common.last_gps_times[self.common.next as usize].value <<= 32;
                    self.common.last_gps_times[self.common.next as usize].value |=
                        decoder.read_int()? as i64;
                    self.common.last = self.common.next;
                    self.common.last_gps_time_diffs[self.common.last as usize] = 0;
                    self.common.multi_extreme_counters[self.common.last as usize] = 0;
                } else if multi > LASZIP_GPS_TIME_MULTI_CODE_FULL {
                    self.common.last = (self.common.last + multi as usize
                        - LASZIP_GPS_TIME_MULTI_CODE_FULL as usize)
                        & 3;
                    self.decompress_field_with(&mut decoder, last_point)?;
                }
            }
            last_point.set_gps_time(f64::from_bits(
                self.common.last_gps_times[self.common.last as usize].value as u64,
            ));
            Ok(())
        }
    }

    impl<R: Read> BufferFieldDecompressor<R> for GpsTimeDecompressor {
        fn size_of_field(&self) -> usize {
            std::mem::size_of::<i64>()
        }

        fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()> {
            let mut current_value = GpsTime::default();
            self.init_first_point(src, &mut current_value)?;
            current_value.pack_into(first_point);
            Ok(())
        }

        fn decompress_with(
            &mut self,
            mut decoder: &mut ArithmeticDecoder<R>,
            buf: &mut [u8],
        ) -> std::io::Result<()> {
            let mut current_value = GpsTime::default();
            self.decompress_field_with(&mut decoder, &mut current_value)?;
            current_value.pack_into(buf);
            Ok(())
        }
    }
}
