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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//                                                                           -
//                       ****************************                        -
//                        ARITHMETIC CODING EXAMPLES                         -
//                       ****************************                        -
//                                                                           -
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//                                                                           -
// Fast arithmetic coding implementation                                     -
// -> 32-bit variables, 32-bit product, periodic updates, table decoding     -
//                                                                           -
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//                                                                           -
// Version 1.00  -  April 25, 2004                                           -
//                                                                           -
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//                                                                           -
//                                  WARNING                                  -
//                                 =========                                 -
//                                                                           -
// The only purpose of this program is to demonstrate the basic principles   -
// of arithmetic coding. The original version of this code can be found in   -
// Digital Signal Compression: Principles and Practice                       -
// (Cambridge University Press, 2011, ISBN: 9780511984655)                   -
//                                                                           -
// Copyright (c) 2019 by Amir Said (said@ieee.org) &                         -
//                       William A. Pearlman (pearlw@ecse.rpi.edu)           -
//                                                                           -
// Redistribution and use in source and binary forms, with or without        -
// modification, are permitted provided that the following conditions are    -
// met:                                                                      -
//                                                                           -
// 1. Redistributions of source code must retain the above copyright notice, -
// this list of conditions and the following disclaimer.                     -
//                                                                           -
// 2. Redistributions in binary form must reproduce the above copyright      -
// notice, this list of conditions and the following disclaimer in the       -
// documentation and/or other materials provided with the distribution.      -
//                                                                           -
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS       -
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED -
// TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A           -
// PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER -
// OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,  -
// EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,       -
// PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR        -
// PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF    -
// LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING      -
// NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS        -
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.              -
//                                                                           -
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//                                                                           -
// A description of the arithmetic coding method used here is available in   -
//                                                                           -
// Lossless Compression Handbook, ed. K. Sayood                              -
// Chapter 5: Arithmetic Coding (A. Said), pp. 101-152, Academic Press, 2003 -
//                                                                           -
// A. Said, Introduction to Arithetic Coding Theory and Practice             -
// HP Labs report HPL-2004-76  -  http://www.hpl.hp.com/techreports/         -
//                                                                           -
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

use byteorder::ReadBytesExt;
use std::io::Read;

use crate::models;
use crate::models::DM_LENGTH_SHIFT;

// threshold for renormalization
pub const AC_MAX_LENGTH: u32 = 0xFFFFFFFF;
// maximum AC interval length
pub const AC_MIN_LENGTH: u32 = 0x01000000;

pub struct ArithmeticDecoder<T: Read> {
    in_stream: T,
    value: u32,
    length: u32,
}

impl<T: Read> ArithmeticDecoder<T> {
    pub fn new(in_stream: T) -> Self {
        Self {
            in_stream,
            value: 0,
            length: AC_MAX_LENGTH,
        }
    }

    pub fn reset(&mut self) {
        self.value = 0;
        self.length = AC_MAX_LENGTH;
    }

    pub fn read_init_bytes(&mut self) -> std::io::Result<()> {
        let mut v = [0u8; 4];
        self.in_stream.read_exact(&mut v)?;

        self.value = (v[0] as u32) << 24 | (v[1] as u32) << 16 | (v[2] as u32) << 8 | v[3] as u32;
        Ok(())
    }

    pub fn decode_bit(&mut self, model: &mut models::ArithmeticBitModel) -> std::io::Result<u32> {
        let x = model.bit_0_prob * (self.length >> models::BM_LENGTH_SHIFT); // product l x p0

        let sym = self.value >= x;

        if !sym {
            self.length = x;
            model.bit_0_count += 1;
        } else {
            self.value -= x;
            self.length -= x;
        }
        if self.length < AC_MIN_LENGTH {
            self.renorm_dec_interval()?;
        }
        model.bits_until_update -= 1;
        if model.bits_until_update == 0 {
            model.update();
        }
        Ok(sym as u32)
    }

    pub fn decode_symbol(&mut self, model: &mut models::ArithmeticModel) -> std::io::Result<u32> {
        let mut sym;
        let mut n;
        let mut x;
        let mut y = self.length;
        //this was a null ptr check
        if !model.decoder_table.is_empty() {
            // use table look-up for faster decoding
            self.length >>= DM_LENGTH_SHIFT;
            let dv = self.value / self.length;
            let t = dv >> model.table_shift;

            sym = model.decoder_table[t as usize]; // initial decision based on table look-up
            n = model.decoder_table[t as usize + 1] + 1;

            while n > sym + 1 {
                // finish with bisection search
                let k = (sym + n) >> 1;
                if model.distribution[k as usize] > dv {
                    n = k;
                } else {
                    sym = k;
                }
            }
            // compute products
            x = model.distribution[sym as usize] * self.length;
            if sym != model.last_symbol {
                y = model.distribution[sym as usize + 1] * self.length;
            }
        } else {
            x = 0;
            sym = 0;
            self.length >>= DM_LENGTH_SHIFT;
            n = model.symbols;
            let mut k = n >> 1;

            // Rust has no do-while
            loop {
                let z = self.length * model.distribution[k as usize];
                if z > self.value {
                    n = k;
                    y = z; // value is smaller
                } else {
                    sym = k;
                    x = z; // value is larger or equal
                }
                k = (sym + n) >> 1;
                if k == sym {
                    break;
                }
            }
        }
        // update interval
        self.value -= x;
        self.length = y - x;

        if self.length < AC_MIN_LENGTH {
            self.renorm_dec_interval()?;
        }
        model.symbol_count[sym as usize] += 1;
        model.symbols_until_update -= 1;
        if model.symbols_until_update == 0 {
            model.update();
        }
        Ok(sym)
    }

    pub fn read_bit(&mut self) -> std::io::Result<u32> {
        // decode symbol, change length
        self.length >>= 1;
        let sym = self.value / self.length;
        // update interval
        self.value -= self.length * sym;

        if self.length < AC_MIN_LENGTH {
            self.renorm_dec_interval()?;
        }
        Ok(sym)
    }

    pub fn read_bits(&mut self, mut bits: u32) -> std::io::Result<u32> {
        assert!(bits > 0 && (bits <= 32));
        if bits > 19 {
            let tmp = self.read_short()? as u32;
            bits -= 16;
            let tmpl = self.read_bits(bits)? << 16;
            Ok(tmpl | tmp)
        } else {
            // decode symbol, change length
            self.length >>= bits;
            let sym = self.value / self.length;

            // update interval
            self.value -= self.length * sym;

            if self.length < AC_MIN_LENGTH {
                self.renorm_dec_interval()?;
            }
            Ok(sym)
        }
    }

    #[allow(dead_code)]
    fn read_byte(&mut self) -> std::io::Result<u8> {
        // decode symbol, change length
        self.length >>= 8;
        let sym = self.value / self.length;
        // update interval
        self.value -= self.length * sym;
        if self.length < AC_MIN_LENGTH {
            self.renorm_dec_interval()?;
        }
        assert!(sym < (1 << 8));
        Ok(sym as u8)
    }

    fn read_short(&mut self) -> std::io::Result<u16> {
        // decode symbol, change length
        self.length >>= 16;
        let sym = self.value / self.length;
        // update interval
        self.value -= self.length * sym;
        if self.length < AC_MIN_LENGTH {
            self.renorm_dec_interval()?;
        }
        assert!(sym < (1 << 16));
        Ok(sym as u16)
    }

    pub fn read_int(&mut self) -> std::io::Result<u32> {
        let lower_int = self.read_short()?;
        let upper_int = self.read_short()?;
        Ok((upper_int as u32) << 16 | (lower_int) as u32)
    }

    pub fn read_int_64(&mut self) -> std::io::Result<u64> {
        let lower_int = self.read_int()? as u64;
        let upper_int = self.read_int()? as u64;
        Ok((upper_int << 32) | lower_int)
    }

    //TODO readFloat, readDouble
    fn renorm_dec_interval(&mut self) -> std::io::Result<()> {
        loop {
            self.value = (self.value << 8) | self.in_stream.read_u8()? as u32;
            self.length <<= 8;
            if self.length >= AC_MIN_LENGTH {
                break;
            }
        }
        Ok(())
    }

    pub fn in_stream(&mut self) -> &mut T {
        &mut self.in_stream
    }

    pub fn into_stream(self) -> T {
        self.in_stream
    }
}
