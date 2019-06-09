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


use num_traits::Zero;
use std::ops::{BitAnd, BitXor};

#[inline]
pub fn flag_diff<T>(value: T, other: T, flag: <T as BitXor>::Output) -> bool
    where T: BitXor + BitAnd,
          <T as BitXor>::Output: BitAnd,
         <<T as BitXor>::Output as BitAnd>::Output: PartialEq<u16>
{
    ((value ^ other) & flag) != 0u16
}

pub struct StreamingMedian<T: Zero + Copy + PartialOrd> {
    values: [T; 5],
    high: bool,
}

impl<T: Zero + Copy + PartialOrd> StreamingMedian<T> {
    pub fn new() -> Self {
        Self {
            values: [T::zero(); 5],
            high: true,
        }
    }

    pub fn add(&mut self, v: T) {
        if self.high {
            if v < self.values[2] {
                self.values[4] = self.values[3];
                self.values[3] = self.values[2];
                if v < self.values[0] {
                    self.values[2] = self.values[1];
                    self.values[1] = self.values[0];
                    self.values[0] = v;
                } else if v < self.values[1] {
                    self.values[2] = self.values[1];
                    self.values[1] = v;
                } else {
                    self.values[2] = v;
                }
            } else {
                if v < self.values[3] {
                    self.values[4] = self.values[3];
                    self.values[3] = v;
                } else {
                    self.values[4] = v;
                }
                self.high = false;
            }
        } else {
            if self.values[2] < v {
                self.values[0] = self.values[1];
                self.values[1] = self.values[2];
                if self.values[4] < v {
                    self.values[2] = self.values[3];
                    self.values[3] = self.values[4];
                    self.values[4] = v;
                } else if self.values[3] < v {
                    self.values[2] = self.values[3];
                    self.values[3] = v;
                } else {
                    self.values[2] = v;
                }
            } else {
                if self.values[1] < v {
                    self.values[0] = self.values[1];
                    self.values[1] = v;
                } else {
                    self.values[0] = v;
                }
                self.high = true;
            }
        }
    }

    pub fn get(&self) -> T {
        self.values[2]
    }
}

// for LAS files with the return (r) and the number (n) of
// returns field correctly populated the mapping should really
// be only the following.
//  { 15, 15, 15, 15, 15, 15, 15, 15 },
//  { 15,  0, 15, 15, 15, 15, 15, 15 },
//  { 15,  1,  2, 15, 15, 15, 15, 15 },
//  { 15,  3,  4,  5, 15, 15, 15, 15 },
//  { 15,  6,  7,  8,  9, 15, 15, 15 },
//  { 15, 10, 11, 12, 13, 14, 15, 15 },
//  { 15, 15, 15, 15, 15, 15, 15, 15 },
//  { 15, 15, 15, 15, 15, 15, 15, 15 }
// however, some files start the numbering of r and n with 0,
// only have return counts r, or only have number of return
// counts n, or mix up the position of r and n. we therefore
// "complete" the table to also map those "undesired" r & n
// combinations to different contexts
pub const NUMBER_RETURN_MAP: [[u8; 8]; 8] = [
    [15, 14, 13, 12, 11, 10, 9, 8],
    [14, 0, 1, 3, 6, 10, 10, 9],
    [13, 1, 2, 4, 7, 11, 11, 10],
    [12, 3, 4, 5, 8, 12, 12, 11],
    [11, 6, 7, 8, 9, 13, 13, 12],
    [10, 10, 11, 12, 13, 14, 14, 13],
    [9, 10, 11, 12, 13, 14, 15, 14],
    [8, 9, 10, 11, 12, 13, 14, 15]
];

// for LAS files with the return (r) and the number (n) of
// returns field correctly populated the mapping should really
// be only the following.
//  {  0,  7,  7,  7,  7,  7,  7,  7 },
//  {  7,  0,  7,  7,  7,  7,  7,  7 },
//  {  7,  1,  0,  7,  7,  7,  7,  7 },
//  {  7,  2,  1,  0,  7,  7,  7,  7 },
//  {  7,  3,  2,  1,  0,  7,  7,  7 },
//  {  7,  4,  3,  2,  1,  0,  7,  7 },
//  {  7,  5,  4,  3,  2,  1,  0,  7 },
//  {  7,  6,  5,  4,  3,  2,  1,  0 }
// however, some files start the numbering of r and n with 0,
// only have return counts r, or only have number of return
// counts n, or mix up the position of r and n. we therefore
// "complete" the table to also map those "undesired" r & n
// combinations to different contexts
pub const NUMBER_RETURN_LEVEL: [[u8; 8]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7],
    [1, 0, 1, 2, 3, 4, 5, 6],
    [2, 1, 0, 1, 2, 3, 4, 5],
    [3, 2, 1, 0, 1, 2, 3, 4],
    [4, 3, 2, 1, 0, 1, 2, 3],
    [5, 4, 3, 2, 1, 0, 1, 2],
    [6, 5, 4, 3, 2, 1, 0, 1],
    [7, 6, 5, 4, 3, 2, 1, 0]
];

#[inline]
pub fn u32_zero_bit(n: u32) -> u32 {
    n & 0xFFFFFFFEu32
}
