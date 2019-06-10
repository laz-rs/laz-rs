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

    fn unpack(input: &[u8]) -> Self::Type;
    fn pack(value: Self::Type, output: &mut [u8]);
}

impl Packable for u32 {
    type Type = u32;

    fn unpack(input: &[u8]) -> Self::Type {
        let b1 = input[0] as u32;
        let b2 = input[1] as u32;
        let b3 = input[2] as u32;
        let b4 = input[3] as u32;

        b4 << 24 | (b3 & 0xFFF) << 16 | (b2 & 0xFF) << 8 | b1 & 0xFF
    }

    fn pack(value: Self::Type, output: &mut [u8]) {
        output[3] = ((value >> 24) & 0xFF) as u8;
        output[2] = ((value >> 16) & 0xFF) as u8;
        output[1] = ((value >> 8) & 0xFF) as u8;
        output[0] = (value & 0xFF) as u8;
    }
}

impl Packable for u16 {
    type Type = u16;

    fn unpack(input: &[u8]) -> Self::Type {
        let b1 = input[0] as u16;
        let b2 = input[1] as u16;

        (b2 & 0xFF) << 8 | (b1 & 0xFF)
    }

    fn pack(value: Self::Type, output: &mut [u8]) {
        output[1] = ((value >> 8) & 0xFF) as u8;
        output[0] = (value & 0xFF) as u8
    }
}

impl Packable for u8 {
    type Type = u8;

    fn unpack(input: &[u8]) -> Self::Type {
        input[0]
    }

    fn pack(value: Self::Type, output: &mut [u8]) {
        output[0] = value;
    }
}

impl Packable for i32 {
    type Type = i32;

    fn unpack(input: &[u8]) -> Self::Type {
        u32::unpack(input) as i32
    }

    fn pack(value: Self::Type, output: &mut [u8]) {
        u32::pack(value as u32, output)
    }
}

impl Packable for i16 {
    type Type = i16;

    fn unpack(input: &[u8]) -> Self::Type {
        u16::unpack(input) as i16
    }

    fn pack(value: Self::Type, output: &mut [u8]) {
        u16::pack(value as u16, output)
    }
}

impl Packable for i8 {
    type Type = i8;

    fn unpack(input: &[u8]) -> Self::Type {
        input[0] as i8
    }

    fn pack(value: Self::Type, output: &mut [u8]) {
        output[0] = value as u8;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_packer() {
        let in_val: i32 = -25;
        let mut buf = [0u8; std::mem::size_of::<i32>()];
        i32::pack(in_val, &mut buf);
        let v = i32::unpack(&buf);
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
