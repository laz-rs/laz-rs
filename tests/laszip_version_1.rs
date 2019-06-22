use laz::record::{PointRecordCompressor, PointRecordDecompressor};

use laz::las::point10::{LasPoint0, Point0};
use laz::las::v1;
use laz::las::{Point1, Point2, Point3};
use laz::record::{
    BufferFieldCompressor, BufferFieldDecompressor, BufferRecordCompressor,
    BufferRecordDecompressor, PointFieldCompressor, PointFieldDecompressor,
};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};

const LAS_HEADER_SIZE: u64 = 227;
const NUM_POINTS: usize = 1065;
const VLR_HEADER_SIZE: u64 = 54;

macro_rules! loop_test_on_point_type {
    ($func_name:ident, $source_las:expr, $point_type:ident, $point_start:expr, $field_compressors:expr, $field_decompressors:expr) => {
        #[test]
        fn $func_name() {
            let mut las_file = File::open($source_las).unwrap();
            las_file.seek(SeekFrom::Start($point_start)).unwrap();

            let mut compressor = PointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));
            for f_c in $field_compressors {
                compressor.add_boxed_compressor(f_c);
            }

            let mut point = $point_type::default();
            for _ in 0..NUM_POINTS {
                point.read_from(&mut las_file).unwrap();
                compressor.compress_next(&point).unwrap();
            }
            compressor.done().unwrap();

            let mut compression_output = compressor.into_stream();
            compression_output.set_position(0);
            let mut decompressor =
                PointRecordDecompressor::<_, $point_type>::new(compression_output);

            for f_d in $field_decompressors {
                decompressor.add_boxed_decompressor(f_d);
            }

            las_file.seek(SeekFrom::Start($point_start)).unwrap();
            let mut expected_point = $point_type::default();
            for i in 0..NUM_POINTS {
                expected_point.read_from(&mut las_file).unwrap();
                let decompressed_point = decompressor.decompress_next().unwrap();
                assert_eq!(
                    decompressed_point, expected_point,
                    "Points {} are not eq!",
                    i
                );
            }
        }
    };
}

macro_rules! vec_of_boxed_field_compressors {
    ($point_type:ident, $($x: expr),*) => {{
        let mut vector = Vec::new();
        $(vector.push(Box::new($x) as Box<dyn PointFieldCompressor<_, $point_type>>);)*
        vector
    }}
}

macro_rules! vec_of_boxed_field_decompressors {
    ($point_type:ident, $($x: expr),*) => {{
        let mut vector = Vec::new();
        $(vector.push(Box::new($x) as Box<dyn PointFieldDecompressor<_, $point_type>>);)*
        vector
    }}
}

loop_test_on_point_type!(
    test_point_0,
    "tests/data/point10.las",
    Point0,
    LAS_HEADER_SIZE,
    vec_of_boxed_field_compressors![Point0, v1::LasPoint0Compressor::new()],
    vec_of_boxed_field_decompressors![Point0, v1::LasPoint0Decompressor::new()]
);

loop_test_on_point_type!(
    point_1,
    "tests/data/point-time.las",
    Point1,
    LAS_HEADER_SIZE,
    vec_of_boxed_field_compressors![
        Point1,
        v1::LasPoint0Compressor::new(),
        v1::LasGpsTimeCompressor::new()
    ],
    vec_of_boxed_field_decompressors![
        Point1,
        v1::LasPoint0Decompressor::new(),
        v1::LasGpsTimeDecompressor::new()
    ]
);

loop_test_on_point_type!(
    test_point_2,
    "tests/data/point-color.las",
    Point2,
    LAS_HEADER_SIZE,
    vec_of_boxed_field_compressors![
        Point2,
        v1::LasPoint0Compressor::new(),
        v1::LasRGBCompressor::new()
    ],
    vec_of_boxed_field_decompressors![
        Point2,
        v1::LasPoint0Decompressor::new(),
        v1::LasRGBDecompressor::new()
    ]
);

loop_test_on_point_type!(
    test_point_3,
    "tests/data/point-time-color.las",
    Point3,
    LAS_HEADER_SIZE,
    vec_of_boxed_field_compressors![
        Point3,
        v1::LasPoint0Compressor::new(),
        v1::LasGpsTimeCompressor::new(),
        v1::LasRGBCompressor::new()
    ],
    vec_of_boxed_field_decompressors![
        Point3,
        v1::LasPoint0Decompressor::new(),
        v1::LasGpsTimeDecompressor::new(),
        v1::LasRGBDecompressor::new()
    ]
);

// We don't test extra bytes using the 'loop_on_point_type'
// as it requires a struct with extra_bytes support (like Point3WithExtraBytes)
// but laziness happens, extra bytes are tested with'loop_on_buffer_tho'

macro_rules! loop_test_on_buffer {
    ($test_name:ident, $source_las:expr, $point_size:expr, $point_start:expr, $field_compressors:expr, $field_decompressors:expr) => {
        #[test]
        fn $test_name() {
            let mut las_file = File::open($source_las).unwrap();
            las_file.seek(SeekFrom::Start($point_start)).unwrap();

            let mut expected_buf = [0u8; $point_size];

            let mut compressor = BufferRecordCompressor::new(Cursor::new(Vec::<u8>::new()));
            for c in $field_compressors {
                compressor.add_boxed_compressor(c);
            }

            for _ in 0..NUM_POINTS {
                las_file.read_exact(&mut expected_buf).unwrap();
                compressor.compress(&expected_buf).unwrap();
            }
            compressor.done().unwrap();

            let mut compression_output = compressor.into_stream();
            compression_output.set_position(0);

            let mut decompressor = BufferRecordDecompressor::new(compression_output);
            for d in $field_decompressors {
                decompressor.add_boxed_decompressor(d);
            }

            let mut buf = [0u8; $point_size];
            las_file.seek(SeekFrom::Start($point_start)).unwrap();
            for i in 0..NUM_POINTS {
                las_file.read_exact(&mut expected_buf).unwrap();
                decompressor
                    .decompress(&mut buf)
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

macro_rules! vec_of_boxed_buffer_compressors {
    ($($x: expr),*) => {{
        let mut vector = Vec::new();
        $(vector.push(Box::new($x) as Box<dyn BufferFieldCompressor<_>>);)*
        vector
    }}
}

macro_rules! vec_of_boxed_buffer_decompressors {
    ($($x: expr),*) => {{
        let mut vector = Vec::new();
        $(vector.push(Box::new($x) as Box<dyn BufferFieldDecompressor<_>>);)*
        vector
    }}
}

loop_test_on_buffer!(
    test_point_0_buffer,
    "tests/data/point10.las",
    20,
    LAS_HEADER_SIZE,
    vec_of_boxed_buffer_compressors![v1::LasPoint0Compressor::new()],
    vec_of_boxed_buffer_decompressors![v1::LasPoint0Decompressor::new()]
);

loop_test_on_buffer!(
    test_point_1_buffer,
    "tests/data/point-time.las",
    28,
    LAS_HEADER_SIZE,
    vec_of_boxed_buffer_compressors![
        v1::LasPoint0Compressor::new(),
        v1::LasGpsTimeCompressor::new()
    ],
    vec_of_boxed_buffer_decompressors![
        v1::LasPoint0Decompressor::new(),
        v1::LasGpsTimeDecompressor::new()
    ]
);

loop_test_on_buffer!(
    test_point_2_buffer,
    "tests/data/point-color.las",
    26,
    LAS_HEADER_SIZE,
    vec_of_boxed_buffer_compressors![v1::LasPoint0Compressor::new(), v1::LasRGBCompressor::new()],
    vec_of_boxed_buffer_decompressors![
        v1::LasPoint0Decompressor::new(),
        v1::LasRGBDecompressor::new()
    ]
);

loop_test_on_buffer!(
    test_point_3_buffer,
    "tests/data/point-time-color.las",
    34,
    LAS_HEADER_SIZE,
    vec_of_boxed_buffer_compressors![
        v1::LasPoint0Compressor::new(),
        v1::LasGpsTimeCompressor::new(),
        v1::LasRGBCompressor::new()
    ],
    vec_of_boxed_buffer_decompressors![
        v1::LasPoint0Decompressor::new(),
        v1::LasGpsTimeDecompressor::new(),
        v1::LasRGBDecompressor::new()
    ]
);

loop_test_on_buffer!(
    test_point_3_extra_bytes_buffer,
    "tests/data/extra-bytes.las",
    61,
    LAS_HEADER_SIZE + VLR_HEADER_SIZE + (5 * 192),
    vec_of_boxed_buffer_compressors![
        v1::LasPoint0Compressor::new(),
        v1::LasGpsTimeCompressor::new(),
        v1::LasRGBCompressor::new(),
        v1::LasExtraByteCompressor::new(27)
    ],
    vec_of_boxed_buffer_decompressors![
        v1::LasPoint0Decompressor::new(),
        v1::LasGpsTimeDecompressor::new(),
        v1::LasRGBDecompressor::new(),
        v1::LasExtraByteDecompressor::new(27)
    ]
);
