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
//! Defines the different version of compressors and decompressors for the GpsTime

use std::io::Read;
use std::ops::{Add, AddAssign};

use byteorder::{LittleEndian, ReadBytesExt};

use crate::packers::Packable;

const LASZIP_GPS_TIME_MULTI: i32 = 500;
const LASZIP_GPS_TIME_MULTI_MINUS: i32 = -10;
const LASZIP_GPS_TIME_MULTI_UNCHANGED: i32 =
    (LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 1);
const LASZIP_GPS_TIME_MULTI_CODE_FULL: i32 =
    (LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 2);

const LASZIP_GPS_TIME_MULTI_TOTAL: i32 = (LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 6);

pub trait LasGpsTime {
    fn gps_time(&self) -> f64;
    fn set_gps_time(&mut self, new_value: f64);

    fn read_from<R: Read>(&mut self, src: &mut R) -> std::io::Result<()> {
        self.set_gps_time(src.read_f64::<LittleEndian>()?);
        Ok(())
    }
}

/// Struct to store GpsTime
///
/// As the value (f64 as per LAS spec) needs to be reinterpreted
/// (not simply converted with 'as') to i64 (or u64)
/// during compression / decompression this struct provides a convenient wrapper
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct GpsTime {
    pub value: i64,
}

impl From<f64> for GpsTime {
    fn from(v: f64) -> Self {
        Self {
            value: v.to_bits() as i64,
        }
    }
}

impl From<GpsTime> for i64 {
    fn from(gps: GpsTime) -> Self {
        gps.value
    }
}

impl From<i64> for GpsTime {
    fn from(v: i64) -> Self {
        Self { value: v }
    }
}

impl Add<f64> for GpsTime {
    type Output = Self;

    fn add(self, rhs: f64) -> Self::Output {
        Self::from(self.value + rhs.to_bits() as i64)
    }
}

impl Add<i64> for GpsTime {
    type Output = Self;

    fn add(self, rhs: i64) -> Self::Output {
        Self::from(self.value + rhs)
    }
}

impl AddAssign<f64> for GpsTime {
    fn add_assign(&mut self, rhs: f64) {
        self.value += rhs.to_bits() as i64;
    }
}

impl AddAssign<i64> for GpsTime {
    fn add_assign(&mut self, rhs: i64) {
        self.value += rhs;
    }
}

impl From<GpsTime> for f64 {
    fn from(gps: GpsTime) -> Self {
        {
            f64::from_bits(gps.value as u64)
        }
    }
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
    fn unpack_from(input: &[u8]) -> Self {
        if input.len() < std::mem::size_of::<i64>() {
            panic!(
                "GpsTime::unpack_from expected a buffer of {} bytes",
                std::mem::size_of::<i64>()
            );
        }
        let lower = u32::unpack_from(&input[0..std::mem::size_of::<u32>()]);
        let upper =
            u32::unpack_from(&input[std::mem::size_of::<u32>()..(2 * std::mem::size_of::<u32>())]);

        GpsTime {
            value: i64::from(upper) << 32 | i64::from(lower),
        }
    }

    fn pack_into(&self, output: &mut [u8]) {
        u32::pack_into(
            &((self.value & 0xFFFF_FFFF) as u32),
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

    use num_traits::clamp;

    use crate::compressors::{
        IntegerCompressor, IntegerCompressorBuilder, DEFAULT_COMPRESS_CONTEXTS,
    };
    use crate::decoders::ArithmeticDecoder;
    use crate::decompressors::{IntegerDecompressor, IntegerDecompressorBuilder};
    use crate::encoders::ArithmeticEncoder;
    use crate::las::gps::LasGpsTime;
    use crate::las::utils::read_and_unpack;
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;
    use crate::record::{FieldCompressor, FieldDecompressor};

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

    impl Default for LasGpsTimeDecompressor {
        fn default() -> Self {
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

    pub struct LasGpsTimeCompressor {
        last_gps: i64,
        gps_time_multi_model: ArithmeticModel,
        gps_time_0_diff_model: ArithmeticModel,
        ic_gps_time: IntegerCompressor,
        multi_extreme_counter: i32,
        last_gps_time_diff: i32,
    }

    impl Default for LasGpsTimeCompressor {
        fn default() -> Self {
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

    impl<W: Write> FieldCompressor<W> for LasGpsTimeCompressor {
        fn size_of_field(&self) -> usize {
            std::mem::size_of::<f64>()
        }

        fn compress_first(&mut self, dst: &mut W, buf: &[u8]) -> std::io::Result<()> {
            self.last_gps = GpsTime::unpack_from(buf).into();
            dst.write_all(buf)
        }

        fn compress_with(
            &mut self,
            mut encoder: &mut ArithmeticEncoder<W>,
            buf: &[u8],
        ) -> std::io::Result<()> {
            let current_point = GpsTime::unpack_from(buf);
            let current_gps_time_value = current_point.gps_time().to_bits() as i64;

            if self.last_gps_time_diff == 0 {
                if current_gps_time_value == self.last_gps {
                    encoder.encode_symbol(&mut self.gps_time_0_diff_model, 0)?;
                } else {
                    let current_gps_time_diff_64 = current_gps_time_value - self.last_gps;
                    let current_gps_time_diff_32 = current_gps_time_diff_64 as i32;

                    if current_gps_time_diff_64 == i64::from(current_gps_time_diff_32) {
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

                    if current_gps_time_diff_64 == i64::from(current_gps_time_diff_32) {
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

    impl<R: Read> FieldDecompressor<R> for LasGpsTimeDecompressor {
        fn size_of_field(&self) -> usize {
            std::mem::size_of::<f64>()
        }

        fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()> {
            self.last_gps = i64::from(read_and_unpack::<_, GpsTime>(src, first_point)?);
            Ok(())
        }

        fn decompress_with(
            &mut self,
            mut decoder: &mut ArithmeticDecoder<R>,
            buf: &mut [u8],
        ) -> std::io::Result<()> {
            let multi;
            if self.last_gps_time_diff == 0 {
                multi = decoder.decode_symbol(&mut self.gps_time_0_diff_model)?;
                if multi == 1 {
                    // the difference can be represented with 32 bits
                    self.last_gps_time_diff = self.ic_gps_time.decompress(&mut decoder, 0, 0)?;
                    self.last_gps += i64::from(self.last_gps_time_diff);
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
                    self.last_gps += i64::from(gps_time_diff);
                } else if multi < LASZIP_GPS_TIME_MULTI_MAX - 1 {
                    self.last_gps = decoder.read_int_64()? as i64;
                }
            }
            GpsTime::from(self.last_gps).pack_into(buf);
            Ok(())
        }
    }
}

pub mod v2 {
    use std::io::{Read, Write};

    use crate::compressors::{IntegerCompressor, IntegerCompressorBuilder};
    use crate::decoders::ArithmeticDecoder;
    use crate::decompressors::{IntegerDecompressor, IntegerDecompressorBuilder};
    use crate::encoders::ArithmeticEncoder;
    use crate::las::utils::{i32_quantize, read_and_unpack};
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;
    use crate::record::{FieldCompressor, FieldDecompressor};

    use super::{
        GpsTime, LASZIP_GPS_TIME_MULTI, LASZIP_GPS_TIME_MULTI_CODE_FULL,
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

    impl Default for GpsTimeCompressor {
        fn default() -> Self {
            Self {
                ic_gps_time: IntegerCompressorBuilder::new()
                    .bits(32)
                    .contexts(9)
                    .build_initialized(),
                common: Common::new(),
            }
        }
    }

    impl<W: Write> FieldCompressor<W> for GpsTimeCompressor {
        fn size_of_field(&self) -> usize {
            std::mem::size_of::<i64>()
        }

        fn compress_first(&mut self, dst: &mut W, buf: &[u8]) -> std::io::Result<()> {
            self.common.last_gps_times[0] = GpsTime::unpack_from(buf);
            dst.write_all(buf)
        }

        fn compress_with(
            &mut self,
            mut encoder: &mut ArithmeticEncoder<W>,
            buf: &[u8],
        ) -> std::io::Result<()> {
            let this_val = GpsTime::unpack_from(&buf);
            assert!(self.common.last < 4);
            unsafe {
                if *self
                    .common
                    .last_gps_time_diffs
                    .get_unchecked(self.common.last)
                    == 0
                {
                    if this_val.value
                        == self
                            .common
                            .last_gps_times
                            .get_unchecked(self.common.last)
                            .value
                    {
                        encoder.encode_symbol(&mut self.common.gps_time_0_diff, 0)?;
                    } else {
                        // calculate the difference between the two doubles as an integer
                        let curr_gps_time_diff_64 = this_val.value
                            - self
                                .common
                                .last_gps_times
                                .get_unchecked(self.common.last)
                                .value;
                        let curr_gps_time_diff_32 = curr_gps_time_diff_64 as i32;

                        if curr_gps_time_diff_64 == i64::from(curr_gps_time_diff_32) {
                            // this difference is small enough to be represented with 32 bits
                            encoder.encode_symbol(&mut self.common.gps_time_0_diff, 1)?;
                            self.ic_gps_time
                                .compress(&mut encoder, 0, curr_gps_time_diff_32, 0)?;
                            *self
                                .common
                                .last_gps_time_diffs
                                .get_unchecked_mut(self.common.last) = curr_gps_time_diff_32;
                            *self
                                .common
                                .multi_extreme_counters
                                .get_unchecked_mut(self.common.last) = 0;
                        } else {
                            // the difference is huge
                            // maybe the double belongs to another time sequence
                            for i in 1..4 {
                                let other_gps_time_diff_64 = this_val.value
                                    - self
                                        .common
                                        .last_gps_times
                                        .get_unchecked((self.common.last + i) & 3)
                                        .value;
                                let other_gps_time_diff_32 = other_gps_time_diff_64 as i32;

                                if other_gps_time_diff_64 == i64::from(other_gps_time_diff_32) {
                                    // it belongs to another sequence
                                    encoder.encode_symbol(
                                        &mut self.common.gps_time_0_diff,
                                        (i + 2) as u32,
                                    )?;
                                    self.common.last = (self.common.last + i) & 3;
                                    return self.compress_with(&mut encoder, buf);
                                }
                            }
                            // no other sequence found. start new sequence.
                            encoder.encode_symbol(&mut self.common.gps_time_0_diff, 2)?;
                            self.ic_gps_time.compress(
                                &mut encoder,
                                (self
                                    .common
                                    .last_gps_times
                                    .get_unchecked(self.common.last)
                                    .value
                                    >> 32) as i32,
                                (this_val.value >> 32) as i32,
                                8,
                            )?;

                            encoder.write_int(this_val.value as u32)?;

                            self.common.next = (self.common.next + 1) & 3;
                            self.common.last = self.common.next;
                            *self
                                .common
                                .last_gps_time_diffs
                                .get_unchecked_mut(self.common.last) = 0;
                            *self
                                .common
                                .multi_extreme_counters
                                .get_unchecked_mut(self.common.last) = 0;
                        }
                        *self
                            .common
                            .last_gps_times
                            .get_unchecked_mut(self.common.last) = this_val;
                    }
                } else {
                    //the last integer difference was *not* zero
                    if this_val.value
                        == self
                            .common
                            .last_gps_times
                            .get_unchecked(self.common.last)
                            .value
                    {
                        // if the doubles have not changed use a special symbol
                        encoder.encode_symbol(
                            &mut self.common.gps_time_multi,
                            LASZIP_GPS_TIME_MULTI_UNCHANGED as u32,
                        )?;
                    } else {
                        // the last integer difference was *not* zero
                        let curr_gps_time_diff_64 = this_val.value
                            - self
                                .common
                                .last_gps_times
                                .get_unchecked(self.common.last)
                                .value;
                        let curr_gps_time_diff_32 = curr_gps_time_diff_64 as i32;

                        // if the current gps time difference can be represented with 32 bits
                        if curr_gps_time_diff_64 == i64::from(curr_gps_time_diff_32) {
                            // compute multiplier between current and last integer difference
                            let multi_f = curr_gps_time_diff_32 as f32
                                / *self
                                    .common
                                    .last_gps_time_diffs
                                    .get_unchecked(self.common.last)
                                    as f32;
                            let multi = i32_quantize(multi_f);

                            // compress the residual curr_gps_time_diff in dependance on the multiplier
                            if multi == 1 {
                                // this is the case we assume we get most often for regular spaced pulses
                                encoder.encode_symbol(&mut self.common.gps_time_multi, 1)?;
                                self.ic_gps_time.compress(
                                    &mut encoder,
                                    *self
                                        .common
                                        .last_gps_time_diffs
                                        .get_unchecked(self.common.last),
                                    curr_gps_time_diff_32,
                                    1,
                                )?;
                                *self
                                    .common
                                    .multi_extreme_counters
                                    .get_unchecked_mut(self.common.last) = 0;
                            } else if multi > 0 {
                                if multi < LASZIP_GPS_TIME_MULTI {
                                    // positive multipliers up to LASZIP_GPSTIME_MULTI are compressed directly
                                    encoder.encode_symbol(
                                        &mut self.common.gps_time_multi,
                                        multi as u32,
                                    )?;
                                    let context = if multi < 10 { 2u32 } else { 3u32 };
                                    self.ic_gps_time.compress(
                                        &mut encoder,
                                        multi.wrapping_mul(
                                            *self
                                                .common
                                                .last_gps_time_diffs
                                                .get_unchecked(self.common.last),
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
                                            *self
                                                .common
                                                .last_gps_time_diffs
                                                .get_unchecked(self.common.last),
                                        ),
                                        curr_gps_time_diff_32,
                                        4,
                                    )?;

                                    let multi_extreme_counter =
                                        &mut self.common.multi_extreme_counters[self.common.last];
                                    *multi_extreme_counter += 1;
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
                                        (LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS)
                                            as u32,
                                    )?;
                                    self.ic_gps_time.compress(
                                        &mut encoder,
                                        LASZIP_GPS_TIME_MULTI_MINUS.wrapping_mul(
                                            self.common.last_gps_time_diffs[self.common.last],
                                        ),
                                        curr_gps_time_diff_32,
                                        6,
                                    )?;
                                    let multi_extreme_counter =
                                        &mut self.common.multi_extreme_counters[self.common.last];
                                    *multi_extreme_counter += 1;
                                    if *multi_extreme_counter > 3 {
                                        self.common.last_gps_time_diffs[self.common.last] =
                                            curr_gps_time_diff_32;
                                        *multi_extreme_counter = 0;
                                    }
                                }
                            } else {
                                encoder.encode_symbol(&mut self.common.gps_time_multi, 0)?;
                                self.ic_gps_time.compress(
                                    &mut encoder,
                                    0,
                                    curr_gps_time_diff_32,
                                    7,
                                )?;
                                let multi_extreme_counter =
                                    &mut self.common.multi_extreme_counters[self.common.last];
                                *multi_extreme_counter += 1;
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
                                    - self.common.last_gps_times[((self.common.last + i) & 3)]
                                        .value;
                                let other_gps_time_diff_32 = other_gps_time_diff_64 as i32;

                                if other_gps_time_diff_64 == i64::from(other_gps_time_diff_32) {
                                    // it belongs to this sequence
                                    encoder.encode_symbol(
                                        &mut self.common.gps_time_multi,
                                        (LASZIP_GPS_TIME_MULTI_CODE_FULL + i as i32) as u32,
                                    )?;
                                    self.common.last = (self.common.last + i) & 3;
                                    return self.compress_with(&mut encoder, buf);
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
                            *self
                                .common
                                .last_gps_time_diffs
                                .get_unchecked_mut(self.common.last) = 0;
                            *self
                                .common
                                .multi_extreme_counters
                                .get_unchecked_mut(self.common.last) = 0;
                        }
                        *self
                            .common
                            .last_gps_times
                            .get_unchecked_mut(self.common.last) = this_val;
                    }
                }
                Ok(())
            }
        }
    }

    pub struct GpsTimeDecompressor {
        common: Common,
        ic_gps_time: IntegerDecompressor,
    }

    impl Default for GpsTimeDecompressor {
        fn default() -> Self {
            Self {
                common: Common::new(),
                ic_gps_time: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(9)
                    .build_initialized(),
            }
        }
    }

    impl<R: Read> FieldDecompressor<R> for GpsTimeDecompressor {
        fn size_of_field(&self) -> usize {
            std::mem::size_of::<i64>()
        }

        fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()> {
            unsafe {
                *self.common.last_gps_times.get_unchecked_mut(0) =
                    read_and_unpack::<_, GpsTime>(src, first_point)?;
            }
            Ok(())
        }

        fn decompress_with(
            &mut self,
            mut decoder: &mut ArithmeticDecoder<R>,
            buf: &mut [u8],
        ) -> std::io::Result<()> {
            let mut multi: i32;
            assert!(self.common.last < 4);
            unsafe {
                if *self
                    .common
                    .last_gps_time_diffs
                    .get_unchecked(self.common.last)
                    == 0
                {
                    // it the last integer difference was zero
                    multi = decoder.decode_symbol(&mut self.common.gps_time_0_diff)? as i32;

                    if multi == 1 {
                        // the difference can be represented with 32 bits
                        *self
                            .common
                            .last_gps_time_diffs
                            .get_unchecked_mut(self.common.last) =
                            self.ic_gps_time.decompress(&mut decoder, 0, 0)?;
                        self.common
                            .last_gps_times
                            .get_unchecked_mut(self.common.last)
                            .value += i64::from(*self
                            .common
                            .last_gps_time_diffs
                            .get_unchecked(self.common.last));
                        *self
                            .common
                            .multi_extreme_counters
                            .get_unchecked_mut(self.common.last) = 0;
                    } else if multi == 2 {
                        // the difference is huge
                        self.common.next = (self.common.next + 1) & 3;
                        self.common
                            .last_gps_times
                            .get_unchecked_mut(self.common.next)
                            .value = i64::from(self.ic_gps_time.decompress(
                            &mut decoder,
                            (self
                                .common
                                .last_gps_times
                                .get_unchecked(self.common.last)
                                .value
                                >> 32) as i32,
                            8,
                        )?);
                        self.common
                            .last_gps_times
                            .get_unchecked_mut(self.common.next)
                            .value <<= 32;
                        self.common
                            .last_gps_times
                            .get_unchecked_mut(self.common.next)
                            .value |= i64::from(decoder.read_int()?);

                        self.common.last = self.common.next;
                        *self
                            .common
                            .last_gps_time_diffs
                            .get_unchecked_mut(self.common.last) = 0;
                        *self
                            .common
                            .multi_extreme_counters
                            .get_unchecked_mut(self.common.last) = 0;
                    } else if multi > 2 {
                        // we switch to another sequence
                        self.common.last = (self.common.last + multi as usize - 2) & 3;
                        self.decompress_with(&mut decoder, buf)?;
                    }
                } else {
                    multi = decoder.decode_symbol(&mut self.common.gps_time_multi)? as i32;

                    if multi == 1 {
                        self.common
                            .last_gps_times
                            .get_unchecked_mut(self.common.last)
                            .value += i64::from(self.ic_gps_time.decompress(
                            &mut decoder,
                            *self
                                .common
                                .last_gps_time_diffs
                                .get_unchecked(self.common.last as usize),
                            1,
                        )?);
                        *self
                            .common
                            .multi_extreme_counters
                            .get_unchecked_mut(self.common.last) = 0;
                    } else if multi < LASZIP_GPS_TIME_MULTI_UNCHANGED {
                        let gps_time_diff: i32;
                        if multi == 0 {
                            gps_time_diff = self.ic_gps_time.decompress(&mut decoder, 0, 7)?;
                            *self
                                .common
                                .multi_extreme_counters
                                .get_unchecked_mut(self.common.last) += 1;
                            if *self
                                .common
                                .multi_extreme_counters
                                .get_unchecked(self.common.last as usize)
                                > 3
                            {
                                *self
                                    .common
                                    .last_gps_time_diffs
                                    .get_unchecked_mut(self.common.last) = gps_time_diff;
                                *self
                                    .common
                                    .multi_extreme_counters
                                    .get_unchecked_mut(self.common.last) = 0;
                            }
                        } else if multi < LASZIP_GPS_TIME_MULTI {
                            // TODO this can be made shorter, the if only changes the context param
                            if multi < 10 {
                                gps_time_diff = self.ic_gps_time.decompress(
                                    &mut decoder,
                                    multi.wrapping_mul(
                                        *self
                                            .common
                                            .last_gps_time_diffs
                                            .get_unchecked(self.common.last),
                                    ),
                                    2,
                                )?;
                            } else {
                                gps_time_diff = self.ic_gps_time.decompress(
                                    &mut decoder,
                                    multi.wrapping_mul(
                                        *self
                                            .common
                                            .last_gps_time_diffs
                                            .get_unchecked(self.common.last),
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
                                    *self
                                        .common
                                        .last_gps_time_diffs
                                        .get_unchecked(self.common.last),
                                ),
                                4,
                            )?;
                            *self
                                .common
                                .multi_extreme_counters
                                .get_unchecked_mut(self.common.last) += 1;
                            if *self
                                .common
                                .multi_extreme_counters
                                .get_unchecked_mut(self.common.last)
                                > 3
                            {
                                *self
                                    .common
                                    .last_gps_time_diffs
                                    .get_unchecked_mut(self.common.last) = gps_time_diff;
                                *self
                                    .common
                                    .multi_extreme_counters
                                    .get_unchecked_mut(self.common.last) = 0;
                            }
                        } else {
                            multi = LASZIP_GPS_TIME_MULTI - multi;
                            if multi > LASZIP_GPS_TIME_MULTI_MINUS {
                                gps_time_diff = self.ic_gps_time.decompress(
                                    &mut decoder,
                                    multi.wrapping_mul(
                                        *self
                                            .common
                                            .last_gps_time_diffs
                                            .get_unchecked(self.common.last),
                                    ),
                                    5,
                                )?;
                            } else {
                                gps_time_diff = self.ic_gps_time.decompress(
                                    &mut decoder,
                                    LASZIP_GPS_TIME_MULTI_MINUS.wrapping_mul(
                                        *self
                                            .common
                                            .last_gps_time_diffs
                                            .get_unchecked(self.common.last),
                                    ),
                                    6,
                                )?;
                                *self
                                    .common
                                    .multi_extreme_counters
                                    .get_unchecked_mut(self.common.last) += 1;
                                if *self
                                    .common
                                    .multi_extreme_counters
                                    .get_unchecked_mut(self.common.last)
                                    > 3
                                {
                                    *self
                                        .common
                                        .last_gps_time_diffs
                                        .get_unchecked_mut(self.common.last) = gps_time_diff;
                                    *self
                                        .common
                                        .multi_extreme_counters
                                        .get_unchecked_mut(self.common.last) = 0;
                                }
                            }
                        }
                        self.common
                            .last_gps_times
                            .get_unchecked_mut(self.common.last)
                            .value += i64::from(gps_time_diff);
                    } else if multi == LASZIP_GPS_TIME_MULTI_CODE_FULL {
                        self.common.next = (self.common.next + 1) & 3;
                        self.common
                            .last_gps_times
                            .get_unchecked_mut(self.common.next)
                            .value = i64::from(self.ic_gps_time.decompress(
                            &mut decoder,
                            (self
                                .common
                                .last_gps_times
                                .get_unchecked(self.common.last)
                                .value
                                >> 32) as i32,
                            8,
                        )?);
                        self.common
                            .last_gps_times
                            .get_unchecked_mut(self.common.next)
                            .value <<= 32;
                        self.common
                            .last_gps_times
                            .get_unchecked_mut(self.common.next)
                            .value |= i64::from(decoder.read_int()?);
                        self.common.last = self.common.next;
                        *self
                            .common
                            .last_gps_time_diffs
                            .get_unchecked_mut(self.common.last) = 0;
                        *self
                            .common
                            .multi_extreme_counters
                            .get_unchecked_mut(self.common.last) = 0;
                    } else if multi > LASZIP_GPS_TIME_MULTI_CODE_FULL {
                        self.common.last = (self.common.last + multi as usize
                            - LASZIP_GPS_TIME_MULTI_CODE_FULL as usize)
                            & 3;
                        self.decompress_with(&mut decoder, buf)?;
                    }
                }
                self.common
                    .last_gps_times
                    .get_unchecked(self.common.last)
                    .pack_into(buf);
                Ok(())
            }
        }
    }
}
