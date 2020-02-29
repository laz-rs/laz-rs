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

//! Packing types from / to bytes

/// Definition of the packing & unpacking trait
///
/// Types that can be packed and unpacked from byte slices.
///
/// This trait allows to have something that seems a bit faster
/// that using a std::io::Cursor + the byteorder crate.
pub trait Packable {
    fn unpack_from(input: &[u8]) -> Self;
    fn pack_into(&self, output: &mut [u8]);

    unsafe fn unpack_from_unchecked(input: &[u8]) -> Self;
    unsafe fn pack_into_unchecked(&self, output: &mut [u8]);
}

impl Packable for u32 {
    fn unpack_from(input: &[u8]) -> Self {
        assert!(input.len() >= 4, "u32::unpack_from expected a slice of 4 bytes");
        unsafe { Self::unpack_from_unchecked(input) }
    }

    fn pack_into(&self, output: &mut [u8]) {
        assert!(output.len() >= 4, "u32::pack_into expected a slice of 4 bytes");
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
        output.get_unchecked_mut(..4).copy_from_slice(&self.to_le_bytes())
    }
}

impl Packable for u16 {
    fn unpack_from(input: &[u8]) -> Self {
        assert!(input.len() >= 2, "u16::unpack_from expected a slice of 2 bytes");
        unsafe { Self::unpack_from_unchecked(input) }
    }

    fn pack_into(&self, output: &mut [u8]) {
        assert!(output.len() >= 2, "u32::pack_into expected a slice of 4 bytes");
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
        output.get_unchecked_mut(..2).copy_from_slice(&self.to_le_bytes());
    }
}

impl Packable for u8 {
    fn unpack_from(input: &[u8]) -> Self {
        input[0]
    }

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
    fn unpack_from(input: &[u8]) -> Self {
        u32::unpack_from(input) as i32
    }

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
    fn unpack_from(input: &[u8]) -> Self {
        u16::unpack_from(input) as i16
    }

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
    fn unpack_from(input: &[u8]) -> Self {
        input[0] as i8
    }

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_packer() {
        let in_val: i32 = -25;
        let mut buf = [0u8; std::mem::size_of::<i32>()];
        in_val.pack_into(&mut buf);
        let v = i32::unpack_from(&buf);
        assert_eq!(v, in_val);
    }
}
