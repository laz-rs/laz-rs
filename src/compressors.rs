/*
===============================================================================

  CONTENTS:
    Integer compressor

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

use std::io::Write;

use crate::encoders;
use crate::models;

pub const DEFAULT_BITS: u32 = 16;
pub const DEFAULT_CONTEXTS: u32 = 1;
pub const DEFAULT_BITS_HIGH: u32 = 8;
pub const DEFAULT_RANGE: u32 = 0;
pub const DEFAULT_COMPRESS_CONTEXTS: u32 = 0;

const COMPRESS_ONLY_K: bool = false;

#[derive(Debug)]
pub struct IntegerCompressor {
    k: u32,

    // bits: u32,
    contexts: u32,
    bits_high: u32,
    // range: u32,
    corr_bits: u32,
    corr_range: u32,
    corr_min: i32,
    corr_max: i32,

    m_bits: Vec<models::ArithmeticModel>,
    m_corrector_0: models::ArithmeticBitModel,
    m_corrector: Vec<models::ArithmeticModel>,
}

impl IntegerCompressor {
    pub fn new(bits: u32, contexts: u32, bits_high: u32, mut range: u32) -> Self {
        let mut corr_bits: u32;
        let corr_range: u32;
        let corr_min: i32;
        let corr_max: i32;

        // the corrector's significant bits and range
        if range != 0 {
            corr_bits = 0;
            corr_range = range;

            while range != 0 {
                range >>= 1;
                corr_bits += 1;
            }
            if corr_range == (1u32 << (corr_bits - 1)) {
                corr_bits -= 1;
            }

            // the corrector must fall into this interval
            corr_min = -((corr_range / 2) as i32);
            corr_max = corr_min + (corr_range - 1) as i32;
        } else if bits >= 1 && bits < 32 {
            corr_bits = bits;
            corr_range = 1u32 << bits;

            // the corrector must fall into this interval
            corr_min = -((corr_range / 2) as i32);
            corr_max = corr_min + (corr_range - 1) as i32;
        } else {
            corr_bits = 32;
            corr_range = 0;
            // the corrector must fall into this interval
            corr_min = std::i32::MIN;
            corr_max = std::i32::MAX;
        }

        Self {
            k: 0,
            // bits,
            contexts,
            bits_high,
            // range,
            corr_bits,
            corr_range,
            corr_min,
            corr_max,
            m_bits: vec![],
            m_corrector_0: models::ArithmeticBitModel::new(),
            m_corrector: vec![],
        }
    }

    pub fn k(&self) -> u32 {
        self.k
    }
    pub fn init(&mut self) {
        if self.m_bits.is_empty() {
            for _i in 0..self.contexts {
                self.m_bits
                    .push(models::ArithmeticModel::new(self.corr_bits + 1, false, &[]));
            }

            if !COMPRESS_ONLY_K {
                for i in 1..=self.corr_bits {
                    let v = if i <= self.bits_high {
                        1 << i
                    } else {
                        1 << self.bits_high
                    };
                    self.m_corrector
                        .push(models::ArithmeticModel::new(v, false, &[]))
                }
            }
        }
    }

    pub fn compress<T: Write>(
        &mut self,
        encoder: &mut encoders::ArithmeticEncoder<T>,
        pred: i32,
        real: i32,
        context: u32,
    ) -> std::io::Result<()> {
        // the corrector will be within the interval [ - (corr_range - 1)  ...  + (corr_range - 1) ]
        let mut corr = real.wrapping_sub(pred);
        // we fold the corrector into the interval [ corr_min  ...  corr_max ]
        if corr < self.corr_min {
            corr += self.corr_range as i32;
        } else if corr > self.corr_max {
            corr -= self.corr_range as i32;
        }

        let m_bit = &mut self.m_bits[context as usize];
        let mut c = corr;
        //===== start of "writeCorrector ==============================================*/
        let mut c1: u32;

        // find the tightest interval [ - (2^k - 1)  ...  + (2^k) ] that contains c

        self.k = 0;

        // do this by checking the absolute value of c (adjusted for the case that c is 2^k)
        c1 = if c <= 0 { c.wrapping_neg() } else { c - 1 } as u32;

        // this loop could be replaced with more efficient code
        while c1 != 0 {
            c1 >>= 1;
            self.k += 1;
        }

        // the number k is between 0 and corr_bits and describes the interval the corrector
        encoder.encode_symbol(m_bit, self.k)?;
        if COMPRESS_ONLY_K {
            //TODO
            panic!("COMPRESS_ONLY_K == true is not supported");
        } else {
            if self.k != 0 {
                // then c is either smaller than 0 or bigger than 1
                debug_assert!(c != 0 && c != 1);
                if self.k < 32 {
                    // translate the corrector c into the k-bit interval [ 0 ... 2^k - 1 ]
                    if c >= 0 {
                        // so we translate c into the interval [ 2^(k-1) ...  + 2^k - 1 ] by subtracting 1
                        c -= 1;
                    } else {
                        // so we translate c into the interval [ 0 ...  + 2^(k-1) - 1 ] by adding (2^k - 1)
                        c += ((1u32 << self.k) - 1) as i32;
                    }

                    if self.k <= self.bits_high
                    // for small k we code the interval in one step
                    {
                        // compress c with the range coder
                        encoder.encode_symbol(
                            &mut self.m_corrector[(self.k - 1) as usize],
                            c as u32,
                        )?;
                    } else
                    // for larger k we need to code the interval in two steps
                    {
                        // figure out how many lower bits there are
                        let k1 = self.k - self.bits_high;
                        // c1 represents the lowest k-bits_high+1 bits
                        c1 = (c & ((1u32 << k1) - 1u32) as i32) as u32;
                        // c represents the highest bits_high bits
                        c >>= k1 as i32;
                        // compress the higher bits using a context table
                        encoder.encode_symbol(
                            &mut self.m_corrector[(self.k - 1) as usize],
                            c as u32,
                        )?;
                        // store the lower k1 bits raw
                        encoder.write_bits(k1, c1)?;
                    }
                }
            } else {
                // then c is 0 or 1
                debug_assert!(c == 0 || c == 1);
                encoder.encode_bit(&mut self.m_corrector_0, c as u32)?;
            }
            Ok(())
        } // end COMPRESS_ONLY_K
    }
}

pub struct IntegerCompressorBuilder {
    bits: u32,
    contexts: u32,
    bits_high: u32,
    range: u32,
}

impl IntegerCompressorBuilder {
    pub fn new() -> Self {
        Self {
            bits: DEFAULT_BITS,
            contexts: DEFAULT_CONTEXTS,
            bits_high: DEFAULT_BITS_HIGH,
            range: DEFAULT_RANGE,
        }
    }

    pub fn bits(&mut self, bits: u32) -> &mut Self {
        self.bits = bits;
        self
    }

    pub fn contexts(&mut self, contexts: u32) -> &mut Self {
        self.contexts = contexts;
        self
    }

    pub fn build(&self) -> IntegerCompressor {
        IntegerCompressor::new(self.bits, self.contexts, self.bits_high, self.range)
    }

    pub fn build_initialized(&self) -> IntegerCompressor {
        let mut ic = self.build();
        ic.init();
        ic
    }
}
