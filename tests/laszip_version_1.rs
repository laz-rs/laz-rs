use laz::las::{extra_bytes, gps, point10, rgb};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};

use laz::record::{RecordCompressor, RecordDecompressor};

const LAS_HEADER_SIZE: u64 = 227;
const NUM_POINTS: usize = 1065;
const VLR_HEADER_SIZE: u64 = 54;

#[test]
fn test_point_format_0_version_1_loop() {
    let mut las_file = File::open("tests/data/point10.las").unwrap();
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    const POINT_SIZE: usize = 20;

    let mut buf = [0u8; POINT_SIZE];
    let mut expected_buff = [0u8; POINT_SIZE];

    let mut compressor = RecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(point10::v1::Point10Compressor::new());

    for _ in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        compressor.compress(&expected_buff).unwrap();
    }
    compressor.done().unwrap();

    let mut compression_output = compressor.into_stream();
    compression_output.set_position(0);
    let mut decompressor = RecordDecompressor::new(compression_output);
    decompressor.add_field_decompressor(point10::v1::Point10Decompressor::new());

    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    for i in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        decompressor
            .decompress(&mut buf)
            .expect(&format!("Failed to decompress point {}", i));

        assert_eq!(expected_buff, buf);
    }
}

#[test]
fn test_point_format_1_version_1_loop() {
    let mut las_file = File::open("tests/data/point-time.las").unwrap();
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    const POINT_SIZE: usize = 28;

    let mut buf = [0u8; POINT_SIZE];
    let mut expected_buff = [0u8; POINT_SIZE];

    let mut compressor = RecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(point10::v1::Point10Compressor::new());
    compressor.add_field_compressor(gps::v1::GpsTimeCompressor::new());

    for _ in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        compressor.compress(&expected_buff).unwrap();
    }
    compressor.done().unwrap();

    let mut compression_output = compressor.into_stream();
    compression_output.set_position(0);
    let mut decompressor = RecordDecompressor::new(compression_output);
    decompressor.add_field_decompressor(point10::v1::Point10Decompressor::new());
    decompressor.add_field_decompressor(gps::v1::GpsTimeDecompressor::new());

    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    for i in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        decompressor
            .decompress(&mut buf)
            .expect(&format!("Failed to decompress point {}", i));

        assert_eq!(expected_buff, buf);
    }
}

#[test]
fn test_point_format_2_version_1_loop() {
    let mut las_file = File::open("tests/data/point-color.las").unwrap();
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    const POINT_SIZE: usize = 26;

    let mut buf = [0u8; POINT_SIZE];
    let mut expected_buff = [0u8; POINT_SIZE];

    let mut compressor = RecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(point10::v1::Point10Compressor::new());
    compressor.add_field_compressor(rgb::v1::RGBCompressor::new());

    for _ in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        compressor.compress(&expected_buff).unwrap();
    }
    compressor.done().unwrap();

    let mut compression_output = compressor.into_stream();
    compression_output.set_position(0);
    let mut decompressor = RecordDecompressor::new(compression_output);
    decompressor.add_field_decompressor(point10::v1::Point10Decompressor::new());
    decompressor.add_field_decompressor(rgb::v1::RGBDecompressor::new());

    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    for i in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        decompressor
            .decompress(&mut buf)
            .expect(&format!("Failed to decompress point {}", i));;

        assert_eq!(expected_buff, buf);
    }
}

#[test]
fn test_point_format_3_version_1_loop() {
    let mut las_file = File::open("tests/data/point-color-time.las").unwrap();
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    const POINT_SIZE: usize = 34;

    let mut buf = [0u8; POINT_SIZE];
    let mut expected_buff = [0u8; POINT_SIZE];

    let mut compressor = RecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(point10::v1::Point10Compressor::new());
    compressor.add_field_compressor(gps::v1::GpsTimeCompressor::new());
    compressor.add_field_compressor(rgb::v1::RGBCompressor::new());

    for _ in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        compressor.compress(&expected_buff).unwrap();
    }
    compressor.done().unwrap();

    let mut compression_output = compressor.into_stream();
    compression_output.set_position(0);
    let mut decompressor = RecordDecompressor::new(compression_output);
    decompressor.add_field_decompressor(point10::v1::Point10Decompressor::new());
    decompressor.add_field_decompressor(gps::v1::GpsTimeDecompressor::new());
    decompressor.add_field_decompressor(rgb::v1::RGBDecompressor::new());

    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    for i in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        decompressor
            .decompress(&mut buf)
            .expect(&format!("Failed to decompress point {}", i));;

        assert_eq!(expected_buff[..20], buf[..20]);
        assert_eq!(expected_buff[20..], buf[20..]);
    }
}

#[test]
fn test_point_format_3_with_extra_bytes_version_1_loop() {
    let mut las_file = File::open("tests/data/extra-bytes.las").unwrap();
    // account for the extra bytes vlr
    las_file
        .seek(SeekFrom::Start(
            LAS_HEADER_SIZE + VLR_HEADER_SIZE + (5 * 192),
        ))
        .unwrap();
    const POINT_SIZE: usize = 61;

    let mut buf = [0u8; POINT_SIZE];
    let mut expected_buff = [0u8; POINT_SIZE];

    let mut compressor = RecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(point10::v1::Point10Compressor::new());
    compressor.add_field_compressor(gps::v1::GpsTimeCompressor::new());
    compressor.add_field_compressor(rgb::v1::RGBCompressor::new());
    compressor.add_field_compressor(extra_bytes::v1::ExtraBytesCompressor::new(27));

    for _ in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        compressor.compress(&expected_buff).unwrap();
    }
    compressor.done().unwrap();

    let mut compression_output = compressor.into_stream();
    compression_output.set_position(0);
    let mut decompressor = RecordDecompressor::new(compression_output);
    decompressor.add_field_decompressor(point10::v1::Point10Decompressor::new());
    decompressor.add_field_decompressor(gps::v1::GpsTimeDecompressor::new());
    decompressor.add_field_decompressor(rgb::v1::RGBDecompressor::new());
    decompressor.add_field_decompressor(extra_bytes::v1::ExtraBytesDecompressor::new(27));

    // account for the extra bytes vlr
    las_file
        .seek(SeekFrom::Start(
            LAS_HEADER_SIZE + VLR_HEADER_SIZE + (5 * 192),
        ))
        .unwrap();
    for i in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        decompressor
            .decompress(&mut buf)
            .expect(&format!("Failed to decompress point {}", i));;

        assert_eq!(expected_buff[..30], buf[..30]);
        assert_eq!(expected_buff[30..], buf[30..]);
    }
}
