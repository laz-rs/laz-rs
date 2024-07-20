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
    terms of the Apache Public License 2.0 published by the Apache Software
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

use crate::decoders;
use crate::decoders::AC_MIN_LENGTH;
use crate::models;
use crate::models::DM_LENGTH_SHIFT;
use std::io::Write;

const AC_BUFFER_SIZE: usize = 4096;

pub struct ArithmeticEncoder<T: Write> {
    out_buffer: Box<[u8]>,

    out_byte: *mut u8,
    end_byte: *const u8,

    base: u32,
    length: u32,

    out_stream: T,
}

impl<T: Write> ArithmeticEncoder<T> {
    pub fn new(out_stream: T) -> Self {
        let mut out_buffer = vec![0u8; 2 * AC_BUFFER_SIZE].into_boxed_slice();

        let out_byte = out_buffer.as_mut_ptr_range().start;
        let end_byte = out_buffer.as_mut_ptr_range().end;

        Self {
            out_buffer,
            out_byte,
            end_byte,
            base: 0,
            length: decoders::AC_MAX_LENGTH,
            out_stream,
        }
    }

    #[inline]
    fn end_of_buffer(&self) -> *const u8 {
        self.out_buffer.as_ptr_range().end
    }

    pub fn reset(&mut self) {
        self.base = 0;
        self.length = decoders::AC_MAX_LENGTH;
        self.out_buffer.fill(0);
        self.out_byte = self.out_buffer.as_mut_ptr();
        self.end_byte = self.end_of_buffer();
    }

    pub fn done(&mut self) -> std::io::Result<()> {
        // done encoding: set final data bytes
        let init_base = self.base;
        let mut another_byte = true;

        if self.length > 2 * AC_MIN_LENGTH {
            // base offset
            self.base = self.base.wrapping_add(AC_MIN_LENGTH);
            // set new length for 1 more byte
            self.length = AC_MIN_LENGTH >> 1;
        } else {
            // base offset
            self.base = self.base.wrapping_add(AC_MIN_LENGTH >> 1);
            // set new length for 2 more bytes
            self.length = AC_MIN_LENGTH >> 9;
            another_byte = false;
        }

        if init_base > self.base {
            self.propagate_carry();
        }
        self.renorm_enc_interval()?;

        if self.end_byte != self.end_of_buffer() {
            debug_assert!(
                (self.out_byte.cast_const())
                    < self.out_buffer.as_ptr().wrapping_add(AC_BUFFER_SIZE)
            );
            let slc = unsafe {
                std::slice::from_raw_parts(
                    self.out_buffer.as_ptr().wrapping_add(AC_BUFFER_SIZE),
                    AC_BUFFER_SIZE,
                )
            };
            self.out_stream.write_all(&slc)?;
        }

        let buffer_size = self.out_byte as isize - self.out_buffer.as_ptr() as isize;
        if buffer_size != 0 {
            let slc = &self.out_buffer[..buffer_size as usize];
            self.out_stream.write_all(&slc)?
        }

        self.out_stream.write_all(&[0u8, 0u8])?;

        if another_byte {
            self.out_stream.write_all(&[0u8])?
        }
        Ok(())
    }

    //TODO symbol is a bit, should it be bool type instead ?
    pub fn encode_bit(
        &mut self,
        model: &mut models::ArithmeticBitModel,
        sym: u32,
    ) -> std::io::Result<()> {
        debug_assert!(sym <= 1);
        // product l x p0
        let x = model.bit_0_prob * (self.length >> models::BM_LENGTH_SHIFT);

        //update interval
        if sym == 0 {
            self.length = x;
            model.bit_0_count += 1;
        } else {
            let init_base = self.base;
            self.base = self.base.wrapping_add(x);
            self.length -= x;
            if init_base > self.base {
                // overflow = carry
                self.propagate_carry();
            }
        }
        if self.length < decoders::AC_MIN_LENGTH {
            self.renorm_enc_interval()?;
        }

        model.bits_until_update -= 1;
        if model.bits_until_update == 0 {
            model.update();
        }
        Ok(())
    }

    pub fn encode_symbol(
        &mut self,
        model: &mut models::ArithmeticModel,
        sym: u32,
    ) -> std::io::Result<()> {
        debug_assert!(sym <= model.last_symbol);

        let x;
        let init_base = self.base;

        //compute products
        if sym == model.last_symbol {
            x = model.distribution[sym as usize] * (self.length >> DM_LENGTH_SHIFT);
            self.base = self.base.wrapping_add(x); // update interval
            self.length -= x; // no product needed
        } else {
            self.length >>= DM_LENGTH_SHIFT;
            x = model.distribution[sym as usize] * self.length;
            self.base = self.base.wrapping_add(x);
            self.length = model.distribution[(sym + 1) as usize] * self.length - x;
        }

        if init_base > self.base {
            self.propagate_carry();
        }
        if self.length < AC_MIN_LENGTH {
            self.renorm_enc_interval()?;
        }
        model.symbol_count[sym as usize] += 1;
        model.symbols_until_update -= 1;
        if model.symbols_until_update == 0 {
            model.update();
        }
        Ok(())
    }

    /* Encode a bit without modelling  */
    // again sym is a bool
    #[allow(dead_code)]
    pub fn write_bit(&mut self, sym: u32) -> std::io::Result<()> {
        debug_assert!(sym <= 1);

        let init_base = self.base;
        // new interval base and length
        self.length >>= 1;
        self.base = self.base.wrapping_add(sym * self.length);

        // overflow = carry
        if init_base > self.base {
            self.propagate_carry();
        }

        if self.length < AC_MIN_LENGTH {
            self.renorm_enc_interval()?;
        }
        Ok(())
    }

    pub fn write_bits(&mut self, mut bits: u32, mut sym: u32) -> std::io::Result<()> {
        debug_assert!(bits <= 32 && sym < (1u32 << bits));

        if bits > 19 {
            self.write_short((sym & u32::from(std::u16::MAX)) as u16)?;
            sym >>= 16;
            bits -= 16;
        }

        let init_base = self.base;
        // new interval base and length
        self.length >>= bits;
        self.base = self.base.wrapping_add(sym * self.length);

        // overflow = carry
        if init_base > self.base {
            self.propagate_carry();
        }

        if self.length < AC_MIN_LENGTH {
            self.renorm_enc_interval()?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn write_byte(&mut self, sym: u8) -> std::io::Result<()> {
        let init_base = self.base;
        self.length >>= 8;

        self.base = self.base.wrapping_add(u32::from(sym) * self.length);
        // overflow = carry
        if init_base > self.base {
            self.propagate_carry();
        }

        if self.length < AC_MIN_LENGTH {
            self.renorm_enc_interval()?;
        }
        Ok(())
    }

    pub fn write_short(&mut self, sym: u16) -> std::io::Result<()> {
        let init_base = self.base;
        self.length >>= 16;

        self.base = self.base.wrapping_add(u32::from(sym) * self.length);
        // overflow = carry
        if init_base > self.base {
            self.propagate_carry();
        }

        if self.length < AC_MIN_LENGTH {
            self.renorm_enc_interval()?;
        }
        Ok(())
    }

    pub fn write_int(&mut self, sym: u32) -> std::io::Result<()> {
        // lower 16 bits
        self.write_short((sym & 0x0000_FFFFu32) as u16)?;
        // upper 16 bits
        self.write_short((sym >> 16) as u16)
    }

    pub fn write_int64(&mut self, sym: u64) -> std::io::Result<()> {
        // lower 32 bits
        self.write_int((sym & 0x0000_0000_FFFF_FFFF) as u32)?;
        // upper 32 bits
        self.write_int((sym >> 32) as u32)
    }

    pub fn get_ref(&self) -> &T {
        &self.out_stream
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.out_stream
    }

    pub fn into_inner(self) -> T {
        self.out_stream
    }

    fn propagate_carry(&mut self) {
        let mut p = if self.out_byte.cast_const() == self.out_buffer.as_ptr() {
            self.end_of_buffer().wrapping_sub(1).cast_mut()
        } else {
            self.out_byte.wrapping_sub(1)
        };

        unsafe {
            while *p == 0xFFu8 {
                *p = 0;
                if p.cast_const() == self.out_buffer.as_ptr() {
                    p = self.end_of_buffer().wrapping_sub(1).cast_mut()
                } else {
                    p = p.wrapping_sub(1);
                }
                debug_assert!(self.out_buffer.as_ptr() <= p);
                debug_assert!(p.cast_const() < self.end_of_buffer());
                debug_assert!(self.out_byte.cast_const() < self.end_of_buffer());
            }
            *p += 1;
        }
    }

    fn renorm_enc_interval(&mut self) -> std::io::Result<()> {
        loop {
            debug_assert!(self.out_buffer.as_ptr() <= self.out_byte);
            debug_assert!(self.out_byte.cast_const() < self.end_of_buffer());
            debug_assert!(self.out_byte.cast_const() < self.end_byte);
            unsafe {
                *self.out_byte = (self.base >> 24) as u8;
            }
            self.out_byte = self.out_byte.wrapping_add(1);

            if self.out_byte.cast_const() == self.end_byte {
                self.manage_out_buffer()?;
            }
            self.base <<= 8;
            self.length <<= 8; // length multiplied by 256
            if self.length >= AC_MIN_LENGTH {
                break;
            }
        }
        Ok(())
    }

    fn manage_out_buffer(&mut self) -> std::io::Result<()> {
        debug_assert!(self.out_byte.cast_const() == self.end_byte);

        if self.out_byte.cast_const() == self.end_of_buffer() {
            self.out_byte = self.out_buffer.as_mut_ptr();
        }

        let slc = unsafe { std::slice::from_raw_parts(self.out_byte, AC_BUFFER_SIZE) };
        self.out_stream.write_all(slc)?;
        self.end_byte = self.out_byte.wrapping_add(AC_BUFFER_SIZE);

        debug_assert!(self.end_byte > self.out_byte);
        debug_assert!(self.out_byte.cast_const() < self.end_of_buffer());
        Ok(())
    }
}

unsafe impl<T: Write + Send> Send for ArithmeticEncoder<T> {}
