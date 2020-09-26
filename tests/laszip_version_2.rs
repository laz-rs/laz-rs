use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};

use laz::las::laszip::{
    LasZipCompressor, LasZipDecompressor, LazItemRecordBuilder, LazItemType, LazVlr, LazVlrBuilder,
};

const LAS_HEADER_SIZE: u64 = 227;
const VLR_HEADER_SIZE: u64 = 54;
const OFFSET_TO_LASZIP_VLR_DATA: u64 = LAS_HEADER_SIZE + VLR_HEADER_SIZE;
const NUM_POINTS: usize = 1065;

/// Tests both the decompression and compression
/// The generated function reads the LAS and LAZ file (they must hold the same points data)
/// and compares the decompressed points with the uncompressed points,
/// it also compressed the points to read them again and compare that we have the same data
macro_rules! loop_test_on_buffer {
    ($test_name:ident, $source_las:expr, $source_laz:expr, $point_size:expr, $las_point_start:expr, $laz_vlr_data_start:expr) => {
        #[test]
        fn $test_name() {
            let mut las_file = File::open($source_las).unwrap();
            las_file.seek(SeekFrom::Start($las_point_start)).unwrap();

            let mut laz_file = File::open($source_laz).unwrap();
            laz_file.seek(SeekFrom::Start($laz_vlr_data_start)).unwrap();

            let laz_vlr = LazVlr::read_from(&mut laz_file).unwrap();
            assert_eq!(laz_vlr.items_size(), $point_size);

            let mut compressor = LasZipCompressor::from_laz_items(
                Cursor::new(Vec::<u8>::new()),
                laz_vlr.items().clone(),
            )
            .unwrap();
            let mut decompressor = LasZipDecompressor::new(laz_file, laz_vlr).unwrap();

            // write the laz record_data in a buffer, to use it later to create
            // our decompressor
            let mut my_laz_vlr = Cursor::new(Vec::<u8>::with_capacity(64));
            compressor.vlr().write_to(&mut my_laz_vlr).unwrap();

            let mut buf = [0u8; $point_size];
            let mut expected_buf = [0u8; $point_size];

            for i in 0..NUM_POINTS {
                decompressor.decompress_one(&mut buf).unwrap();
                las_file.read_exact(&mut expected_buf).unwrap();
                compressor.compress_one(&expected_buf).unwrap();

                let mut s;
                let mut e = 0;
                for j in 0..$point_size / 32 {
                    s = j * 32;
                    e = (j + 1) * 32;
                    assert_eq!(
                        buf[s..e],
                        expected_buf[s..e],
                        "Buffers[{}..{}] for point {} are not eq!",
                        s,
                        e,
                        i
                    );
                }
                assert_eq!(buf[e..], expected_buf[e..]);
            }
            compressor.done().unwrap();

            let mut compression_output = compressor.into_inner();
            compression_output.set_position(0);
            my_laz_vlr.set_position(0);
            let my_laz_vlr = LazVlr::read_from(&mut my_laz_vlr).unwrap();
            assert_eq!(my_laz_vlr.items_size(), $point_size);

            let mut decompressor = LasZipDecompressor::new(compression_output, my_laz_vlr).unwrap();

            las_file.seek(SeekFrom::Start($las_point_start)).unwrap();
            for i in 0..NUM_POINTS {
                las_file.read_exact(&mut expected_buf).unwrap();
                decompressor
                    .decompress_one(&mut buf)
                    .expect(&format!("Failed to decompress point {}", i));

                let mut s;
                let mut e = 0;
                for j in 0..$point_size / 32 {
                    s = j * 32;
                    e = (j + 1) * 32;
                    assert_eq!(
                        buf[s..e],
                        expected_buf[s..e],
                        "Buffers[{}..{}] for point {} are not eq!",
                        s,
                        e,
                        i
                    );
                }
                assert_eq!(buf[e..], expected_buf[e..]);
            }
        }
    };
}

loop_test_on_buffer!(
    test_point_format_0_version_2,
    "tests/data/point10.las",
    "tests/data/point10.laz",
    20,
    LAS_HEADER_SIZE,
    OFFSET_TO_LASZIP_VLR_DATA
);

loop_test_on_buffer!(
    test_point_format_1_version_2,
    "tests/data/point-time.las",
    "tests/data/point-time.laz",
    28,
    LAS_HEADER_SIZE,
    OFFSET_TO_LASZIP_VLR_DATA
);

loop_test_on_buffer!(
    test_point_format_2_version_2,
    "tests/data/point-color.las",
    "tests/data/point-color.laz",
    26,
    LAS_HEADER_SIZE,
    OFFSET_TO_LASZIP_VLR_DATA
);

loop_test_on_buffer!(
    test_point_format_3_version_2,
    "tests/data/point-time-color.las",
    "tests/data/point-time-color.laz",
    34,
    LAS_HEADER_SIZE,
    OFFSET_TO_LASZIP_VLR_DATA
);

loop_test_on_buffer!(
    test_point_format_3_with_extra_bytes_version_2,
    "tests/data/extra-bytes.las",
    "tests/data/extra-bytes.laz",
    61,
    LAS_HEADER_SIZE + VLR_HEADER_SIZE + (5 * 192),
    LAS_HEADER_SIZE + (2 * VLR_HEADER_SIZE) + (5 * 192)
);

#[test]
fn test_seek() {
    // We use a small chunk size to generate chunked data so that we
    // can test the seek function
    const CHUNK_SIZE: u32 = 50;
    const POINT_SIZE: usize = 20;
    let mut las_file = File::open("tests/data/point10.las").unwrap();
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();

    let vlr = LazVlrBuilder::new()
        .with_chunk_size(CHUNK_SIZE)
        .with_laz_items(
            LazItemRecordBuilder::new()
                .add_item(LazItemType::Point10)
                .build(),
        )
        .build();

    let mut vlr_data = Cursor::new(Vec::<u8>::new());
    vlr.write_to(&mut vlr_data).unwrap();
    vlr_data.seek(SeekFrom::Start(0)).unwrap();

    let mut compressor = LasZipCompressor::new(Cursor::new(Vec::<u8>::new()), vlr).unwrap();

    let mut buf = [0u8; POINT_SIZE];
    for _ in 0..NUM_POINTS {
        las_file.read_exact(&mut buf).unwrap();
        compressor.compress_one(&buf).unwrap();
    }
    compressor.done().unwrap();

    let mut compressed_data_stream = compressor.into_inner();
    compressed_data_stream.seek(SeekFrom::Start(0)).unwrap();

    let mut decompressor = LasZipDecompressor::new(
        compressed_data_stream,
        LazVlr::read_from(&mut vlr_data).unwrap(),
    )
    .unwrap();

    let mut decompression_buf = [0u8; POINT_SIZE];

    let point_idx = 5;
    las_file
        .seek(SeekFrom::Start(
            LAS_HEADER_SIZE + (point_idx * POINT_SIZE) as u64,
        ))
        .unwrap();
    decompressor.seek(point_idx as u64).unwrap();

    decompressor.decompress_one(&mut decompression_buf).unwrap();
    las_file.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, &decompression_buf);

    let point_idx = 496;
    las_file
        .seek(SeekFrom::Start(
            LAS_HEADER_SIZE + (point_idx * POINT_SIZE) as u64,
        ))
        .unwrap();
    decompressor.seek(point_idx as u64).unwrap();

    decompressor.decompress_one(&mut decompression_buf).unwrap();
    las_file.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, &decompression_buf);

    // stream to a point that is beyond the number of points compressed
    // BUT the point index fall into the last chunk index
    let point_idx = NUM_POINTS + 1;
    las_file
        .seek(SeekFrom::Start(
            LAS_HEADER_SIZE + (point_idx * POINT_SIZE) as u64,
        ))
        .unwrap();
    decompressor.seek(point_idx as u64).unwrap();

    assert!(decompressor.decompress_one(&mut decompression_buf).is_err());
    assert!(las_file.read_exact(&mut buf).is_err());

    // stream to a point that is beyond the number of points compressed
    // and that does not belong to the last chunk
    let point_idx = NUM_POINTS + 36;
    las_file
        .seek(SeekFrom::Start(
            LAS_HEADER_SIZE + (point_idx * POINT_SIZE) as u64,
        ))
        .unwrap();
    decompressor.seek(point_idx as u64).unwrap();

    assert!(decompressor.decompress_one(&mut decompression_buf).is_err());
    assert!(las_file.read_exact(&mut buf).is_err());
}
