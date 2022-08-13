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

use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::ops::{BitAnd, BitXor};

use num_traits::Zero;

use crate::decoders::ArithmeticDecoder;
use crate::encoders::ArithmeticEncoder;
use crate::packers::Packable;

#[derive(Copy, Clone)]
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
        unsafe {
            if self.high {
                if v < *self.values.get_unchecked(2) {
                    *self.values.get_unchecked_mut(4) = *self.values.get_unchecked(3);
                    *self.values.get_unchecked_mut(3) = *self.values.get_unchecked(2);
                    if v < *self.values.get_unchecked(0) {
                        *self.values.get_unchecked_mut(2) = *self.values.get_unchecked(1);
                        *self.values.get_unchecked_mut(1) = *self.values.get_unchecked(0);
                        *self.values.get_unchecked_mut(0) = v;
                    } else if v < *self.values.get_unchecked(1) {
                        *self.values.get_unchecked_mut(2) = *self.values.get_unchecked(1);
                        *self.values.get_unchecked_mut(1) = v;
                    } else {
                        *self.values.get_unchecked_mut(2) = v;
                    }
                } else {
                    if v < *self.values.get_unchecked(3) {
                        *self.values.get_unchecked_mut(4) = *self.values.get_unchecked(3);
                        *self.values.get_unchecked_mut(3) = v;
                    } else {
                        *self.values.get_unchecked_mut(4) = v;
                    }
                    self.high = false;
                }
            } else {
                if *self.values.get_unchecked(2) < v {
                    *self.values.get_unchecked_mut(0) = *self.values.get_unchecked(1);
                    *self.values.get_unchecked_mut(1) = *self.values.get_unchecked(2);
                    if *self.values.get_unchecked(4) < v {
                        *self.values.get_unchecked_mut(2) = *self.values.get_unchecked(3);
                        *self.values.get_unchecked_mut(3) = *self.values.get_unchecked(4);
                        *self.values.get_unchecked_mut(4) = v;
                    } else if *self.values.get_unchecked(3) < v {
                        *self.values.get_unchecked_mut(2) = *self.values.get_unchecked(3);
                        *self.values.get_unchecked_mut(3) = v;
                    } else {
                        *self.values.get_unchecked_mut(2) = v;
                    }
                } else {
                    if *self.values.get_unchecked(1) < v {
                        *self.values.get_unchecked_mut(0) = *self.values.get_unchecked(1);
                        *self.values.get_unchecked_mut(1) = v;
                    } else {
                        *self.values.get_unchecked_mut(0) = v;
                    }
                    self.high = true;
                }
            }
        }
    }

    pub fn get(&self) -> T {
        unsafe { *self.values.get_unchecked(2) }
    }
}

#[inline]
pub fn flag_diff<T>(value: T, other: T, flag: <T as BitXor>::Output) -> bool
where
    T: BitXor + BitAnd,
    <T as BitXor>::Output: BitAnd,
    <<T as BitXor>::Output as BitAnd>::Output: PartialEq<u16>,
{
    ((value ^ other) & flag) != 0u16
}

#[inline]
pub(crate) fn u32_zero_bit(n: u32) -> u32 {
    n & 0xFF_FF_FF_FEu32
}

#[inline]
pub(crate) fn u8_clamp(n: i32) -> u8 {
    use num_traits::clamp;
    clamp(n, i32::from(std::u8::MIN), i32::from(std::u8::MAX)) as u8
}

#[inline(always)]
pub(crate) fn lower_byte(n: u16) -> u8 {
    (n & 0x00_FF) as u8
}

#[inline(always)]
pub(crate) fn upper_byte(n: u16) -> u8 {
    (n >> 8) as u8
}

#[inline(always)]
pub(crate) fn lower_byte_changed(lhs: u16, rhs: u16) -> bool {
    lower_byte(lhs) != lower_byte(rhs)
}

#[inline(always)]
pub(crate) fn upper_byte_changed(lhs: u16, rhs: u16) -> bool {
    upper_byte(lhs) != upper_byte(rhs)
}

#[inline]
pub fn i32_quantize(n: f32) -> i32 {
    if n >= 0.0f32 {
        (n + 0.5f32) as i32
    } else {
        (n - 0.5f32) as i32
    }
}

#[inline]
pub(crate) fn copy_bytes_into_decoder<R: Read + Seek>(
    is_requested: bool,
    num_bytes: usize,
    decoder: &mut ArithmeticDecoder<Cursor<Vec<u8>>>,
    src: &mut R,
) -> std::io::Result<bool> {
    let inner_vec = decoder.get_mut().get_mut();
    if is_requested {
        if num_bytes > 0 {
            inner_vec.resize(num_bytes, 0);
            src.read_exact(&mut inner_vec[..num_bytes])?;
            decoder.read_init_bytes()?;
            Ok(true)
        } else {
            inner_vec.resize(0, 0);
            Ok(false)
        }
    } else {
        if num_bytes > 0 {
            src.seek(SeekFrom::Current(num_bytes as i64))?;
        }
        Ok(false)
    }
}

pub(crate) fn inner_buffer_len_of(encoder: &ArithmeticEncoder<Cursor<Vec<u8>>>) -> usize {
    encoder.get_ref().get_ref().len()
}

#[inline]
pub(crate) fn copy_encoder_content_to<W: Write>(
    encoder: &mut ArithmeticEncoder<Cursor<Vec<u8>>>,
    dst: &mut W,
) -> std::io::Result<()> {
    dst.write_all(encoder.get_mut().get_ref())
}

#[inline(always)]
pub(crate) fn read_and_unpack<R: Read, P: Packable>(
    src: &mut R,
    buf: &mut [u8],
) -> std::io::Result<P> {
    src.read_exact(buf)?;
    Ok(P::unpack_from(buf))
}

macro_rules! is_nth_bit_set {
    ($sym:expr, $n:expr) => {
        ($sym & (1 << $n)) != 0
    };
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
    [8, 9, 10, 11, 12, 13, 14, 15],
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
    [7, 6, 5, 4, 3, 2, 1, 0],
];
// for LAS points with correctly populated return numbers (1 <= r <= n) and
// number of returns of given pulse (1 <= n <= 15) the return mapping that
// serializes the possible combinations into one number should be the following
//
//  { ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, --- },
//  { ---,   0, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, --- },
//  { ---,   1,   2, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, --- },
//  { ---,   3,   4,   5, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, --- },
//  { ---,   6,   7,   8,   9, ---, ---, ---, ---, ---, ---, ---, ---, ---, ---, --- },
//  { ---,  10,  11,  12,  13,  14, ---, ---, ---, ---, ---, ---, ---, ---, ---, --- },
//  { ---,  15,  16,  17,  18,  19,  20, ---, ---, ---, ---, ---, ---, ---, ---, --- },
//  { ---,  21,  22,  23,  24,  25,  26,  27, ---, ---, ---, ---, ---, ---, ---, --- },
//  { ---,  28,  29,  30,  31,  32,  33,  34,  35, ---, ---, ---, ---, ---, ---, --- },
//  { ---,  36,  37,  38,  39,  40,  41,  42,  43,  44, ---, ---, ---, ---, ---, --- },
//  { ---,  45,  46,  47,  48,  49,  50,  51,  52,  53,  54, ---, ---, ---, ---, --- },
//  { ---,  55,  56,  57,  58,  59,  60,  61,  62,  63,  64,  65, ---, ---, ---, --- },
//  { ---,  66,  67,  68,  69,  70,  71,  72,  73,  74,  75,  76,  77, ---, ---, --- },
//  { ---,  78,  89,  80,  81,  82,  83,  84,  85,  86,  87,  88,  89,  90, ---, --- },
//  { ---,  91,  92,  93,  94,  95,  96,  97,  98,  99, 100, 101, 102, 103, 104, --- },
//  { ---, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119 }
//
// we drastically simplify the number of return combinations that we want to distinguish
// down to 16 as higher returns will not have significant entropy differences
//
//  { --, --, --, --, --, --, --, --, --, --, --, --, --, --, --, -- },
//  { --,  0, --, --, --, --, --, --, --, --, --, --, --, --, --, -- },
//  { --,  1,  2, --, --, --, --, --, --, --, --, --, --, --, --, -- },
//  { --,  3,  4,  5, --, --, --, --, --, --, --, --, --, --, --, -- },
//  { --,  6,  7,  8,  9, --, --, --, --, --, --, --, --, --, --, -- },
//  { --, 10, 11, 12, 13, 14, --, --, --, --, --, --, --, --, --, -- },
//  { --, 10, 11, 12, 13, 14, 15, --, --, --, --, --, --, --, --, -- },
//  { --, 10, 11, 12, 12, 13, 14, 15, --, --, --, --, --, --, --, -- },
//  { --, 10, 11, 12, 12, 13, 13, 14, 15, --, --, --, --, --, --, -- },
//  { --, 10, 11, 11, 12, 12, 13, 13, 14, 15, --, --, --, --, --, -- },
//  { --, 10, 11, 11, 12, 12, 13, 13, 14, 14, 15, --, --, --, --, -- },
//  { --, 10, 10, 11, 11, 12, 12, 13, 13, 14, 14, 15, --, --, --, -- },
//  { --, 10, 10, 11, 11, 12, 12, 13, 13, 14, 14, 15, 15, --, --, -- },
//  { --, 10, 10, 11, 11, 12, 12, 12, 13, 13, 14, 14, 15, 15, --, -- },
//  { --, 10, 10, 11, 11, 12, 12, 12, 13, 13, 13, 14, 14, 15, 15, -- },
//  { --, 10, 10, 11, 11, 12, 12, 12, 13, 13, 13, 14, 14, 14, 15, 15 }

// however, as some files start the numbering of r and n with 0, only have return counts
// r, only have number of return per pulse n, or mix up position of r and n, we complete
// the table to also map those "undesired" r and n combinations to different contexts
/*
const U8 number_return_map_4bit[16][16] =
{
  { 15, 14, 13, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0 },
  { 14,  0,  1,  3,  6, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10 },
  { 13,  1,  2,  4,  7, 11, 11, 11, 11, 11, 11, 10, 10, 10, 10, 10 },
  { 12,  3,  4,  5,  8, 12, 12, 12, 12, 11, 11, 11, 11, 11, 11, 11 },
  { 11,  6,  7,  8,  9, 13, 13, 12, 12, 12, 12, 11, 11, 11, 11, 11 },
  { 10, 10, 11, 12, 13, 14, 14, 13, 13, 12, 12, 12, 12, 12, 12, 12 },
  {  9, 10, 11, 12, 13, 14, 15, 14, 13, 13, 13, 12, 12, 12, 12, 12 },
  {  8, 10, 11, 12, 12, 13, 14, 15, 14, 13, 13, 13, 13, 12, 12, 12 },
  {  7, 10, 11, 12, 12, 13, 13, 14, 15, 14, 14, 13, 13, 13, 13, 13 },
  {  6, 10, 11, 11, 12, 12, 13, 13, 14, 15, 14, 14, 14, 13, 13, 13 },
  {  5, 10, 11, 11, 12, 12, 13, 13, 14, 14, 15, 14, 14, 14, 13, 13 },
  {  4, 10, 10, 11, 11, 12, 12, 13, 13, 14, 14, 15, 15, 14, 14, 14 },
  {  3, 10, 10, 11, 11, 12, 12, 13, 13, 14, 14, 15, 15, 15, 14, 14 },
  {  2, 10, 10, 11, 11, 12, 12, 12, 13, 13, 14, 14, 15, 15, 15, 14 },
  {  1, 10, 10, 11, 11, 12, 12, 12, 13, 13, 13, 14, 14, 15, 15, 15 },
  {  0, 10, 10, 11, 11, 12, 12, 12, 13, 13, 13, 14, 14, 14, 15, 15 }
};
// simplify down to 10 contexts
const U8 number_return_map_10ctx[16][16] =
{
  {  0,  1,  2,  3,  4,  5,  6,  7,  8,  9,  9,  9,  9,  9,  9,  9 },
  {  1,  0,  1,  3,  6,  6,  6,  6,  6,  6,  6,  6,  6,  6,  6,  6 },
  {  2,  1,  2,  4,  7,  7,  7,  7,  7,  7,  7,  6,  6,  6,  6,  6 },
  {  3,  3,  4,  5,  8,  7,  7,  7,  7,  7,  7,  7,  7,  7,  7,  7 },
  {  4,  6,  7,  8,  9,  8,  8,  7,  7,  7,  7,  7,  7,  7,  7,  7 },
  {  5,  6,  7,  7,  8,  9,  8,  8,  8,  7,  7,  7,  7,  7,  7,  7 },
  {  6,  6,  7,  7,  8,  8,  9,  8,  8,  8,  8,  7,  7,  7,  7,  7 },
  {  7,  6,  7,  7,  7,  8,  8,  9,  8,  8,  8,  8,  8,  7,  7,  7 },
  {  8,  6,  7,  7,  7,  8,  8,  8,  9,  8,  8,  8,  8,  8,  8,  8 },
  {  9,  6,  7,  7,  7,  7,  8,  8,  8,  9,  8,  8,  8,  8,  8,  8 },
  {  9,  6,  7,  7,  7,  7,  8,  8,  8,  8,  9,  8,  8,  8,  8,  8 },
  {  9,  6,  6,  7,  7,  7,  7,  8,  8,  8,  8,  9,  9,  8,  8,  8 },
  {  9,  6,  6,  7,  7,  7,  7,  8,  8,  8,  8,  9,  9,  9,  8,  8 },
  {  9,  6,  6,  7,  7,  7,  7,  7,  8,  8,  8,  8,  9,  9,  9,  8 },
  {  9,  6,  6,  7,  7,  7,  7,  7,  8,  8,  8,  8,  8,  9,  9,  9 },
  {  9,  6,  6,  7,  7,  7,  7,  7,  8,  8,  8,  8,  8,  8,  9,  9 }
};
// simplify even further down to 6 contexts
*/
pub const NUMBER_RETURN_MAP_6CTX: [[u8; 16]; 16] = [
    [0, 1, 2, 3, 4, 5, 3, 4, 4, 5, 5, 5, 5, 5, 5, 5],
    [1, 0, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3],
    [2, 1, 2, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3],
    [3, 3, 4, 5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4],
    [4, 3, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4],
    [5, 3, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4],
    [3, 3, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4, 4, 4],
    [4, 3, 4, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4, 4],
    [4, 3, 4, 4, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4],
    [5, 3, 4, 4, 4, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4],
    [5, 3, 4, 4, 4, 4, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4],
    [5, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 4, 4, 4],
    [5, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 4, 4],
    [5, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 4],
    [5, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5],
    [5, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5],
];

// for LAS points with return number (1 <= r <= n) and a number of returns
// of given pulse (1 <= n <= 15) the level of penetration counted in number
// of returns should really simply be n - r with all invalid combinations
// being mapped to 15 like shown below
//
//  {  0, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15 },
//  { 15,  0, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15 },
//  { 15,  1,  0, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15 },
//  { 15,  2,  1,  0, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15 },
//  { 15,  3,  2,  1,  0, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15 },
//  { 15,  4,  3,  2,  1,  0, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15 },
//  { 15,  5,  4,  3,  2,  1,  0, 15, 15, 15, 15, 15, 15, 15, 15, 15 },
//  { 15,  6,  5,  4,  3,  2,  1,  0, 15, 15, 15, 15, 15, 15, 15, 15 }
//  { 15,  7,  6,  5,  4,  3,  2,  1,  0, 15, 15, 15, 15, 15, 15, 15 }
//  { 15,  8,  7,  6,  5,  4,  3,  2,  1,  0, 15, 15, 15, 15, 15, 15 }
//  { 15,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0, 15, 15, 15, 15, 15 }
//  { 15, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0, 15, 15, 15, 15 }
//  { 15, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0, 15, 15, 15 }
//  { 15, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0, 15, 15 }
//  { 15, 13, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0, 15 }
//  { 15, 14, 13, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0 }
//
// however, some files start the numbering of r and n with 0, only have
// return counts r, or only have number of returns of given pulse n, or
// mix up the position of r and n. we therefore "complete" the table to
// also map those "undesired" r & n combinations to different contexts.
//
// We also stop the enumeration of the levels of penetration at 7 and
// map all higher penetration levels also to 7 in order to keep the total
// number of contexts reasonably small.
//
/*
const U8 number_return_level_4bit[16][16] =
{
  {  0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15 },
  {  1,  0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14 },
  {  2,  1,  0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13 },
  {  3,  2,  1,  0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12 },
  {  4,  3,  2,  1,  0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11 },
  {  5,  4,  3,  2,  1,  0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10 },
  {  6,  5,  4,  3,  2,  1,  0,  1,  2,  3,  4,  5,  6,  7,  8,  9 },
  {  7,  6,  5,  4,  3,  2,  1,  0,  1,  2,  3,  4,  5,  6,  7,  8 },
  {  8,  7,  6,  5,  4,  3,  2,  1,  0,  1,  2,  3,  4,  5,  6,  7 },
  {  9,  8,  7,  6,  5,  4,  3,  2,  1,  0,  1,  2,  3,  4,  5,  6 },
  { 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0,  1,  2,  3,  4,  5 },
  { 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0,  1,  2,  3,  4 },
  { 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0,  1,  2,  3 },
  { 13, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0,  1,  2 },
  { 14, 13, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0,  1 },
  { 15, 14, 13, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0 }
};
*/
// simplify down to 8 contexts
pub const NUMBER_RETURN_LEVEL_8CT: [[u8; 16]; 16] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 7, 7, 7, 7, 7, 7, 7, 7],
    [1, 0, 1, 2, 3, 4, 5, 6, 7, 7, 7, 7, 7, 7, 7, 7],
    [2, 1, 0, 1, 2, 3, 4, 5, 6, 7, 7, 7, 7, 7, 7, 7],
    [3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7, 7, 7, 7, 7, 7],
    [4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7, 7, 7, 7, 7],
    [5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7, 7, 7, 7],
    [6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7, 7, 7],
    [7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7, 7],
    [7, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7],
    [7, 7, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6],
    [7, 7, 7, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5],
    [7, 7, 7, 7, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4],
    [7, 7, 7, 7, 7, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3],
    [7, 7, 7, 7, 7, 7, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2],
    [7, 7, 7, 7, 7, 7, 7, 7, 6, 5, 4, 3, 2, 1, 0, 1],
    [7, 7, 7, 7, 7, 7, 7, 7, 7, 6, 5, 4, 3, 2, 1, 0],
];
