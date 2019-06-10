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

use crate::compressors::{IntegerCompressor, IntegerCompressorBuilder};
use crate::decoders::ArithmeticDecoder;
use crate::decompressors::{IntegerDecompressor, IntegerDecompressorBuilder};
use crate::encoders::ArithmeticEncoder;
use crate::formats::{FieldCompressor, FieldDecompressor};
use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
use crate::packers::Packable;
use std::io::{Read, Write};

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

#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct GpsTime {
    pub value: i64,
}

impl Packable for GpsTime {
    type Type = GpsTime;

    fn unpack(input: &[u8]) -> Self::Type {
        let lower = u32::unpack(&input[0..std::mem::size_of::<u32>()]);
        let upper =
            u32::unpack(&input[std::mem::size_of::<u32>()..(2 * std::mem::size_of::<u32>())]);

        GpsTime {
            value: (upper as i64) << 32 | lower as i64,
        }
    }

    fn pack(value: Self::Type, output: &mut [u8]) {
        u32::pack(
            (value.value & 0xFFFFFFFF) as u32,
            &mut output[0..std::mem::size_of::<u32>()],
        );
        u32::pack(
            (value.value >> 32) as u32,
            &mut output[std::mem::size_of::<u32>()..(2 * std::mem::size_of::<u32>())],
        );
    }
}

// Common parts for both a compressor and decompressor go here
struct Common {
    have_last: bool,
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
            have_last: false,
            gps_time_multi: ArithmeticModelBuilder::new(LASZIP_GPS_TIME_MULTI as u32).build(),
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
    compressor_inited: bool,
    common: Common,
}

impl GpsTimeCompressor {
    pub fn new() -> Self {
        Self {
            ic_gps_time: IntegerCompressorBuilder::new().bits(32).contexts(9).build(),
            compressor_inited: false,
            common: Common::new(),
        }
    }
}

impl<W: Write> FieldCompressor<W> for GpsTimeCompressor {
    fn size_of_field(&self) -> usize {
        std::mem::size_of::<i64>()
    }

    fn compress_with(&mut self, mut encoder: &mut ArithmeticEncoder<W>, buf: &[u8]) {
        let this_val = GpsTime::unpack(&buf);

        if !self.compressor_inited {
            self.ic_gps_time.init();
            self.compressor_inited = true;
        }

        if !self.common.have_last {
            self.common.have_last = true;

            self.common.last_gps_times[0] = this_val;

            encoder.out_stream().write_all(&buf).unwrap();
        } else {
            // if last integer different was 0
            if self.common.last_gps_time_diffs[self.common.last] == 0 {
                if this_val.value == self.common.last_gps_times[self.common.last].value {
                    encoder.encode_symbol(&mut self.common.gps_time_0_diff, 0);
                } else {
                    // calculate the difference between the two doubles as an integer
                    let curr_gps_time_diff_64 =
                        this_val.value - self.common.last_gps_times[self.common.last].value;
                    let curr_gps_time_diff_32 = curr_gps_time_diff_64 as i32;

                    if curr_gps_time_diff_64 == curr_gps_time_diff_32 as i64 {
                        // this difference is small enough to be represented with 32 bits
                        encoder.encode_symbol(&mut self.common.gps_time_0_diff, 1);
                        self.ic_gps_time
                            .compress(&mut encoder, 0, curr_gps_time_diff_32, 0);
                        self.common.last_gps_time_diffs[self.common.last] = curr_gps_time_diff_32;
                        self.common.multi_extreme_counters[self.common.last] = 0;
                    } else {
                        // the difference is huge
                        // maybe the double belongs to another time sequence
                        for i in 1..4 {
                            let other_gps_time_diff_64 = this_val.value
                                - self.common.last_gps_times[((self.common.last + 1) & 3)].value;
                            let other_gps_time_diff_32 = other_gps_time_diff_64 as i32;

                            if other_gps_time_diff_64 == other_gps_time_diff_32 as i64 {
                                encoder.encode_symbol(&mut self.common.gps_time_0_diff, i + 2);
                                self.common.last = (self.common.last + i as usize) & 3;
                                return self.compress_with(&mut encoder, &buf);
                            }
                        }
                        // no other sequence found. start new sequence.
                        encoder.encode_symbol(&mut self.common.gps_time_0_diff, 2);
                        self.ic_gps_time.compress(
                            &mut encoder,
                            (self.common.last_gps_times[self.common.last].value >> 32) as i32,
                            (this_val.value >> 32) as i32,
                            8,
                        );

                        encoder.write_int(this_val.value as u32);

                        self.common.next = (self.common.next + 1) & 3;
                        self.common.last = self.common.next;
                        self.common.last_gps_time_diffs[self.common.last] = 0;
                        self.common.multi_extreme_counters[self.common.last] = 0;
                    }
                    self.common.last_gps_times[self.common.last] = this_val;
                }
            } else {
                // the last integer difference was *not* zero
                let curr_gps_time_diff_64 =
                    this_val.value - self.common.last_gps_times[self.common.last].value;
                let curr_gps_time_diff_32 = curr_gps_time_diff_64 as i32;

                // if the current gpstime difference can be represented with 32 bits
                if curr_gps_time_diff_64 == curr_gps_time_diff_32 as i64 {
                    // compute multiplier between current and last integer difference
                    let multi_f = curr_gps_time_diff_32 as f32
                        / self.common.last_gps_time_diffs[self.common.last] as f32;
                    let multi = i32_quantize(multi_f);

                    // compress the residual curr_gps_time_diff in dependance on the multiplier
                    if multi == 1 {
                        // this is the case we assume we get most often for regular spaced pulses
                        encoder.encode_symbol(&mut self.common.gps_time_multi, 1);
                        self.ic_gps_time.compress(
                            &mut encoder,
                            self.common.last_gps_time_diffs[self.common.last],
                            curr_gps_time_diff_32,
                            1,
                        );
                        self.common.multi_extreme_counters[self.common.last] = 0;
                    } else if multi > 0 {
                        if multi < LASZIP_GPS_TIME_MULTI {
                            // positive multipliers up to LASZIP_GPSTIME_MULTI are compressed directly
                            encoder.encode_symbol(&mut self.common.gps_time_multi, multi as u32);
                            let context = if multi < 10 { 2u32 } else { 3u32 };
                            self.ic_gps_time.compress(
                                &mut encoder,
                                multi * self.common.last_gps_time_diffs[self.common.last],
                                curr_gps_time_diff_32,
                                context,
                            );
                        } else {
                            encoder.encode_symbol(
                                &mut self.common.gps_time_multi,
                                LASZIP_GPS_TIME_MULTI as u32,
                            );
                            self.ic_gps_time.compress(
                                &mut encoder,
                                LASZIP_GPS_TIME_MULTI
                                    * self.common.last_gps_time_diffs[self.common.last],
                                curr_gps_time_diff_32,
                                3,
                            );
                        }
                    } else if multi < 0 {
                        if multi > LASZIP_GPS_TIME_MULTI_MINUS {
                            // negative multipliers larger than LASZIP_GPSTIME_MULTI_MINUS are compressed directly
                            encoder.encode_symbol(
                                &mut self.common.gps_time_multi,
                                (LASZIP_GPS_TIME_MULTI - multi) as u32,
                            );
                            self.ic_gps_time.compress(
                                &mut encoder,
                                multi * self.common.last_gps_time_diffs[self.common.last],
                                curr_gps_time_diff_32,
                                5,
                            );
                        } else {
                            encoder.encode_symbol(
                                &mut self.common.gps_time_multi,
                                (LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS) as u32,
                            );
                            self.ic_gps_time.compress(
                                &mut encoder,
                                LASZIP_GPS_TIME_MULTI_MINUS
                                    * self.common.last_gps_time_diffs[self.common.last],
                                curr_gps_time_diff_32,
                                6,
                            );
                            self.common.multi_extreme_counters[self.common.last] += 1;
                            if self.common.multi_extreme_counters[self.common.last] > 3 {
                                self.common.last_gps_time_diffs[self.common.last] =
                                    curr_gps_time_diff_32;
                                self.common.multi_extreme_counters[self.common.last] = 0;
                            }
                        }
                    } else {
                        encoder.encode_symbol(&mut self.common.gps_time_multi, 0);
                        self.ic_gps_time
                            .compress(&mut encoder, 7, curr_gps_time_diff_32, 7);
                        self.common.multi_extreme_counters[self.common.last] += 1;
                        if self.common.multi_extreme_counters[self.common.last] > 3 {
                            self.common.last_gps_time_diffs[self.common.last] =
                                curr_gps_time_diff_32;
                            self.common.multi_extreme_counters[self.common.last] = 0;
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
                            );
                            self.common.last = (self.common.last + i) & 3;
                            return self.compress_with(&mut encoder, &buf);
                        }
                    }

                    // no other sequence found start a new one
                    encoder.encode_symbol(
                        &mut self.common.gps_time_multi,
                        LASZIP_GPS_TIME_MULTI_CODE_FULL as u32,
                    );
                    self.ic_gps_time.compress(
                        &mut encoder,
                        (self.common.last_gps_times[self.common.last as usize].value >> 32) as i32,
                        (this_val.value >> 32) as i32,
                        8,
                    );

                    encoder.write_int(this_val.value as u32);
                    self.common.next = (self.common.next + 1) & 3;
                    self.common.last = self.common.next;
                    self.common.last_gps_time_diffs[self.common.last] = 0;
                    self.common.multi_extreme_counters[self.common.last] = 0;
                }
                self.common.last_gps_times[self.common.last] = this_val;
            }
        }
    } //fn
}

pub struct GpsTimeDecompressor {
    common: Common,
    decompressor_inited: bool,
    ic_gps_time: IntegerDecompressor,
}

impl GpsTimeDecompressor {
    pub fn new() -> Self {
        Self {
            common: Common::new(),
            decompressor_inited: false,
            ic_gps_time: IntegerDecompressorBuilder::new()
                .bits(32)
                .contexts(9)
                .build(),
        }
    }
}

impl<R: Read> FieldDecompressor<R> for GpsTimeDecompressor {
    fn size_of_field(&self) -> usize {
        std::mem::size_of::<i64>()
    }

    fn decompress_with(&mut self, mut decoder: &mut ArithmeticDecoder<R>, mut buf: &mut [u8]) {
        if !self.decompressor_inited {
            self.ic_gps_time.init();
            self.decompressor_inited = true;
        }

        if !self.common.have_last {
            decoder.in_stream().read_exact(&mut buf).unwrap();
            self.common.last_gps_times[0] = GpsTime::unpack(&buf);
            self.common.have_last = true;
        } else {
            let mut multi: i32;

            if self.common.last_gps_time_diffs[self.common.last as usize] == 0 {
                // it the last integer difference was zero
                multi = decoder.decode_symbol(&mut self.common.gps_time_0_diff) as i32;

                if multi == 1 {
                    // the difference can be represented with 32 bits
                    self.common.last_gps_time_diffs[self.common.last as usize] =
                        self.ic_gps_time.decompress(&mut decoder, 0, 0);
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
                        ) as i64;
                    self.common.last_gps_times[self.common.next as usize].value <<= 32;
                    self.common.last_gps_times[self.common.next as usize].value |=
                        decoder.read_int() as i64;
                    self.common.last = self.common.next;
                    self.common.last_gps_time_diffs[self.common.last as usize] = 0;
                    self.common.multi_extreme_counters[self.common.last as usize] = 0;
                } else if multi > 2 {
                    // we switch to another sequence
                    self.common.last = (self.common.last + multi as usize - 2) & 3;
                    self.decompress_with(&mut decoder, buf);
                }
            } else {
                multi = decoder.decode_symbol(&mut self.common.gps_time_multi) as i32;

                if multi == 1 {
                    self.common.last_gps_times[self.common.last as usize].value +=
                        self.ic_gps_time.decompress(
                            &mut decoder,
                            self.common.last_gps_time_diffs[self.common.last as usize],
                            1,
                        ) as i64;
                    self.common.multi_extreme_counters[self.common.last as usize] = 0;
                } else if multi < LASZIP_GPS_TIME_MULTI_UNCHANGED {
                    let gps_time_diff: i32;
                    if multi == 0 {
                        gps_time_diff = self.ic_gps_time.decompress(&mut decoder, 0, 7);
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
                                multi * self.common.last_gps_time_diffs[self.common.last as usize],
                                2,
                            );
                        } else {
                            gps_time_diff = self.ic_gps_time.decompress(
                                &mut decoder,
                                multi * self.common.last_gps_time_diffs[self.common.last as usize],
                                3,
                            );
                        }
                    }
                    // < LASZIP_GPS_TIME_MULTI
                    else if multi == LASZIP_GPS_TIME_MULTI {
                        gps_time_diff = self.ic_gps_time.decompress(
                            &mut decoder,
                            multi * self.common.last_gps_time_diffs[self.common.last as usize],
                            4,
                        );
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
                                multi * self.common.last_gps_time_diffs[self.common.last as usize],
                                5,
                            );
                        } else {
                            gps_time_diff = self.ic_gps_time.decompress(
                                &mut decoder,
                                LASZIP_GPS_TIME_MULTI_MINUS
                                    * self.common.last_gps_time_diffs[self.common.last as usize],
                                6,
                            );
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
                        ) as i64;
                    self.common.last_gps_times[self.common.next as usize].value <<= 32;
                    self.common.last_gps_times[self.common.next as usize].value |=
                        decoder.read_int() as i64;
                    self.common.last = self.common.next;
                    self.common.last_gps_time_diffs[self.common.last as usize] = 0;
                    self.common.multi_extreme_counters[self.common.last as usize] = 0;
                } else if multi > LASZIP_GPS_TIME_MULTI_CODE_FULL {
                    self.common.last = (self.common.last + multi as usize
                        - LASZIP_GPS_TIME_MULTI_CODE_FULL as usize)
                        & 3;
                    self.decompress_with(&mut decoder, buf);
                }
            }
        }
        GpsTime::pack(self.common.last_gps_times[self.common.last as usize], buf);
    }
}
