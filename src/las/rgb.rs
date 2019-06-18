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

use num_traits::clamp;

use crate::las::utils::flag_diff;
use crate::packers::Packable;

fn u8_clamp(n: i32) -> u8 {
    clamp(n, i32::from(std::u8::MIN), i32::from(std::u8::MAX)) as u8
}

#[inline(always)]
fn lower_byte(n: u16) -> u8 {
    (n & 0x00FF) as u8
}

#[inline(always)]
fn upper_byte(n: u16) -> u8 {
    (n >> 8) as u8
}

#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct RGB {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
}

impl RGB {
    fn color_diff_bits(&self, other: &RGB) -> u8 {
        (flag_diff(other.red, self.red, 0x00FF) as u8) << 0
            | (flag_diff(other.red, self.red, 0xFF00) as u8) << 1
            | (flag_diff(other.green, self.green, 0x00FF) as u8) << 2
            | (flag_diff(other.green, self.green, 0xFF00) as u8) << 3
            | (flag_diff(other.blue, self.blue, 0x00FF) as u8) << 4
            | (flag_diff(other.blue, self.blue, 0xFF00) as u8) << 5
            | ((flag_diff(self.red, self.green, 0x00FF) as u8)
                | (flag_diff(self.red, self.blue, 0x00FF) as u8)
                | (flag_diff(self.red, self.green, 0xFF00) as u8)
                | (flag_diff(self.red, self.blue, 0xFF00) as u8))
                << 6
    }
}

struct ColorDiff(u8);

impl ColorDiff {
    fn new(v: u8) -> Self {
        Self { 0: v }
    }
    fn lower_red_byte_changed(&self) -> bool {
        self.0 & (1 << 0) != 0
    }

    fn upper_red_byte_changed(&self) -> bool {
        self.0 & (1 << 1) != 0
    }

    fn lower_green_byte_changed(&self) -> bool {
        self.0 & (1 << 2) != 0
    }

    fn upper_green_byte_changed(&self) -> bool {
        self.0 & (1 << 3) != 0
    }

    fn lower_blue_byte_changed(&self) -> bool {
        self.0 & (1 << 4) != 0
    }

    fn upper_blue_byte_changed(&self) -> bool {
        self.0 & (1 << 5) != 0
    }
}

impl Packable for RGB {
    type Type = RGB;

    fn unpack_from(input: &[u8]) -> Self::Type {
        Self {
            red: u16::unpack_from(&input[0..2]),
            green: u16::unpack_from(&input[2..4]),
            blue: u16::unpack_from(&input[4..6]),
        }
    }

    fn pack_into(&self, output: &mut [u8]) {
        u16::pack_into(&self.red, &mut output[0..2]);
        u16::pack_into(&self.green, &mut output[2..4]);
        u16::pack_into(&self.blue, &mut output[4..6]);
    }
}

pub mod v1 {
    use std::io::{Read, Write};
    use std::mem::size_of;

    use crate::compressors::{IntegerCompressor, IntegerCompressorBuilder};
    use crate::decoders::ArithmeticDecoder;
    use crate::decompressors::{IntegerDecompressor, IntegerDecompressorBuilder};
    use crate::encoders::ArithmeticEncoder;
    use crate::record::{FieldCompressor, FieldDecompressor};
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;

    use super::{lower_byte, upper_byte, RGB, ColorDiff};


    pub struct RGBDecompressor {
        last: RGB,
        have_last: bool,
        byte_used_model: ArithmeticModel,
        ic_rgb: IntegerDecompressor,
    }

    impl RGBDecompressor {
        pub fn new() -> Self {
            Self {
                last: Default::default(),
                have_last: false,
                byte_used_model: ArithmeticModelBuilder::new(64).build(),
                ic_rgb: IntegerDecompressorBuilder::new()
                    .bits(8) // 8 bits, because we encode byte by byte
                    .contexts(6) // there are 6 bytes in a RGB component
                    .build_initialized(),
            }
        }
    }

    impl<R: Read> FieldDecompressor<R> for RGBDecompressor {
        fn size_of_field(&self) -> usize {
            3 * size_of::<u16>()
        }

        fn decompress_with(
            &mut self,
            mut decoder: &mut ArithmeticDecoder<R>,
            mut buf: &mut [u8],
        ) -> std::io::Result<()> {
            if !self.have_last {
                decoder.in_stream().read_exact(&mut buf)?;
                self.last = RGB::unpack_from(buf);
                self.have_last = true;
            } else {
                let color_diff =
                    ColorDiff::new(decoder.decode_symbol(&mut self.byte_used_model)? as u8);

                if color_diff.lower_red_byte_changed() {
                    self.last.red =
                        self.ic_rgb
                            .decompress(&mut decoder, lower_byte(self.last.red) as i32, 0)?
                            as u16;
                }

                if color_diff.upper_red_byte_changed() {
                    self.last.red |=
                        self.ic_rgb
                            .decompress(&mut decoder, upper_byte(self.last.red) as i32, 1)?
                            as u16;
                }

                if color_diff.lower_green_byte_changed() {
                    self.last.green = self.ic_rgb.decompress(
                        &mut decoder,
                        lower_byte(self.last.green) as i32,
                        2,
                    )? as u16;
                }

                if color_diff.upper_green_byte_changed() {
                    self.last.green |= self.ic_rgb.decompress(
                        &mut decoder,
                        upper_byte(self.last.green) as i32,
                        3,
                    )? as u16;
                }

                if color_diff.lower_blue_byte_changed() {
                    self.last.blue = self.ic_rgb.decompress(
                        &mut decoder,
                        lower_byte(self.last.blue) as i32,
                        4,
                    )? as u16;
                }

                if color_diff.upper_blue_byte_changed() {
                    self.last.blue |= self.ic_rgb.decompress(
                        &mut decoder,
                        upper_byte(self.last.blue) as i32,
                        5,
                    )? as u16;
                }

                self.last.pack_into(&mut buf);
            }
            Ok(())
        }
    }

    pub struct RGBCompressor {
        last: Option<RGB>,
        byte_used_model: ArithmeticModel,
        ic_rgb: IntegerCompressor,
    }

    impl RGBCompressor {
        pub fn new() -> Self {
            Self {
                last: None,
                byte_used_model: ArithmeticModelBuilder::new(64).build(),
                ic_rgb: IntegerCompressorBuilder::new()
                    .bits(8)
                    .contexts(6)
                    .build_initialized(),
            }
        }
    }

    impl<W: Write> FieldCompressor<W> for RGBCompressor {
        fn size_of_field(&self) -> usize {
            3 * size_of::<u16>()
        }

        fn compress_with(
            &mut self,
            mut encoder: &mut ArithmeticEncoder<W>,
            buf: &[u8],
        ) -> std::io::Result<()> {
            let current = RGB::unpack_from(buf);
            if let Some(last) = self.last.as_mut() {
                let sym = ((lower_byte(last.red) != lower_byte(current.red)) as u8) << 0
                    | ((upper_byte(last.red) != upper_byte(current.red)) as u8) << 1
                    | ((lower_byte(last.green) != lower_byte(current.green)) as u8) << 2
                    | ((upper_byte(last.green) != upper_byte(current.green)) as u8) << 3
                    | ((lower_byte(last.blue) != lower_byte(current.blue)) as u8) << 4
                    | ((upper_byte(last.blue) != upper_byte(current.blue)) as u8) << 5;

                encoder.encode_symbol(&mut self.byte_used_model, sym as u32)?;
                let color_diff = ColorDiff::new(sym);

                if color_diff.lower_red_byte_changed() {
                    self.ic_rgb.compress(
                        &mut encoder,
                        lower_byte(last.red) as i32,
                        lower_byte(current.red) as i32,
                        0,
                    )?;
                }

                if color_diff.upper_red_byte_changed() {
                    self.ic_rgb.compress(
                        &mut encoder,
                        upper_byte(last.red) as i32,
                        upper_byte(current.red) as i32,
                        1,
                    )?;
                }

                if color_diff.lower_green_byte_changed() {
                    self.ic_rgb.compress(
                        &mut encoder,
                        lower_byte(last.green) as i32,
                        lower_byte(current.green) as i32,
                        2,
                    )?;
                }

                if color_diff.upper_green_byte_changed() {
                    self.ic_rgb.compress(
                        &mut encoder,
                        upper_byte(last.green) as i32,
                        upper_byte(current.green) as i32,
                        3,
                    )?;
                }

                if color_diff.lower_blue_byte_changed() {
                    self.ic_rgb.compress(
                        &mut encoder,
                        lower_byte(last.blue) as i32,
                        lower_byte(current.blue) as i32,
                        4,
                    )?;
                }

                if color_diff.upper_blue_byte_changed() {
                    self.ic_rgb.compress(
                        &mut encoder,
                        upper_byte(last.blue) as i32,
                        upper_byte(current.blue) as i32,
                        5,
                    )?;
                }
            } else {
                encoder.out_stream().write_all(buf)?;
            }
            self.last = Some(current);
            Ok(())
        }
    }
}

pub mod v2 {
    use std::io::{Read, Write};
    use std::mem::size_of;

    use crate::decoders::ArithmeticDecoder;
    use crate::encoders::ArithmeticEncoder;
    use crate::record::{FieldCompressor, FieldDecompressor};
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;

    use super::{lower_byte, u8_clamp, upper_byte, RGB, ColorDiff};

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

        fn compress_with(
            &mut self,
            encoder: &mut ArithmeticEncoder<W>,
            buf: &[u8],
        ) -> std::io::Result<()> {
            let this_val = super::RGB::unpack_from(&buf);

            if !self.have_last {
                self.have_last = true;
                encoder.out_stream().write_all(&buf)?;
            } else {
                let mut diff_l = 0i32;
                let mut diff_h = 0i32;
                let mut corr;

                let sym: u32 = this_val.color_diff_bits(&self.last) as u32;

                encoder.encode_symbol(&mut self.byte_used, sym)?;
                let color_diff = ColorDiff{0: sym as u8};

                if color_diff.lower_red_byte_changed() {
                    diff_l = lower_byte(this_val.red) as i32 - lower_byte(self.last.red) as i32;
                    encoder.encode_symbol(&mut self.rgb_diff_0, diff_l as u8 as u32)?;
                }

                if color_diff.upper_red_byte_changed(){
                    diff_h = upper_byte(this_val.red) as i32 - upper_byte(self.last.red) as i32;
                    encoder.encode_symbol(&mut self.rgb_diff_1, diff_h as u8 as u32)?;
                }
                if (sym & (1 << 6)) != 0 {
                    if color_diff.lower_green_byte_changed() {
                        corr = lower_byte(this_val.green) as i32
                            - u8_clamp(diff_l + lower_byte(self.last.green) as i32) as i32;
                        encoder.encode_symbol(&mut self.rgb_diff_2, corr as u8 as u32)?;
                    }

                    if color_diff.lower_blue_byte_changed() {
                        diff_l = (diff_l + lower_byte(this_val.green) as i32
                            - lower_byte(self.last.green) as i32)
                            / 2;
                        corr = lower_byte(this_val.blue) as i32
                            - u8_clamp(diff_l + lower_byte(self.last.blue) as i32) as i32;
                        encoder.encode_symbol(&mut self.rgb_diff_4, corr as u8 as u32)?;
                    }

                    if color_diff.upper_green_byte_changed() {
                        corr = upper_byte(this_val.green) as i32
                            - u8_clamp(diff_h + upper_byte(self.last.green) as i32) as i32;
                        encoder.encode_symbol(&mut self.rgb_diff_3, corr as u8 as u32)?;
                    }

                    if color_diff.upper_blue_byte_changed() {
                        diff_h = (diff_h + upper_byte(this_val.green) as i32
                            - upper_byte(self.last.green) as i32)
                            / 2;
                        corr = upper_byte(this_val.blue) as i32
                            - u8_clamp(diff_h + upper_byte(self.last.blue) as i32) as i32;
                        encoder.encode_symbol(&mut self.rgb_diff_5, corr as u8 as u32)?;
                    }
                }
            }
            self.last = this_val;
            Ok(())
        }
    }

    pub type RGBDecompressor = RGBCompressor;

    impl<R: Read> FieldDecompressor<R> for RGBDecompressor {
        fn size_of_field(&self) -> usize {
            3 * size_of::<u16>()
        }

        fn decompress_with(
            &mut self,
            decoder: &mut ArithmeticDecoder<R>,
            mut buf: &mut [u8],
        ) -> std::io::Result<()> {
            if !self.have_last {
                decoder.in_stream().read_exact(&mut buf)?;
                self.last = RGB::unpack_from(&buf);
                self.have_last = true;
            } else {
                let sym = decoder.decode_symbol(&mut self.byte_used)?;
                let color_diff = ColorDiff{0: sym as u8};

                let mut this_val = RGB::default();
                let mut corr;
                let mut diff;

                if color_diff.lower_red_byte_changed() {
                    corr = decoder.decode_symbol(&mut self.rgb_diff_0)? as u8;
                    this_val.red = corr.wrapping_add(lower_byte(self.last.red)) as u16;
                } else {
                    this_val.red = self.last.red & 0xFF;
                }

                if color_diff.upper_red_byte_changed() {
                    corr = decoder.decode_symbol(&mut self.rgb_diff_1)? as u8;
                    this_val.red |= (corr.wrapping_add(upper_byte(self.last.red)) as u16) << 8;
                } else {
                    this_val.red |= self.last.red & 0xFF00;
                }

                if (sym & (1 << 6)) != 0 {
                    diff = lower_byte(this_val.red) as i32 - lower_byte(self.last.red) as i32;

                    if color_diff.lower_green_byte_changed() {
                        corr = decoder.decode_symbol(&mut self.rgb_diff_2)? as u8;
                        this_val.green = corr
                            .wrapping_add(u8_clamp(diff + lower_byte(self.last.green) as i32) as u8)
                            as u16;
                    } else {
                        this_val.green = self.last.green & 0x00FF;
                    }

                    if color_diff.lower_blue_byte_changed() {
                        corr = decoder.decode_symbol(&mut self.rgb_diff_4)? as u8;
                        diff = (diff + lower_byte(this_val.green) as i32
                            - lower_byte(self.last.green) as i32)
                            / 2;
                        this_val.blue = (corr
                            .wrapping_add(u8_clamp(diff + lower_byte(self.last.blue) as i32) as u8))
                            as u16;
                    } else {
                        this_val.blue = self.last.blue & 0x00FF;
                    }

                    diff = upper_byte(this_val.red) as i32 - upper_byte(self.last.red) as i32;
                    if color_diff.upper_green_byte_changed() {
                        corr = decoder.decode_symbol(&mut self.rgb_diff_3)? as u8;
                        this_val.green |= (corr
                            .wrapping_add(u8_clamp(diff + upper_byte(self.last.green) as i32))
                            as u16)
                            << 8;
                    } else {
                        this_val.green |= self.last.green & 0xFF00;
                    }

                    if color_diff.upper_blue_byte_changed() {
                        corr = decoder.decode_symbol(&mut self.rgb_diff_5)? as u8;
                        diff = (diff + upper_byte(this_val.green) as i32
                            - upper_byte(self.last.green) as i32)
                            / 2;

                        this_val.blue |= ((corr
                            + (u8_clamp(diff + upper_byte(self.last.blue) as i32)) as u8)
                            as u16)
                            << 08;
                    } else {
                        this_val.blue |= self.last.blue & 0xFF00;
                    }
                } else {
                    this_val.green = this_val.red;
                    this_val.blue = this_val.red;
                }
                this_val.pack_into(&mut buf);
                self.last = this_val;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lower_red_changed() {
        let a = RGB {
            red: 0,
            green: 0,
            blue: 0,
        };
        let b = RGB {
            red: 1,
            green: 0,
            blue: 0,
        };

        assert_eq!(a.color_diff_bits(&b), 0b00000001);
        assert_eq!(b.color_diff_bits(&a), 0b01000001);
    }

    #[test]
    fn upper_red_changed() {
        let a = RGB {
            red: 0,
            green: 0,
            blue: 0,
        };
        let b = RGB {
            red: 256,
            green: 0,
            blue: 0,
        };

        assert_eq!(a.color_diff_bits(&b), 0b00000010);
        assert_eq!(b.color_diff_bits(&a), 0b01000010);
    }

    #[test]
    fn lower_green_changes() {
        let a = RGB {
            red: 0,
            green: 0,
            blue: 0,
        };
        let b = RGB {
            red: 0,
            green: 1,
            blue: 0,
        };

        assert_eq!(a.color_diff_bits(&b), 0b00000100);
        assert_eq!(b.color_diff_bits(&a), 0b01000100);
    }

    #[test]
    fn upper_green_changes() {
        let a = RGB {
            red: 0,
            green: 0,
            blue: 0,
        };
        let b = RGB {
            red: 0,
            green: 256,
            blue: 0,
        };

        assert_eq!(a.color_diff_bits(&b), 0b00001000);
        assert_eq!(b.color_diff_bits(&a), 0b01001000);
    }

    #[test]
    fn lower_blue_changes() {
        let a = RGB {
            red: 0,
            green: 0,
            blue: 0,
        };
        let b = RGB {
            red: 0,
            green: 0,
            blue: 1,
        };

        assert_eq!(a.color_diff_bits(&b), 0b00010000);
        assert_eq!(b.color_diff_bits(&a), 0b01010000);
    }

    #[test]
    fn upper_blue_changes() {
        let a = RGB {
            red: 0,
            green: 0,
            blue: 0,
        };
        let b = RGB {
            red: 0,
            green: 0,
            blue: 256,
        };

        assert_eq!(a.color_diff_bits(&b), 0b00100000);
        assert_eq!(b.color_diff_bits(&a), 0b01100000);
    }

    #[test]
    fn test_nothing_changes() {
        let a = RGB::default();
        let b = RGB::default();

        assert_eq!(a.color_diff_bits(&b), 0b00000000);
        assert_eq!(b.color_diff_bits(&a), 0b00000000);
    }
}
