/*
===============================================================================

  CONTENTS:
    Integer decompressor

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

use crate::decoders;
use crate::models;

const DEFAULT_BITS: u32 = 16;
const DEFAULT_CONTEXTS: u32 = 1;
const DEFAULT_BITS_HIGH: u32 = 8;
const DEFAULT_RANGE: u32 = 0;

pub const DEFAULT_DECOMPRESS_CONTEXTS: u32 = 0;

const COMPRESS_ONLY_K: bool = false;

#[derive(Clone)]
pub struct IntegerDecompressor {
    k: u32,

    //bits: u32,
    contexts: u32,
    bits_high: u32,
    //range: u32,
    corr_bits: u32,
    corr_range: u32,
    corr_min: i32,
    //corr_max: i32,
    m_bits: Vec<models::ArithmeticModel>,
    m_corrector0: models::ArithmeticBitModel,
    m_corrector: Vec<models::ArithmeticModel>,
}

impl IntegerDecompressor {
    pub fn new(bits: u32, contexts: u32, bits_high: u32, mut range: u32) -> Self {
        let mut corr_bits: u32;
        let corr_range: u32;
        let corr_min: i32;
        //let corr_max: i32;

        if range != 0 {
            // the corrector's significant bits and range
            corr_bits = 0;
            corr_range = range;

            while range != 0 {
                range = range >> 1;
                corr_bits += 1;
            }
            if corr_range == 1u32 << (corr_bits - 1) {
                corr_bits -= 1;
            }
            // the corrector must fall into this interval
            corr_min = -(corr_range as i32 / 2);
        //corr_max = (corr_min + corr_range as i32 - 1) as i32;
        } else if bits != 0 && (bits < 32) {
            corr_bits = bits;
            corr_range = 1u32 << bits;
            // the corrector must fall into this interval
            corr_min = -(corr_range as i32 / 2);
        //corr_max = (corr_min + corr_range as i32 - 1) as i32;
        } else {
            corr_bits = 32;
            corr_range = 0;
            // the corrector must fall into this interval
            corr_min = std::i32::MIN;
            //corr_max = std::i32::MAX;
        }
        Self {
            k: 0,
            //bits,
            contexts,
            bits_high,
            //range,
            corr_bits,
            corr_range,
            corr_min,
            m_bits: vec![],
            m_corrector0: models::ArithmeticBitModel::new(),
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

            // m_corrector0 is already initialized
            if !COMPRESS_ONLY_K {
                for i in 1..=self.corr_bits {
                    let v = if i <= self.bits_high {
                        1 << i
                    } else {
                        1 << self.bits_high
                    };
                    self.m_corrector
                        .push(models::ArithmeticModel::new(v, false, &[]));
                }
            }
        }
    }

    pub fn decompress<T: Read>(
        &mut self,
        dec: &mut decoders::ArithmeticDecoder<T>,
        pred: i32,
        context: u32,
    ) -> std::io::Result<i32> {
        let m_bit = &mut self.m_bits[context as usize];
        //--- read corrector ---//
        let corr = {
            let mut c: i32;
            // decode within which interval the corrector is falling
            self.k = dec.decode_symbol(m_bit)?;
            // decode the exact location of the corrector within the interval
            if COMPRESS_ONLY_K {
                if self.k != 0 {
                    // then c is either smaller than 0 or bigger than 1
                    if self.k < 32 {
                        c = dec.read_bits(self.k)? as i32;
                        if c >= (1u32 << (self.k - 1)) as i32 {
                            // if c is in the interval [ 2^(k-1)  ...  + 2^k - 1 ]
                            // so we translate c back into the interval [ 2^(k-1) + 1  ...  2^k ] by adding 1
                            c += 1;
                        } else {
                            // otherwise c is in the interval [ 0 ...  + 2^(k-1) - 1 ]
                            // so we translate c back into the interval [ - (2^k - 1)  ...  - (2^(k-1)) ] by subtracting (2^k - 1)
                            c -= ((1u32 << self.k) - 1) as i32;
                        }
                    } else {
                        c = self.corr_min;
                    }
                } else {
                    c = dec.read_bit()? as i32;
                }
            }
            // COMPRESS_ONLY_K
            else {
                if self.k != 0 {
                    // then c is either smaller than 0 or bigger than 1
                    if self.k < 32 {
                        if self.k <= self.bits_high {
                            // for small k we can do this in one step
                            // decompress c with the range coder
                            c = dec.decode_symbol(&mut self.m_corrector[(self.k - 1) as usize])?
                                as i32;
                        } else {
                            // for larger k we need to do this in two steps
                            let k1 = self.k - self.bits_high;
                            // decompress higher bits with table
                            c = dec.decode_symbol(&mut self.m_corrector[(self.k - 1) as usize])?
                                as i32;
                            let c1 = dec.read_bits(k1)?;
                            // put the corrector back together
                            c = (c << k1 as i32) | c1 as i32;
                        }

                        // translate c back into its correct interval
                        if c >= (1u32 << (self.k - 1)) as i32 {
                            // so we translate c back into the interval [ 2^(k-1) + 1  ...  2^k ] by adding 1
                            c += 1;
                        } else {
                            // otherwise c is in the interval [ 0 ...  + 2^(k-1) - 1 ]
                            // so we translate c back into the interval [ - (2^k - 1)  ...  - (2^(k-1)) ] by subtracting (2^k - 1)
                            c -= ((1u32 << self.k) - 1) as i32;
                        }
                    } else {
                        c = self.corr_min;
                    }
                } else {
                    c = dec.decode_bit(&mut self.m_corrector0)? as i32;
                }
            } // COMPRESS_ONLY_K
            c
        };
        //--- read corrector ---//
        let mut real = pred.wrapping_add(corr);
        if real < 0 {
            real += self.corr_range as i32;
        } else if real >= self.corr_range as i32 {
            real -= self.corr_range as i32
        }
        Ok(real)
    }
}

pub struct IntegerDecompressorBuilder {
    bits: u32,
    contexts: u32,
    bits_high: u32,
    range: u32,
}

impl IntegerDecompressorBuilder {
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

    pub fn build(&self) -> IntegerDecompressor {
        IntegerDecompressor::new(self.bits, self.contexts, self.bits_high, self.range)
    }

    pub fn build_initialized(&self) -> IntegerDecompressor {
        let mut idc = self.build();
        idc.init();
        idc
    }
}
