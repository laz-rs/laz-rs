use std::fs::File;
use std::io::{BufReader, Cursor};

use laz::las::file::SimpleReader;
use laz::{LasZipCompressor, LasZipDecompressor, LazItemRecordBuilder, LazVlrBuilder};

fn organize_as_variable_size_chunks(
    raw_points: &[u8],
    point_size: usize,
    chunks_sizes: &[usize],
) -> Vec<Vec<u8>> {
    let mut chunks = Vec::<Vec<u8>>::with_capacity(chunks_sizes.len());
    for chunk_size in chunks_sizes {
        let mut chunk = vec![0u8; chunk_size * point_size];
        chunk.copy_from_slice(&raw_points[..chunk_size * point_size]);
        chunks.push(chunk);
    }
    chunks
}

fn check_chunks(concatenated_chunks: &[u8], chunks: &Vec<Vec<u8>>) {
    let mut start = 0usize;
    let mut end = 0usize;
    for chunk in chunks {
        end += chunk.len();
        assert_eq!(&concatenated_chunks[start..end], chunk);
        start = end;
    }
}

/// Test variable chunk size compression by first compressing data with
/// variable size chunks enabled, then checking if the decompression gives back
/// the expected points.
///
/// This test both the compression and decompression.
#[test]
fn test_variable_size_chunks() {
    // 1. Data preparation: read a LAS file to pick points that we are going to use
    // in this test.
    let las_file = BufReader::new(File::open("tests/data/point-time-color.las").unwrap());
    let mut las_reader = SimpleReader::new(las_file).unwrap();
    let mut las_points_bytes = Vec::<u8>::new();
    las_reader.read_to_end(&mut las_points_bytes).unwrap();

    let point_size = las_reader.header.point_size as usize;
    let chunk_sizes = [1, 2, 3, 4, 5, 6, 5, 4, 3, 2, 1];
    let chunks = organize_as_variable_size_chunks(&las_points_bytes, point_size, &chunk_sizes);

    // 2. Compress into multiple chunks of variable size.
    let laz_vlr = LazVlrBuilder::from_laz_items(
        LazItemRecordBuilder::default_for_point_format_id(las_reader.header.point_format_id, 0)
            .unwrap(),
    )
    .with_variable_chunk_size()
    .build();
    let mut compressed_output = Cursor::new(Vec::<u8>::new());
    {
        let mut compressor =
            LasZipCompressor::new(&mut compressed_output, laz_vlr.clone()).unwrap();
        compressor.compress_chunks(&chunks).unwrap();
        compressor.done().unwrap();
    }

    // 3. Decompress what we just compressed and see if we get the same points
    // that we gave to the compressor
    compressed_output.set_position(0);
    let mut decompressor = LasZipDecompressor::new(&mut compressed_output, laz_vlr).unwrap();
    let num_points_compressed = chunk_sizes.iter().sum::<usize>();
    let mut points_out = vec![0u8; point_size * num_points_compressed];
    decompressor.decompress_many(&mut points_out).unwrap();
    check_chunks(&points_out, &chunks);
}

/// Test variable chunk size compression by first compressing data with
/// variable size chunks enabled, then checking if the decompression gives back
/// the expected points.
///
/// This **only** test the compression
#[cfg(feature = "parallel")]
#[test]
fn test_variable_size_chunks_parallel_compression() {
    use laz::ParLasZipCompressor;
    // 1. Data preparation: read a LAS file to pick points that we are going to use
    // in this test.
    let las_file = BufReader::new(File::open("tests/data/point-time-color.las").unwrap());
    let mut las_reader = SimpleReader::new(las_file).unwrap();
    let mut las_points_bytes = Vec::<u8>::new();
    las_reader.read_to_end(&mut las_points_bytes).unwrap();

    let point_size = las_reader.header.point_size as usize;
    let chunk_sizes = [1, 2, 3, 4, 5, 6, 5, 4, 3, 2, 1];
    let chunks = organize_as_variable_size_chunks(&las_points_bytes, point_size, &chunk_sizes);

    // 2. Compress into multiple chunks of variable size.
    let laz_vlr = LazVlrBuilder::from_laz_items(
        LazItemRecordBuilder::default_for_point_format_id(las_reader.header.point_format_id, 0)
            .unwrap(),
    )
    .with_variable_chunk_size()
    .build();
    let mut compressed_output = Cursor::new(Vec::<u8>::new());
    {
        let mut compressor =
            ParLasZipCompressor::new(&mut compressed_output, laz_vlr.clone()).unwrap();
        compressor.compress_chunks(&chunks).unwrap();
        compressor.done().unwrap();
    }

    // 3. Decompress what we just compressed and see if we get the same points
    // that we gave to the compressor
    compressed_output.set_position(0);
    let mut decompressor = LasZipDecompressor::new(&mut compressed_output, laz_vlr).unwrap();
    let num_points_compressed = chunk_sizes.iter().sum::<usize>();
    let mut points_out = vec![0u8; point_size * num_points_compressed];
    decompressor.decompress_many(&mut points_out).unwrap();
    check_chunks(&points_out, &chunks);
}

/// This **only** test the compression
#[cfg(feature = "parallel")]
#[test]
fn test_variable_size_chunks_parallel_decompression() {
    use laz::ParLasZipDecompressor;
    // 1. Data preparation: read a LAS file to pick points that we are going to use
    // in this test.
    let las_file = BufReader::new(File::open("tests/data/point-time-color.las").unwrap());
    let mut las_reader = SimpleReader::new(las_file).unwrap();
    let mut las_points_bytes = Vec::<u8>::new();
    las_reader.read_to_end(&mut las_points_bytes).unwrap();

    let point_size = las_reader.header.point_size as usize;
    let chunk_sizes = [1, 2, 3, 4, 5, 6, 5, 4, 3, 2, 1];
    let chunks = organize_as_variable_size_chunks(&las_points_bytes, point_size, &chunk_sizes);

    // 2. Compress into multiple chunks of variable size.
    let laz_vlr = LazVlrBuilder::from_laz_items(
        LazItemRecordBuilder::default_for_point_format_id(las_reader.header.point_format_id, 0)
            .unwrap(),
    )
    .with_variable_chunk_size()
    .build();
    let mut compressed_output = Cursor::new(Vec::<u8>::new());
    {
        let mut compressor =
            LasZipCompressor::new(&mut compressed_output, laz_vlr.clone()).unwrap();
        compressor.compress_chunks(&chunks).unwrap();
        compressor.done().unwrap();
    }

    // 3. Decompress what we just compressed and see if we get the same points
    // that we gave to the compressor
    compressed_output.set_position(0);
    let mut decompressor = ParLasZipDecompressor::new(&mut compressed_output, laz_vlr).unwrap();
    let num_points_compressed = chunk_sizes.iter().sum::<usize>();
    let mut points_out = vec![0u8; point_size * num_points_compressed];
    decompressor.decompress_many(&mut points_out).unwrap();
    check_chunks(&points_out, &chunks);
}
