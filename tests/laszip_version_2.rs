use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};

use laz::{
    LasZipCompressor, LasZipDecompressor, LazItemRecordBuilder, LazItemType, LazVlr, LazVlrBuilder,
};

fn loop_test_on_buffer_(las_path: &str, laz_path: &str) {
    let mut las_file = File::open(las_path).unwrap();
    let (las_header, _) = laz::las::file::read_header_and_vlrs(&mut las_file).unwrap();

    let mut laz_file = File::open(laz_path).unwrap();
    let (laz_header, laz_vlr) = laz::las::file::read_header_and_vlrs(&mut laz_file).unwrap();
    let laz_vlr = laz_vlr.expect("Expected  a laz vlr in the laz file");

    assert_eq!(laz_header.point_size, las_header.point_size);
    assert_eq!(laz_header.num_points, las_header.num_points);

    let mut point_buf = vec![0u8; las_header.point_size as usize];
    let mut expected_point_buf = point_buf.clone();
    let mut compressed_data = Cursor::new(Vec::<u8>::new());
    {
        let mut decompressor = LasZipDecompressor::new(&mut laz_file, laz_vlr.clone()).unwrap();
        let mut compressor = LasZipCompressor::new(&mut compressed_data, laz_vlr.clone()).unwrap();

        for _ in 0..las_header.num_points {
            las_file.read_exact(&mut expected_point_buf).unwrap();
            decompressor.decompress_one(&mut point_buf).unwrap();
            assert_eq!(point_buf, expected_point_buf);
            compressor.compress_one(&expected_point_buf).unwrap();
        }
        compressor.done().unwrap();
    }

    las_file
        .seek(SeekFrom::Start(las_header.offset_to_points as u64))
        .unwrap();
    compressed_data.set_position(0);
    {
        let mut decompressor =
            LasZipDecompressor::new(&mut compressed_data, laz_vlr.clone()).unwrap();

        for _ in 0..las_header.num_points {
            las_file.read_exact(&mut expected_point_buf).unwrap();
            decompressor.decompress_one(&mut point_buf).unwrap();
            assert_eq!(point_buf, expected_point_buf);
        }
    }
}

#[test]
fn test_point_format_0_version_2() {
    loop_test_on_buffer_("tests/data/point10.las", "tests/data/point10.laz")
}

#[test]
fn test_point_format_1_version_2() {
    loop_test_on_buffer_("tests/data/point-time.las", "tests/data/point-time.laz")
}

#[test]
fn test_point_format_2_version_2() {
    loop_test_on_buffer_("tests/data/point-color.las", "tests/data/point-color.laz")
}

#[test]
fn test_point_format_3_version_2() {
    loop_test_on_buffer_(
        "tests/data/point-time-color.las",
        "tests/data/point-time-color.laz",
    )
}

#[test]
fn test_point_format_3_with_extra_bytes_version_2() {
    loop_test_on_buffer_("tests/data/extra-bytes.las", "tests/data/extra-bytes.laz")
}

fn create_data_with_small_chunk_size() -> (File, Cursor<Vec<u8>>, Cursor<Vec<u8>>) {
    const CHUNK_SIZE: u32 = 50;
    let mut las_file = File::open("tests/data/point10.las").unwrap();
    let (las_header, _) = laz::las::file::read_header_and_vlrs(&mut las_file).unwrap();
    las_file
        .seek(SeekFrom::Start(las_header.offset_to_points as u64))
        .unwrap();

    let vlr = LazVlrBuilder::new(
        LazItemRecordBuilder::new()
            .add_item(LazItemType::Point10)
            .build(),
    )
    .with_fixed_chunk_size(CHUNK_SIZE)
    .build();

    let mut vlr_data = Cursor::new(Vec::<u8>::new());
    vlr.write_to(&mut vlr_data).unwrap();
    vlr_data.seek(SeekFrom::Start(0)).unwrap();

    let mut compressor = LasZipCompressor::new(Cursor::new(Vec::<u8>::new()), vlr).unwrap();

    let mut buf = vec![0u8; las_header.point_size as usize];
    for _ in 0..las_header.num_points {
        las_file.read_exact(&mut buf).unwrap();
        compressor.compress_one(&buf).unwrap();
    }
    compressor.done().unwrap();

    let mut compressed_data_stream = compressor.into_inner();
    compressed_data_stream.seek(SeekFrom::Start(0)).unwrap();
    return (las_file, compressed_data_stream, vlr_data);
}

#[test]
fn test_seek() {
    // We use a small chunk size to generate chunked data so that we
    // can test the seek function
    const POINT_SIZE: usize = 20;
    let (mut las_file, compressed_data_stream, mut vlr_data) = create_data_with_small_chunk_size();
    las_file.seek(SeekFrom::Start(0)).unwrap();
    let (las_header, _) = laz::las::file::read_header_and_vlrs(&mut las_file).unwrap();

    let mut decompressor = LasZipDecompressor::new(
        compressed_data_stream,
        LazVlr::read_from(&mut vlr_data).unwrap(),
    )
    .unwrap();

    let mut decompression_buf = vec![0u8; las_header.point_size as usize];
    let mut buf = vec![0u8; las_header.point_size as usize];

    let point_idx = 5;
    las_file
        .seek(SeekFrom::Start(
            las_header.offset_to_points as u64 + (point_idx * POINT_SIZE) as u64,
        ))
        .unwrap();
    decompressor.seek(point_idx as u64).unwrap();

    decompressor.decompress_one(&mut decompression_buf).unwrap();
    las_file.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, &decompression_buf);

    let point_idx = 496;
    las_file
        .seek(SeekFrom::Start(
            las_header.offset_to_points as u64 + (point_idx * POINT_SIZE) as u64,
        ))
        .unwrap();
    decompressor.seek(point_idx as u64).unwrap();

    decompressor.decompress_one(&mut decompression_buf).unwrap();
    las_file.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, &decompression_buf);

    // stream to a point that is beyond the number of points compressed
    // BUT the point index fall into the last chunk index
    let point_idx = las_header.num_points as u64 + 1;
    las_file
        .seek(SeekFrom::Start(
            las_header.offset_to_points as u64 + (point_idx * las_header.point_size as u64) as u64,
        ))
        .unwrap();
    decompressor.seek(point_idx as u64).unwrap();

    assert!(decompressor.decompress_one(&mut decompression_buf).is_err());
    assert!(las_file.read_exact(&mut buf).is_err());

    // stream to a point that is beyond the number of points compressed
    // and that does not belong to the last chunk
    let point_idx = las_header.num_points as u64 + 36;
    las_file
        .seek(SeekFrom::Start(
            las_header.offset_to_points as u64 + (point_idx * las_header.point_size as u64) as u64,
        ))
        .unwrap();
    decompressor.seek(point_idx as u64).unwrap();

    assert!(decompressor.decompress_one(&mut decompression_buf).is_err());
    assert!(las_file.read_exact(&mut buf).is_err());
}

#[cfg(feature = "parallel")]
#[test]
fn test_parallel_seek() {
    // We use a small chunk size to generate chunked data so that we
    // can test the seek function
    let (mut las_file, compressed_data_stream, mut vlr_data) = create_data_with_small_chunk_size();
    las_file.seek(SeekFrom::Start(0)).unwrap();
    let (las_header, _) = laz::las::file::read_header_and_vlrs(&mut las_file).unwrap();

    let mut decompressor = laz::ParLasZipDecompressor::new(
        compressed_data_stream,
        LazVlr::read_from(&mut vlr_data).unwrap(),
    )
    .unwrap();
    let point_size = las_header.point_size as usize;
    let mut decompression_buf = vec![0u8; point_size];
    let mut buf = vec![0u8; point_size];

    let point_idx = 5;
    las_file
        .seek(SeekFrom::Start(
            las_header.offset_to_points as u64 + (point_idx * point_size) as u64,
        ))
        .unwrap();
    decompressor.seek(point_idx as u64).unwrap();

    decompressor
        .decompress_many(&mut decompression_buf)
        .unwrap();
    las_file.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, &decompression_buf);

    let point_idx = 496;
    las_file
        .seek(SeekFrom::Start(
            las_header.offset_to_points as u64 + (point_idx * point_size) as u64,
        ))
        .unwrap();
    decompressor.seek(point_idx as u64).unwrap();

    decompressor
        .decompress_many(&mut decompression_buf)
        .unwrap();
    las_file.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, &decompression_buf);

    // stream to a point that is beyond the number of points compressed
    // BUT the point index fall into the last chunk index
    let point_idx = las_header.num_points as u64 + 1;
    las_file
        .seek(SeekFrom::Start(
            las_header.offset_to_points as u64 + (point_idx * las_header.point_size as u64) as u64,
        ))
        .unwrap();
    decompressor.seek(point_idx as u64).unwrap();

    assert!(decompressor
        .decompress_many(&mut decompression_buf)
        .is_err());
    assert!(las_file.read_exact(&mut buf).is_err());

    // stream to a point that is beyond the number of points compressed
    // and that does not belong to the last chunk
    let point_idx = las_header.num_points as u64 + 36;
    las_file
        .seek(SeekFrom::Start(
            las_header.offset_to_points as u64 + (point_idx * las_header.point_size as u64) as u64,
        ))
        .unwrap();
    decompressor.seek(point_idx as u64).unwrap();

    assert!(decompressor
        .decompress_many(&mut decompression_buf)
        .is_err());
    assert!(las_file.read_exact(&mut buf).is_err());
}
