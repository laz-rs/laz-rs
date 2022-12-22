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

#![doc(hidden)]
//! Packing types from / to bytes

use std::mem::size_of;

/// Definition of the packing & unpacking trait
///
/// Types that can be packed and unpacked from byte slices.
///
/// This trait allows to have something that seems a bit faster
/// that using a std::io::Cursor + the byteorder crate.
///
/// # Important
///
/// The byteorder is LittleEndian as this is the byte-order
/// used throughout the LAS Standard.
pub trait Packable {
    fn unpack_from(input: &[u8]) -> Self;
    fn pack_into(&self, output: &mut [u8]);

    unsafe fn unpack_from_unchecked(input: &[u8]) -> Self;
    unsafe fn pack_into_unchecked(&self, output: &mut [u8]);
}

impl Packable for u64 {
    #[inline]
    fn unpack_from(input: &[u8]) -> Self {
        assert!(
            input.len() >= size_of::<Self>(),
            "u64::unpack_from expected a slice of {} bytes",
            size_of::<Self>()
        );
        unsafe { Self::unpack_from_unchecked(input) }
    }

    #[inline]
    fn pack_into(&self, output: &mut [u8]) {
        assert!(
            output.len() >= size_of::<Self>(),
            "u64::pack_into expected a slice of {} bytes",
            size_of::<Self>()
        );
        unsafe { self.pack_into_unchecked(output) }
    }

    #[inline]
    unsafe fn unpack_from_unchecked(input: &[u8]) -> Self {
        let b1 = *input.get_unchecked(0);
        let b2 = *input.get_unchecked(1);
        let b3 = *input.get_unchecked(2);
        let b4 = *input.get_unchecked(3);
        let b5 = *input.get_unchecked(4);
        let b6 = *input.get_unchecked(5);
        let b7 = *input.get_unchecked(6);
        let b8 = *input.get_unchecked(7);

        u64::from_le_bytes([b1, b2, b3, b4, b5, b6, b7, b8])
    }

    #[inline]
    unsafe fn pack_into_unchecked(&self, output: &mut [u8]) {
        output
            .get_unchecked_mut(..8)
            .copy_from_slice(&self.to_le_bytes())
    }
}

impl Packable for u32 {
    #[inline]
    fn unpack_from(input: &[u8]) -> Self {
        assert!(
            input.len() >= 4,
            "u32::unpack_from expected a slice of 4 bytes"
        );
        unsafe { Self::unpack_from_unchecked(input) }
    }

    #[inline]
    fn pack_into(&self, output: &mut [u8]) {
        assert!(
            output.len() >= 4,
            "u32::pack_into expected a slice of 4 bytes"
        );
        unsafe { self.pack_into_unchecked(output) }
    }

    #[inline]
    unsafe fn unpack_from_unchecked(input: &[u8]) -> Self {
        let b1 = *input.get_unchecked(0);
        let b2 = *input.get_unchecked(1);
        let b3 = *input.get_unchecked(2);
        let b4 = *input.get_unchecked(3);

        u32::from_le_bytes([b1, b2, b3, b4])
    }

    #[inline]
    unsafe fn pack_into_unchecked(&self, output: &mut [u8]) {
        output
            .get_unchecked_mut(..4)
            .copy_from_slice(&self.to_le_bytes())
    }
}

impl Packable for u16 {
    #[inline]
    fn unpack_from(input: &[u8]) -> Self {
        assert!(
            input.len() >= 2,
            "u16::unpack_from expected a slice of 2 bytes"
        );
        unsafe { Self::unpack_from_unchecked(input) }
    }

    #[inline]
    fn pack_into(&self, output: &mut [u8]) {
        assert!(
            output.len() >= 2,
            "u32::pack_into expected a slice of 4 bytes"
        );
        unsafe { self.pack_into_unchecked(output) }
    }

    #[inline]
    unsafe fn unpack_from_unchecked(input: &[u8]) -> Self {
        let b1 = *input.get_unchecked(0);
        let b2 = *input.get_unchecked(1);

        u16::from_le_bytes([b1, b2])
    }

    #[inline]
    unsafe fn pack_into_unchecked(&self, output: &mut [u8]) {
        output
            .get_unchecked_mut(..2)
            .copy_from_slice(&self.to_le_bytes());
    }
}

impl Packable for u8 {
    #[inline]
    fn unpack_from(input: &[u8]) -> Self {
        input[0]
    }

    #[inline]
    fn pack_into(&self, output: &mut [u8]) {
        output[0] = *self;
    }

    #[inline]
    unsafe fn unpack_from_unchecked(input: &[u8]) -> Self {
        *input.get_unchecked(0)
    }

    #[inline]
    unsafe fn pack_into_unchecked(&self, output: &mut [u8]) {
        *output.get_unchecked_mut(0) = *self;
    }
}

impl Packable for i32 {
    #[inline]
    fn unpack_from(input: &[u8]) -> Self {
        u32::unpack_from(input) as i32
    }

    #[inline]
    fn pack_into(&self, output: &mut [u8]) {
        (*self as u32).pack_into(output)
    }

    #[inline]
    unsafe fn unpack_from_unchecked(input: &[u8]) -> Self {
        u32::unpack_from_unchecked(input) as i32
    }

    #[inline]
    unsafe fn pack_into_unchecked(&self, output: &mut [u8]) {
        (*self as u32).pack_into_unchecked(output)
    }
}

impl Packable for i16 {
    #[inline]
    fn unpack_from(input: &[u8]) -> Self {
        u16::unpack_from(input) as i16
    }

    #[inline]
    fn pack_into(&self, mut output: &mut [u8]) {
        (*self as u16).pack_into(&mut output)
    }

    #[inline]
    unsafe fn unpack_from_unchecked(input: &[u8]) -> Self {
        u16::unpack_from_unchecked(input) as i16
    }

    #[inline]
    unsafe fn pack_into_unchecked(&self, output: &mut [u8]) {
        (*self as u16).pack_into_unchecked(output)
    }
}

impl Packable for i8 {
    #[inline]
    fn unpack_from(input: &[u8]) -> Self {
        input[0] as i8
    }

    #[inline]
    fn pack_into(&self, output: &mut [u8]) {
        output[0] = *self as u8;
    }

    #[inline]
    unsafe fn unpack_from_unchecked(input: &[u8]) -> Self {
        *input.get_unchecked(0) as i8
    }

    #[inline]
    unsafe fn pack_into_unchecked(&self, output: &mut [u8]) {
        *output.get_unchecked_mut(0) = (*self) as u8;
    }
}

impl Packable for f32 {
    #[inline]
    fn unpack_from(input: &[u8]) -> Self {
        assert!(
            input.len() >= 4,
            "f32::unpack_from expected a slice of 4 bytes"
        );
        unsafe { Self::unpack_from_unchecked(input) }
    }

    #[inline]
    fn pack_into(&self, output: &mut [u8]) {
        assert!(
            output.len() >= 4,
            "f32::pack_into expected a slice of 4 bytes"
        );
        unsafe { self.pack_into_unchecked(output) }
    }

    #[inline]
    unsafe fn unpack_from_unchecked(input: &[u8]) -> Self {
        let b1 = *input.get_unchecked(0);
        let b2 = *input.get_unchecked(1);
        let b3 = *input.get_unchecked(2);
        let b4 = *input.get_unchecked(3);

        f32::from_le_bytes([b1, b2, b3, b4])
    }

    #[inline]
    unsafe fn pack_into_unchecked(&self, output: &mut [u8]) {
        output
            .get_unchecked_mut(..4)
            .copy_from_slice(&self.to_le_bytes())
    }
}
