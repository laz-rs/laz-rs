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

// length bits discarded before mult.
pub(crate) const DM_LENGTH_SHIFT: u32 = 15;
pub(crate) const DM_MAX_COUNT: u32 = 1 << DM_LENGTH_SHIFT; // for adaptive models

// length bits discarded before mult.
pub(crate) const BM_MAX_COUNT: u32 = 1 << BM_LENGTH_SHIFT;
// for adaptive models
pub(crate) const BM_LENGTH_SHIFT: u32 = 13;

#[derive(Debug)]
pub struct ArithmeticModel {
    pub(crate) symbols: u32,
    pub(crate) compress: bool,

    pub(crate) distribution: Vec<u32>,
    pub(crate) symbol_count: Vec<u32>,
    pub(crate) decoder_table: Vec<u32>,

    pub(crate) total_count: u32,
    pub(crate) update_cycle: u32,
    pub(crate) symbols_until_update: u32,
    pub(crate) last_symbol: u32,
    pub(crate) table_size: u32,
    pub(crate) table_shift: u32,
}

impl ArithmeticModel {
    pub fn new(symbols: u32, compress: bool, init_table: &[u32]) -> Self {
        if symbols < 2 || (symbols > (1 << 11)) {
            panic!("Invalid number of symbols");
        }

        let mut model = Self {
            symbols,
            compress,
            distribution: Vec::<u32>::new(),
            symbol_count: Vec::<u32>::new(),
            decoder_table: Vec::<u32>::new(),
            total_count: 0,
            update_cycle: 0,
            symbols_until_update: 0,
            last_symbol: 0,
            table_size: 0,
            table_shift: 0,
        };

        model.last_symbol = model.symbols - 1;
        if !compress && model.symbols > 16 {
            let mut table_bits = 3u32;
            while symbols > (1u32 << (table_bits + 2)) {
                table_bits += 1;
            }
            model.table_size = 1 << table_bits;
            model.table_shift = DM_LENGTH_SHIFT - table_bits;
            model.decoder_table = vec![0u32; (model.table_size + 2) as usize];
        } else {
            model.table_size = 0;
            model.table_shift = 0;
        }

        model.distribution = vec![0u32; (model.symbols) as usize];
        model.symbol_count = vec![0u32; (model.symbols) as usize];
        model.update_cycle = model.symbols;

        if !init_table.is_empty() {
            for k in 0..model.symbols {
                model.symbol_count[k as usize] = init_table[k as usize];
            }
        } else {
            for k in 0..model.symbols {
                model.symbol_count[k as usize] = 1;
            }
        }

        model.update();
        model.symbols_until_update = (model.symbols + 6) >> 1;
        model.update_cycle = (model.symbols + 6) >> 1;
        model
    }

    pub fn update(&mut self) {
        //dbg!("ArithmeticModel::update");
        //halve counts when a threshold os reached
        self.total_count += self.update_cycle;
        if self.total_count > DM_MAX_COUNT {
            self.total_count = 0;
            for symbol_count in &mut self.symbol_count {
                *symbol_count = (*symbol_count + 1) >> 1;
                self.total_count += *symbol_count;
            }
            /* for n in 0..self.symbols as usize {
                self.symbol_count[n] = (self.symbol_count[n] + 1) >> 1;
                self.total_count += self.symbol_count[n];
            }*/
        }

        // compute cumulative distribution, decoder table
        let mut sum = 0u32;
        let scale = 0x80000000u32 / self.total_count;
        let mut s = 0usize;

        if self.compress || self.table_size == 0 {
            for (symbol_distribution, symbol_count) in
                self.distribution.iter_mut().zip(&self.symbol_count)
            {
                *symbol_distribution = (scale * sum) >> (31 - DM_LENGTH_SHIFT);
                sum += *symbol_count;
            }
        } else {
            for (k, (symbol_distribution, symbol_count)) in self
                .distribution
                .iter_mut()
                .zip(&self.symbol_count)
                .enumerate()
            {
                *symbol_distribution = (scale * sum) >> (31 - DM_LENGTH_SHIFT);
                sum += *symbol_count;
                let w = *symbol_distribution >> self.table_shift;

                assert!((w as usize) < self.decoder_table.len());
                while s < w as usize {
                    s += 1;
                    *unsafe { self.decoder_table.get_unchecked_mut(s) } = (k - 1) as u32;
                }
            }

            self.decoder_table[0] = 0;
            //decoder_table as self.table_size + 2 elements
            debug_assert!(self.decoder_table.len() >= self.table_size as usize);
            while s <= self.table_size as usize {
                s += 1;
                *unsafe { self.decoder_table.get_unchecked_mut(s) } = self.symbols - 1;
            }
        }

        self.update_cycle = (5 * self.update_cycle) >> 2;
        let max_cycle = (self.symbols + 6) << 3;

        if self.update_cycle > max_cycle {
            self.update_cycle = max_cycle;
        }
        self.symbols_until_update = self.update_cycle;
    }
}

#[derive(Debug)]
pub struct ArithmeticBitModel {
    pub(crate) bit_0_count: u32,
    pub(crate) bit_count: u32,
    pub(crate) bit_0_prob: u32,
    pub(crate) bits_until_update: u32,
    pub(crate) update_cycle: u32,
}

impl ArithmeticBitModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self) {
        // halve counts when a threshold is reached
        self.bit_count += self.update_cycle;
        if self.bit_count > BM_MAX_COUNT {
            self.bit_count = (self.bit_count + 1) >> 1;
            self.bit_0_count = (self.bit_0_count + 1) >> 1;

            if self.bit_0_count == self.bit_count {
                self.bit_count += 1;
            }
        }

        // compute scaled bit 0 probability
        let scale = 0x80000000u32 / self.bit_count;
        self.bit_0_prob = (self.bit_0_count * scale) >> (31 - BM_LENGTH_SHIFT);

        // set frequency of model updates
        self.update_cycle = (5 * self.update_cycle) >> 2;
        if self.update_cycle > 64 {
            self.update_cycle = 64;
        }
        self.bits_until_update = self.update_cycle;
    }
}

impl Default for ArithmeticBitModel {
    fn default() -> Self {
        // initialization to equiprobable model
        Self {
            bit_0_count: 1,
            bit_count: 2,
            bit_0_prob: 1u32 << (BM_LENGTH_SHIFT - 1),
            // start with frequent updates
            bits_until_update: 4,
            update_cycle: 4,
        }
    }
}

pub struct ArithmeticModelBuilder<'a> {
    symbols: u32,
    compress: bool,
    init_table: &'a [u32],
}

impl<'a> ArithmeticModelBuilder<'a> {
    pub fn new(symbols: u32) -> Self {
        Self {
            symbols,
            compress: false,
            init_table: &[],
        }
    }

    pub fn build(self) -> ArithmeticModel {
        ArithmeticModel::new(self.symbols, self.compress, self.init_table)
    }
}
