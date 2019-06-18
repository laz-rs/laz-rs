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

pub trait Packable {
    type Type;

    fn unpack_from(input: &[u8]) -> Self::Type;
    fn pack_into(&self, output: &mut [u8]);
}

impl Packable for u32 {
    type Type = u32;

    fn unpack_from(input: &[u8]) -> Self::Type {
        if input.len() < 4 {
            panic!("u32::unpack_from expected a buffer of 4 bytes");
        }
        unsafe {
            let b1 = *input.get_unchecked(0) as u32;
            let b2 = *input.get_unchecked(1) as u32;
            let b3 = *input.get_unchecked(2) as u32;
            let b4 = *input.get_unchecked(3) as u32;

            b4 << 24 | (b3 & 0xFFF) << 16 | (b2 & 0xFF) << 8 | b1 & 0xFF
        }
    }

    fn pack_into(&self, output: &mut [u8]) {
        if output.len() < 4 {
            panic!("u32::pack_into expected a buffer of 4 bytes");
        }

        unsafe {
            *output.get_unchecked_mut(3) = ((self >> 24) & 0xFFu32) as u8;
            *output.get_unchecked_mut(2) = ((self >> 16) & 0xFFu32) as u8;
            *output.get_unchecked_mut(1) = ((self >> 8) & 0xFFu32) as u8;
            *output.get_unchecked_mut(0) = (self & 0xFFu32) as u8;
        }

    }
}

impl Packable for u16 {
    type Type = u16;

    fn unpack_from(input: &[u8]) -> Self::Type {
        if input.len() < 2 {
            panic!("u16::unpack_from expected a buffer of 2 bytes");
        }
        unsafe {
            let b1 = *input.get_unchecked(0) as u16;
            let b2 = *input.get_unchecked(1) as u16;

            (b2 & 0xFF) << 8 | (b1 & 0xFF)
        }

    }

    fn pack_into(&self, output: &mut [u8]) {
        if output.len() < 2 {
            panic!("u16::pack_into expected a buffer of 2 bytes");
        }
        unsafe {
            *output.get_unchecked_mut(1) = ((self >> 8) & 0xFFu16) as u8;
            *output.get_unchecked_mut(0) = (self & 0xFFu16) as u8
        }
    }
}

impl Packable for u8 {
    type Type = u8;

    fn unpack_from(input: &[u8]) -> Self::Type {
        input[0]
    }

    fn pack_into(&self, output: &mut [u8]) {
        output[0] = *self;
    }
}

impl Packable for i32 {
    type Type = i32;

    fn unpack_from(input: &[u8]) -> Self::Type {
        u32::unpack_from(input) as i32
    }

    fn pack_into(&self, mut output: &mut [u8]) {
        (*self as u32).pack_into(&mut output)
    }
}

impl Packable for i16 {
    type Type = i16;

    fn unpack_from(input: &[u8]) -> Self::Type {
        u16::unpack_from(input) as i16
    }

    fn pack_into(&self, mut output: &mut [u8]) {
        (*self as u16).pack_into(&mut output)
    }
}

impl Packable for i8 {
    type Type = i8;

    fn unpack_from(input: &[u8]) -> Self::Type {
        input[0] as i8
    }

    fn pack_into(&self, output: &mut [u8]) {
        output[0] = *self as u8;
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
    /*
        #[test]
        fn extensive_packer_test() {
            let mut buf = [0u8; std::mem::size_of::<i32>()];
            for i in std::i32::MIN..std::i32::MAX {
                i32::pack(i, &mut buf);
                let v = i32::unpack(&buf);
                assert_eq!(v, i);
            }
        }
    */
}
