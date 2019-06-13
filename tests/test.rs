use std::convert::TryInto;
use std::io::{Cursor, Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use i32;
use laz::formats::{
    IntegerFieldCompressor, IntegerFieldDecompressor, RecordCompressor, RecordDecompressor,
};

unsafe fn to_slice<T>(value: &T) -> &[u8] {
    let p: *const T = value;
    let bp: *const u8 = p as *const _;
    std::slice::from_raw_parts(bp, std::mem::size_of::<T>())
}

#[test]
fn test_i32_compression_decompression() {
    let mut compressor = RecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(IntegerFieldCompressor::<i32>::new());

    let n = 20000i32;
    for i in 0..n {
        let slc = unsafe { to_slice(&i) };
        compressor.compress(&slc).unwrap();
    }
    compressor.done().unwrap();

    let compressed_data = compressor.into_stream().into_inner();

    let mut decompressor = RecordDecompressor::new(Cursor::new(compressed_data));
    decompressor.add_field(IntegerFieldDecompressor::<i32>::new());

    for i in 0..n {
        let mut buf = [0u8; std::mem::size_of::<i32>()];
        decompressor.decompress(&mut buf).unwrap();

        let val = i32::from_ne_bytes(buf.try_into().unwrap());
        assert_eq!(val, i);
    }
}

#[test]
fn test_u32_compression_decompression() {
    let mut compressor = RecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(IntegerFieldCompressor::<u32>::new());

    let n = 20000u32;
    for i in 0..n {
        let slc = unsafe { to_slice(&i) };
        compressor.compress(&slc).unwrap();
    }
    compressor.done().unwrap();

    let compressed_data = compressor.into_stream().into_inner();

    let mut decompressor = RecordDecompressor::new(Cursor::new(compressed_data));
    decompressor.add_field(IntegerFieldDecompressor::<u32>::new());

    for i in 0..n {
        let mut buf = [0u8; std::mem::size_of::<u32>()];
        decompressor.decompress(&mut buf).unwrap();

        let val = u32::from_ne_bytes(buf.try_into().unwrap());
        assert_eq!(val, i);
    }
}

#[test]
fn test_compress_decompress_simple_struct() {
    struct MyPoint {
        a: i32,
        b: i16,
    }

    impl MyPoint {
        fn write_to<W: Write>(&self, dst: &mut W) {
            dst.write_i32::<LittleEndian>(self.a).unwrap();
            dst.write_i16::<LittleEndian>(self.b).unwrap();
        }

        fn read_from<R: Read>(src: &mut R) -> Self {
            Self {
                a: src.read_i32::<LittleEndian>().unwrap(),
                b: src.read_i16::<LittleEndian>().unwrap(),
            }
        }
    }

    let mut compressor = RecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(IntegerFieldCompressor::<i32>::new());
    compressor.add_field_compressor(IntegerFieldCompressor::<i16>::new());

    let mut buf = vec![0u8; 4 + 2];
    for i in 0..1000 {
        let mut cursor = Cursor::new(&mut buf);
        let p = MyPoint {
            a: i + 50000,
            b: (i + 1000) as i16,
        };
        p.write_to(&mut cursor);
        compressor.compress(&buf).unwrap();
    }
    compressor.done().unwrap();

    let compressed_data = compressor.into_stream().into_inner();

    let mut decompressor = RecordDecompressor::new(Cursor::new(compressed_data));
    decompressor.add_field(IntegerFieldDecompressor::<i32>::new());
    decompressor.add_field(IntegerFieldDecompressor::<i16>::new());

    let mut buf = [0u8; 6];
    for i in 0..1000 {
        decompressor.decompress(&mut buf).unwrap();
        let mut cursor = Cursor::new(&mut buf);
        let p = MyPoint::read_from(&mut cursor);
        assert_eq!(p.a, i + 50000);
        assert_eq!(p.b, (i + 1000) as i16);
    }
}
