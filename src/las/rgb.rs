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


use crate::packers::Packable;
use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
use crate::formats::{FieldCompressor, FieldDecompressor};
use crate::encoders::ArithmeticEncoder;
use crate::decoders::ArithmeticDecoder;

use std::io::{Cursor, Write, Read};
use crate::las::utils::flag_diff;
use std::mem::size_of;

use num_traits::{clamp, AsPrimitive};

fn u8_clamp(n: i32) -> u8 {
    if n <= std::u8::MIN as i32 {
        std::u8::MIN
    } else {
        if n >= std::u8::MAX as i32 {
            std::u8::MAX
        } else {
            n as u8
        }
    }
}


#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct RGB {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
}

impl RGB {
    fn color_diff_bits(&self, other: &RGB) -> u32 {
        (flag_diff(other.red, self.red, 0x00FF) as u32) << 0 |
            (flag_diff(other.red, self.red, 0xFF00) as u32) << 1 |
            (flag_diff(other.green, self.green, 0x00FF) as u32) << 2 |
            (flag_diff(other.green, self.green, 0xFF00) as u32) << 3 |
            (flag_diff(other.blue, self.blue, 0x00FF) as u32) << 4 |
            (flag_diff(other.blue, self.blue, 0xFF00) as u32) << 5 |
            ((flag_diff(self.red, self.green, 0x00FF) as u32) |
                (flag_diff(self.red, self.blue, 0x00FF) as u32) |
                (flag_diff(self.red, self.green, 0xFF00) as u32) |
                (flag_diff(self.red, self.blue, 0xFF00) as u32) << 6)
    }
}


impl Packable for RGB {
    type Type = RGB;

    fn unpack(input: &[u8]) -> Self::Type {
        Self {
            red: u16::unpack(&input[0..2]),
            green: u16::unpack(&input[2..4]),
            blue: u16::unpack(&input[4..6]),
        }
    }

    fn pack(value: Self::Type, mut output: &mut [u8]) {
        u16::pack(value.red, &mut output[0..2]);
        u16::pack(value.green, &mut output[2..4]);
        u16::pack(value.blue, &mut output[4..6]);
    }
}

pub struct RGBCompressor {
    have_last: bool,
    last: RGB,

    byte_used: ArithmeticModel,
    rgb_diff_0: ArithmeticModel,
    rgb_diff_1: ArithmeticModel,
    rgb_diff_2: ArithmeticModel,
    rgb_diff_3: ArithmeticModel,
    rgb_diff_4: ArithmeticModel,
    rgb_diff_5: ArithmeticModel,
}

impl RGBCompressor {
    pub fn new() -> Self {
        Self {
            have_last: false,
            last: Default::default(),
            byte_used: ArithmeticModelBuilder::new(128).build(),
            rgb_diff_0: ArithmeticModelBuilder::new(256).build(),
            rgb_diff_1: ArithmeticModelBuilder::new(256).build(),
            rgb_diff_2: ArithmeticModelBuilder::new(256).build(),
            rgb_diff_3: ArithmeticModelBuilder::new(256).build(),
            rgb_diff_4: ArithmeticModelBuilder::new(256).build(),
            rgb_diff_5: ArithmeticModelBuilder::new(256).build(),
        }
    }
}

impl<W: Write> FieldCompressor<W> for RGBCompressor {
    fn size_of_field(&self) -> usize {
        3 * std::mem::size_of::<u16>()
    }

    fn compress_with(&mut self, encoder: &mut ArithmeticEncoder<W>, buf: &[u8]) {
        let this_val = RGB::unpack(&buf);

        if !self.have_last {
            self.have_last = true;
            encoder.out_stream().write_all(&buf).unwrap();
        } else {
            let mut diff_l = 0i32;
            let mut diff_h = 0i32;
            let mut corr = 0i32;

            let sym: u32 = this_val.color_diff_bits(&self.last);
            println!("sym: {}", sym);

            encoder.encode_symbol(&mut self.byte_used, sym);

            // high and low R
            if (sym & (1 << 0)) != 0 {
                diff_l = (this_val.red & 0x00FF) as i32 - (self.last.red & 0x00FF) as i32;
                //println!("{} {} {}", (this_val.red & 0x00FF), (self.last.red & 0x00FF), diff_l);
                println!("diff_l: {}", diff_l);
                encoder.encode_symbol(&mut self.rgb_diff_0, diff_l as u8 as u32);
            }

            if (sym & (1 << 1)) != 0 {
                diff_h = (this_val.red >> 8) as i32 - (self.last.red >> 8) as i32;
                println!("diff_h: {}", diff_h);
                encoder.encode_symbol(&mut self.rgb_diff_1, diff_h as u8 as u32);
            }
            if (sym & (1 << 6)) != 0 {
                if (sym & (1 << 2)) != 0 {
                    corr = (this_val.green & 0x00FF) as i32 - u8_clamp((diff_l + (self.last.green & 0x00FF) as i32)) as i32;
                    encoder.encode_symbol(&mut self.rgb_diff_2, corr as u8 as u32);
                }

                if (sym & (1 << 4)) != 0 {
                    diff_l = (diff_l + (this_val.green & 0x00FF) as i32 - (self.last.green & 0x00FF) as i32) / 2;
                    corr = (this_val.blue & 0x00FF) as i32 - u8_clamp((diff_l + (self.last.blue & 0x00FF) as i32)) as i32;
                    encoder.encode_symbol(&mut self.rgb_diff_4, corr as u8 as u32);
                }

                if (sym & (1 << 3)) != 0 {
                    corr = (this_val.green >> 8) as i32 - u8_clamp((diff_h + (self.last.green >> 8) as i32)) as i32;
                    encoder.encode_symbol(&mut self.rgb_diff_3, corr as u8 as u32);
                }

                if (sym & (1 << 5)) != 0 {
                    diff_h = (diff_h + (this_val.green >> 8) as i32 - (self.last.green >> 8) as i32) / 2;
                    corr = (this_val.blue >> 8) as i32 - u8_clamp((diff_h + (self.last.blue >> 8) as i32)) as i32;
                    encoder.encode_symbol(&mut self.rgb_diff_5, corr as u8 as u32);
                }
            }
        }
        self.last = this_val;
    }
}

pub type RGBDecompressor = RGBCompressor;

impl<R: Read> FieldDecompressor<R> for RGBDecompressor {
    fn size_of_field(&self) -> usize {
        3 * size_of::<u16>()
    }

    fn decompress_with(&mut self, decoder: &mut ArithmeticDecoder<R>, mut buf: &mut [u8]) {
        if !self.have_last {
            decoder.in_stream().read_exact(&mut buf);
            self.last = RGB::unpack(&buf);
            self.have_last = true;
        } else {
            let sym = decoder.decode_symbol(&mut self.byte_used);

            let mut this_val = RGB::default();
            println!("RGB DEFAULT: {:?}", this_val);
            let mut corr = 0u8;
            let mut diff = 0i32;

            if (sym & (1 << 0)) != 0 {
                corr = decoder.decode_symbol(&mut self.rgb_diff_0) as u8;
                this_val.red = corr.wrapping_add((self.last.red & 0x00FF) as u8) as u16;
            } else {
                this_val.red = self.last.red & 0xFF;
            }

            if (sym & (1 << 1)) != 0 {
                corr = decoder.decode_symbol(&mut self.rgb_diff_1) as u8;
                this_val.red |= (corr.wrapping_add((self.last.red >> 8) as u8) as u16) << 8;
            } else {
                this_val.red |= self.last.red & 0xFF00;
            }

            if (sym & (1 << 6)) != 0 {
                diff = (this_val.red & 0x00FF) as i32 - (self.last.red & 0x00FF) as i32;

                if (sym & (1 << 2)) != 0 {
                    corr = decoder.decode_symbol(&mut self.rgb_diff_2) as u8;
                    this_val.green = corr.wrapping_add(u8_clamp((diff + (self.last.green & 0x00FF) as i32)) as u8) as u16;
                } else {
                    this_val.green = self.last.green & 0x00FF;
                }

                if (sym & (1 << 4)) != 0 {
                    corr = decoder.decode_symbol(&mut self.rgb_diff_4) as u8;
                    diff = (diff + (this_val.green & 0x00FF) as i32 - (self.last.green & 0x00FF) as i32) / 2;
                    this_val.blue = (corr.wrapping_add(u8_clamp((diff + (self.last.blue & 0x00FF) as i32)) as u8)) as u16;
                } else {
                    this_val.blue = self.last.blue & 0x00FF;
                }


                diff = (this_val.red >> 8) as i32 - (self.last.red >> 8) as i32;
                if (sym & (1 << 3)) != 0 {
                    corr = decoder.decode_symbol(&mut self.rgb_diff_3) as u8;
                    this_val.green |= (corr.wrapping_add(u8_clamp((diff + (self.last.green >> 8) as i32))) as u16) << 8;
                } else {
                    this_val.green |= self.last.green & 0xFF00;
                }

                if (sym & (1 << 5)) != 0 {
                    corr = decoder.decode_symbol(&mut self.rgb_diff_5) as u8;
                    diff = (diff + (this_val.green >> 8) as i32 - (self.last.green >> 8) as i32) / 2;

                    this_val.blue |= ((corr + (u8_clamp((diff + (self.last.blue >> 8) as i32))) as u8) as u16) << 08;
                } else {
                    this_val.blue |= (self.last.blue & 0xFF00);
                }
            } else {
                this_val.green = this_val.red;
                this_val.blue = this_val.red;
            }
            RGB::pack(this_val, &mut buf);
            self.last = this_val;
        }
    }
}

