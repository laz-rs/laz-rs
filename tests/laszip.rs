use std::fs::File;
use std::io::{SeekFrom, Seek, Read};

use laz::las::laszip::{LazVlr, LasZipDecompressor};

const LAS_HEADER_SIZE: u64 = 227;
const VLR_HEADER_SIZE: u64 = 54;
const OFFSET_TO_LASZIP_VLR_DATA: u64 = LAS_HEADER_SIZE + VLR_HEADER_SIZE;
const LASZIP_VLR_DATA_SIZE: u64 = 52;
const SIZEOF_CHUNK_TABLE: i64 = 8;
const FILE_SIZE: u64 = 18217;
const COMPRESSED_POINTS_DATA_SIZE: u64 = FILE_SIZE - (LAS_HEADER_SIZE + VLR_HEADER_SIZE + LASZIP_VLR_DATA_SIZE + SIZEOF_CHUNK_TABLE as u64);
const POINT_SIZE: usize = 34;
const NUM_POINTS: usize = 1065;

#[test]
fn test_point_format_0_decompression() {
    let mut laz_file = File::open("tests/data/point10.laz").unwrap();

    laz_file.seek(SeekFrom::Start(OFFSET_TO_LASZIP_VLR_DATA)).unwrap();
    let laz_vlr = LazVlr::read_from(&mut laz_file).unwrap();

    // Seek over chunk table offset
    laz_file.seek(SeekFrom::Current(std::mem::size_of::<u64>() as i64)).unwrap();


    let mut las_file = File::open("tests/data/point10.las").unwrap();
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();


    let mut decompressor = LasZipDecompressor::new(&mut laz_file, laz_vlr);
    let mut buf = [0u8; 20];
    let mut expected_buff = [0u8; 20];
    for i in 0..NUM_POINTS {
        decompressor.decompress_one(&mut buf);

        las_file.read_exact(&mut expected_buff).unwrap();

        assert_eq!(expected_buff, buf);
    }
}


#[test]
fn test_point_format_2_decompression() {
    let mut laz_file = File::open("tests/data/point-color.laz").unwrap();

    laz_file.seek(SeekFrom::Start(OFFSET_TO_LASZIP_VLR_DATA)).unwrap();
    let laz_vlr = LazVlr::read_from(&mut laz_file).unwrap();

    // Seek over chunk table offset
    laz_file.seek(SeekFrom::Current(std::mem::size_of::<u64>() as i64)).unwrap();


    let mut las_file = File::open("tests/data/point-color.las").unwrap();
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();


    let mut decompressor = LasZipDecompressor::new(&mut laz_file, laz_vlr);
    let mut buf = [0u8; 26];
    let mut expected_buff = [0u8; 26];
    for i in 0..NUM_POINTS {
        decompressor.decompress_one(&mut buf);

        las_file.read_exact(&mut expected_buff).unwrap();

        assert_eq!(expected_buff, buf);
    }
}

#[test]
fn test_point_format_1_decompression() {
    let mut laz_file = File::open("tests/data/point-time.laz").unwrap();

    laz_file.seek(SeekFrom::Start(OFFSET_TO_LASZIP_VLR_DATA)).unwrap();
    let laz_vlr = LazVlr::read_from(&mut laz_file).unwrap();

    // Seek over chunk table offset
    laz_file.seek(SeekFrom::Current(std::mem::size_of::<u64>() as i64)).unwrap();


    let mut las_file = File::open("tests/data/point-time.las").unwrap();
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();


    let mut decompressor = LasZipDecompressor::new(&mut laz_file, laz_vlr);
    let mut buf = [0u8; 28];
    let mut expected_buff = [0u8; 28];
    for i in 0..NUM_POINTS {
        println!("=== {} ===", i);
        decompressor.decompress_one(&mut buf);

        las_file.read_exact(&mut expected_buff).unwrap();

        assert_eq!(expected_buff, buf);
    }
}

#[test]
fn test_point_format_3_decompression() {
    let mut laz_file = File::open("tests/data/point-color-time.laz").unwrap();

    laz_file.seek(SeekFrom::Start(OFFSET_TO_LASZIP_VLR_DATA)).unwrap();
    let laz_vlr = LazVlr::read_from(&mut laz_file).unwrap();

    // Seek over chunk table offset
    laz_file.seek(SeekFrom::Current(std::mem::size_of::<u64>() as i64)).unwrap();


    let mut las_file = File::open("tests/data/point-color-time.las").unwrap();
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();


    let mut decompressor = LasZipDecompressor::new(&mut laz_file, laz_vlr);
    let mut buf = [0u8; 34];
    let mut expected_buff = [0u8; 34];
    for i in 0..NUM_POINTS {
        println!("=== {} ===", i);
        decompressor.decompress_one(&mut buf);

        las_file.read_exact(&mut expected_buff).unwrap();


        // can't directly use assert_eq! on the buffers as Debug & Eq are not impl for [0u8; 34]
        // guess they will be when Rust's const generics are
        for (b, expected_b) in buf.iter().zip(expected_buff.iter()) {
            assert_eq!(b, expected_b);
        }
    }
}

#[test]
fn test_point_format_3_with_extra_bytes_decompression() {
    let mut laz_file = File::open("tests/data/extra-bytes.laz").unwrap();

    // offset is different because of the ExtraBytes VLR
    laz_file.seek(SeekFrom::Start(1295)).unwrap();
    let laz_vlr = LazVlr::read_from(&mut laz_file).unwrap();
    println!("{:?}", laz_vlr);

    // Seek over chunk table offset
    laz_file.seek(SeekFrom::Current(std::mem::size_of::<u64>() as i64)).unwrap();


    let mut las_file = File::open("tests/data/extra-bytes.las").unwrap();
    // Again, account for the extra bytes vlr
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE + VLR_HEADER_SIZE + (5 * 192))).unwrap();


    let mut decompressor = LasZipDecompressor::new(&mut laz_file, laz_vlr);
    let mut buf = [0u8; 61];
    let mut expected_buff = [0u8; 61];
    for i in 0..NUM_POINTS {
        println!("=== {} ===", i);
        decompressor.decompress_one(&mut buf);

        las_file.read_exact(&mut expected_buff).unwrap();


        // can't directly use assert_eq! on the buffers as Debug & Eq are not impl for [0u8; 61]
        // guess they will be when Rust's const generics are
        for (b, expected_b) in buf.iter().zip(expected_buff.iter()) {
            assert_eq!(b, expected_b);
        }
    }
}