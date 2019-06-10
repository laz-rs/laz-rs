use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};

use laz::las::laszip::{
    LasZipCompressor, LasZipDecompressor, LazItemRecordBuilder, LazItemType, LazVlr,
};

const LAS_HEADER_SIZE: u64 = 227;
const VLR_HEADER_SIZE: u64 = 54;
const OFFSET_TO_LASZIP_VLR_DATA: u64 = LAS_HEADER_SIZE + VLR_HEADER_SIZE;
//const SIZEOF_CHUNK_TABLE: i64 = 8;
const NUM_POINTS: usize = 1065;

fn assert_buffer_eq(buf1: &[u8], buf2: &[u8]) {
    // can't directly use assert_eq! on the buffers as Debug & Eq are not impl for [0u8; 61]
    // guess they will be when Rust's const generics are
    for (b, expected_b) in buf1.iter().zip(buf2.iter()) {
        assert_eq!(b, expected_b, "lol");
    }
}

#[test]
fn test_point_format_0_loop() {
    let mut laz_file = File::open("tests/data/point10.laz").unwrap();

    laz_file
        .seek(SeekFrom::Start(OFFSET_TO_LASZIP_VLR_DATA))
        .unwrap();
    let laz_vlr = LazVlr::read_from(&mut laz_file).unwrap();

    // Seek over chunk table offset
    laz_file
        .seek(SeekFrom::Current(std::mem::size_of::<u64>() as i64))
        .unwrap();

    let mut las_file = File::open("tests/data/point10.las").unwrap();
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();

    let mut buf = [0u8; 20];
    let mut expected_buff = [0u8; 20];

    let mut decompressor = LasZipDecompressor::new(&mut laz_file, laz_vlr);
    let mut compressor = LasZipCompressor::from_laz_items(
        Cursor::new(Vec::<u8>::new()),
        LazItemRecordBuilder::new()
            .add_item(LazItemType::Point10)
            .build(),
    );

    let mut my_laz_vlr = Cursor::new(Vec::<u8>::with_capacity(52));
    compressor.vlr().write_to(&mut my_laz_vlr).unwrap();

    for _ in 0..NUM_POINTS {
        decompressor.decompress_one(&mut buf);
        las_file.read_exact(&mut expected_buff).unwrap();
        assert_eq!(expected_buff, buf);

        compressor.compress_one(&expected_buff);
    }
    compressor.done();

    let mut compression_output = compressor.into_stream();

    compression_output.set_position(0);
    my_laz_vlr.set_position(0);
    let my_laz_vlr = LazVlr::read_from(&mut my_laz_vlr).unwrap();

    //compression_output.seek(SeekFrom::Start(std::mem::size_of::<u64>() as u64)); // skip offset to chunk table
    let mut decompressor = LasZipDecompressor::new(&mut compression_output, my_laz_vlr);

    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    for _ in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        decompressor.decompress_one(&mut buf);

        assert_eq!(expected_buff, buf);
    }
}

#[test]
fn test_point_format_1_loop() {
    let mut laz_file = File::open("tests/data/point-time.laz").unwrap();

    laz_file
        .seek(SeekFrom::Start(OFFSET_TO_LASZIP_VLR_DATA))
        .unwrap();
    let laz_vlr = LazVlr::read_from(&mut laz_file).unwrap();

    // Seek over chunk table offset
    laz_file
        .seek(SeekFrom::Current(std::mem::size_of::<u64>() as i64))
        .unwrap();

    let mut las_file = File::open("tests/data/point-time.las").unwrap();
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();

    let mut buf = [0u8; 28];
    let mut expected_buff = [0u8; 28];

    let mut decompressor = LasZipDecompressor::new(&mut laz_file, laz_vlr);
    let mut compressor = LasZipCompressor::from_laz_items(
        Cursor::new(Vec::<u8>::new()),
        LazItemRecordBuilder::new()
            .add_item(LazItemType::Point10)
            .add_item(LazItemType::GpsTime)
            .build(),
    );

    let mut my_laz_vlr = Cursor::new(Vec::<u8>::with_capacity(52));
    compressor.vlr().write_to(&mut my_laz_vlr).unwrap();

    for _ in 0..NUM_POINTS {
        decompressor.decompress_one(&mut buf);
        las_file.read_exact(&mut expected_buff).unwrap();
        assert_eq!(expected_buff, buf);

        compressor.compress_one(&expected_buff);
    }
    compressor.done();

    let mut compression_output = compressor.into_stream();

    compression_output.set_position(0);
    my_laz_vlr.set_position(0);
    let my_laz_vlr = LazVlr::read_from(&mut my_laz_vlr).unwrap();

    //compression_output.seek(SeekFrom::Start(std::mem::size_of::<u64>() as u64)); // skip offset to chunk table
    let mut decompressor = LasZipDecompressor::new(&mut compression_output, my_laz_vlr);

    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    for _ in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        decompressor.decompress_one(&mut buf);
        assert_eq!(expected_buff, buf);
    }
}

#[test]
fn test_point_format_2_loop() {
    let mut laz_file = File::open("tests/data/point-color.laz").unwrap();

    laz_file
        .seek(SeekFrom::Start(OFFSET_TO_LASZIP_VLR_DATA))
        .unwrap();
    let laz_vlr = LazVlr::read_from(&mut laz_file).unwrap();

    // Seek over chunk table offset
    laz_file
        .seek(SeekFrom::Current(std::mem::size_of::<u64>() as i64))
        .unwrap();

    let mut las_file = File::open("tests/data/point-color.las").unwrap();
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();

    let mut buf = [0u8; 26];
    let mut expected_buff = [0u8; 26];

    let mut decompressor = LasZipDecompressor::new(&mut laz_file, laz_vlr);
    let mut compressor = LasZipCompressor::from_laz_items(
        Cursor::new(Vec::<u8>::new()),
        LazItemRecordBuilder::new()
            .add_item(LazItemType::Point10)
            .add_item(LazItemType::RGB12)
            .build(),
    );

    let mut my_laz_vlr = Cursor::new(Vec::<u8>::with_capacity(52));
    compressor.vlr().write_to(&mut my_laz_vlr).unwrap();

    for _ in 0..NUM_POINTS {
        decompressor.decompress_one(&mut buf);
        las_file.read_exact(&mut expected_buff).unwrap();
        assert_eq!(expected_buff, buf);

        compressor.compress_one(&expected_buff);
    }
    compressor.done();

    let mut compression_output = compressor.into_stream();

    compression_output.set_position(0);
    my_laz_vlr.set_position(0);
    let my_laz_vlr = LazVlr::read_from(&mut my_laz_vlr).unwrap();

    //compression_output.seek(SeekFrom::Start(std::mem::size_of::<u64>() as u64)); // skip offset to chunk table
    let mut decompressor = LasZipDecompressor::new(&mut compression_output, my_laz_vlr);

    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    for _ in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        decompressor.decompress_one(&mut buf);
        assert_eq!(expected_buff, buf);
    }
}

#[test]
fn test_point_format_3_loop() {
    let mut laz_file = File::open("tests/data/point-color-time.laz").unwrap();

    laz_file
        .seek(SeekFrom::Start(OFFSET_TO_LASZIP_VLR_DATA))
        .unwrap();
    let laz_vlr = LazVlr::read_from(&mut laz_file).unwrap();

    // Seek over chunk table offset
    laz_file
        .seek(SeekFrom::Current(std::mem::size_of::<u64>() as i64))
        .unwrap();

    let mut las_file = File::open("tests/data/point-color-time.las").unwrap();
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();

    let mut buf = [0u8; 34];
    let mut expected_buff = [0u8; 34];

    let mut decompressor = LasZipDecompressor::new(&mut laz_file, laz_vlr);
    let mut compressor = LasZipCompressor::from_laz_items(
        Cursor::new(Vec::<u8>::new()),
        LazItemRecordBuilder::new()
            .add_item(LazItemType::Point10)
            .add_item(LazItemType::GpsTime)
            .add_item(LazItemType::RGB12)
            .build(),
    );

    let mut my_laz_vlr = Cursor::new(Vec::<u8>::with_capacity(52));
    compressor.vlr().write_to(&mut my_laz_vlr).unwrap();

    for _ in 0..NUM_POINTS {
        decompressor.decompress_one(&mut buf);
        las_file.read_exact(&mut expected_buff).unwrap();
        assert_buffer_eq(&expected_buff, &buf);

        compressor.compress_one(&expected_buff);
    }
    compressor.done();

    let mut compression_output = compressor.into_stream();

    compression_output.set_position(0);
    my_laz_vlr.set_position(0);
    let my_laz_vlr = LazVlr::read_from(&mut my_laz_vlr).unwrap();

    //compression_output.seek(SeekFrom::Start(std::mem::size_of::<u64>() as u64)); // skip offset to chunk table
    let mut decompressor = LasZipDecompressor::new(&mut compression_output, my_laz_vlr);

    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    for _ in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        decompressor.decompress_one(&mut buf);

        assert_buffer_eq(&expected_buff, &buf);
    }
}

#[test]
fn test_point_format_3_with_extra_bytes_loop() {
    let mut laz_file = File::open("tests/data/extra-bytes.laz").unwrap();

    laz_file.seek(SeekFrom::Start(1295)).unwrap();
    let laz_vlr = LazVlr::read_from(&mut laz_file).unwrap();

    // Seek over chunk table offset
    laz_file
        .seek(SeekFrom::Current(std::mem::size_of::<u64>() as i64))
        .unwrap();

    let mut las_file = File::open("tests/data/extra-bytes.las").unwrap();
    // Again, account for the extra bytes vlr
    las_file
        .seek(SeekFrom::Start(
            LAS_HEADER_SIZE + VLR_HEADER_SIZE + (5 * 192),
        ))
        .unwrap();

    let mut buf = [0u8; 61];
    let mut expected_buff = [0u8; 61];

    let mut decompressor = LasZipDecompressor::new(&mut laz_file, laz_vlr);
    let mut compressor = LasZipCompressor::from_laz_items(
        Cursor::new(Vec::<u8>::new()),
        LazItemRecordBuilder::new()
            .add_item(LazItemType::Point10)
            .add_item(LazItemType::GpsTime)
            .add_item(LazItemType::RGB12)
            .add_item(LazItemType::Byte(27))
            .build(),
    );

    let mut my_laz_vlr = Cursor::new(Vec::<u8>::with_capacity(52));
    compressor.vlr().write_to(&mut my_laz_vlr).unwrap();

    for _ in 0..NUM_POINTS {
        decompressor.decompress_one(&mut buf);
        las_file.read_exact(&mut expected_buff).unwrap();
        assert_buffer_eq(&expected_buff, &buf);

        compressor.compress_one(&expected_buff);
    }
    compressor.done();

    let mut compression_output = compressor.into_stream();

    compression_output.set_position(0);
    my_laz_vlr.set_position(0);
    let my_laz_vlr = LazVlr::read_from(&mut my_laz_vlr).unwrap();

    //compression_output.seek(SeekFrom::Start(std::mem::size_of::<u64>() as u64)); // skip offset to chunk table
    let mut decompressor = LasZipDecompressor::new(&mut compression_output, my_laz_vlr);

    las_file
        .seek(SeekFrom::Start(
            LAS_HEADER_SIZE + VLR_HEADER_SIZE + (5 * 192),
        ))
        .unwrap();

    for _ in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        decompressor.decompress_one(&mut buf);

        assert_buffer_eq(&expected_buff, &buf);
    }
}
