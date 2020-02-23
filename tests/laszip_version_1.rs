use laz::las::v1;
use laz::record::{
    FieldCompressor, FieldDecompressor, SequentialPointRecordCompressor,
    SequentialPointRecordDecompressor,
};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};

const LAS_HEADER_SIZE: u64 = 227;
const NUM_POINTS: usize = 1065;
const VLR_HEADER_SIZE: u64 = 54;

macro_rules! loop_test_on_buffer {
    ($test_name:ident, $source_las:expr, $point_size:expr, $point_start:expr, $field_compressors:expr, $field_decompressors:expr) => {
        #[test]
        fn $test_name() {
            use laz::record::RecordCompressor;
            use laz::record::RecordDecompressor;
            let mut las_file = File::open($source_las).unwrap();
            las_file.seek(SeekFrom::Start($point_start)).unwrap();

            let mut expected_buf = [0u8; $point_size];

            let mut compressor =
                SequentialPointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));
            for c in $field_compressors {
                compressor.add_boxed_compressor(c);
            }

            for _ in 0..NUM_POINTS {
                las_file.read_exact(&mut expected_buf).unwrap();
                compressor.compress_next(&expected_buf).unwrap();
            }
            compressor.done().unwrap();

            let mut compression_output = compressor.into_stream();
            compression_output.set_position(0);

            let mut decompressor = SequentialPointRecordDecompressor::new(compression_output);
            for d in $field_decompressors {
                decompressor.add_boxed_decompressor(d);
            }

            let mut buf = [0u8; $point_size];
            las_file.seek(SeekFrom::Start($point_start)).unwrap();
            for i in 0..NUM_POINTS {
                las_file.read_exact(&mut expected_buf).unwrap();
                decompressor
                    .decompress_next(&mut buf)
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
        $(vector.push(Box::new($x) as Box<dyn FieldCompressor<_>>);)*
        vector
    }}
}

macro_rules! vec_of_boxed_buffer_decompressors {
    ($($x: expr),*) => {{
        let mut vector = Vec::new();
        $(vector.push(Box::new($x) as Box<dyn FieldDecompressor<_>>);)*
        vector
    }}
}

loop_test_on_buffer!(
    test_point_0_buffer,
    "tests/data/point10.las",
    20,
    LAS_HEADER_SIZE,
    vec_of_boxed_buffer_compressors![v1::LasPoint0Compressor::default()],
    vec_of_boxed_buffer_decompressors![v1::LasPoint0Decompressor::default()]
);

loop_test_on_buffer!(
    test_point_1_buffer,
    "tests/data/point-time.las",
    28,
    LAS_HEADER_SIZE,
    vec_of_boxed_buffer_compressors![
        v1::LasPoint0Compressor::default(),
        v1::LasGpsTimeCompressor::default()
    ],
    vec_of_boxed_buffer_decompressors![
        v1::LasPoint0Decompressor::default(),
        v1::LasGpsTimeDecompressor::default()
    ]
);

loop_test_on_buffer!(
    test_point_2_buffer,
    "tests/data/point-color.las",
    26,
    LAS_HEADER_SIZE,
    vec_of_boxed_buffer_compressors![
        v1::LasPoint0Compressor::default(),
        v1::LasRGBCompressor::default()
    ],
    vec_of_boxed_buffer_decompressors![
        v1::LasPoint0Decompressor::default(),
        v1::LasRGBDecompressor::default()
    ]
);

loop_test_on_buffer!(
    test_point_3_buffer,
    "tests/data/point-time-color.las",
    34,
    LAS_HEADER_SIZE,
    vec_of_boxed_buffer_compressors![
        v1::LasPoint0Compressor::default(),
        v1::LasGpsTimeCompressor::default(),
        v1::LasRGBCompressor::default()
    ],
    vec_of_boxed_buffer_decompressors![
        v1::LasPoint0Decompressor::default(),
        v1::LasGpsTimeDecompressor::default(),
        v1::LasRGBDecompressor::default()
    ]
);

loop_test_on_buffer!(
    test_point_3_extra_bytes_buffer,
    "tests/data/extra-bytes.las",
    61,
    LAS_HEADER_SIZE + VLR_HEADER_SIZE + (5 * 192),
    vec_of_boxed_buffer_compressors![
        v1::LasPoint0Compressor::default(),
        v1::LasGpsTimeCompressor::default(),
        v1::LasRGBCompressor::default(),
        v1::LasExtraByteCompressor::new(27)
    ],
    vec_of_boxed_buffer_decompressors![
        v1::LasPoint0Decompressor::default(),
        v1::LasGpsTimeDecompressor::default(),
        v1::LasRGBDecompressor::default(),
        v1::LasExtraByteDecompressor::new(27)
    ]
);
