use std::fs::File;
use std::io::Cursor;
use std::io::{Read, Seek, SeekFrom};

use crate::las::v1;
use crate::record::{FieldCompressor, FieldDecompressor};

use crate::las::gps::{v2::GpsTimeCompressor, v2::GpsTimeDecompressor, GpsTime};
use crate::las::point0::{v2::LasPoint0Compressor, v2::LasPoint0Decompressor, Point0};
use crate::las::rgb::{v2::LasRGBCompressor, v2::LasRGBDecompressor, RGB};
use crate::packers::Packable;
use crate::record::{
    RecordCompressor, RecordDecompressor, SequentialPointRecordCompressor,
    SequentialPointRecordDecompressor,
};

#[test]
fn test_compression_decompression_of_point_10() {
    let mut compressor =
        SequentialPointRecordCompressor::new(std::io::Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(LasPoint0Compressor::default());

    let n: i32 = 10000;
    let mut buf = [0u8; 20];
    for i in 0..n {
        let point = Point0 {
            x: i,
            y: i + 1000,
            z: i + 10000,
            intensity: (i + (1 << 15)) as u16,
            return_number: ((i >> 3) & 0x7) as u8,
            number_of_returns_of_given_pulse: (i & 0x7) as u8,
            scan_direction_flag: (i & 1) != 0,
            edge_of_flight_line: ((i + 1) & 1) != 0,
            classification: (i % 256) as u8,
            scan_angle_rank: (i % 128) as i8,
            user_data: ((i >> 4) % 256) as u8,
            point_source_id: (i * 30 % (1 << 16)) as u16,
        };

        point.pack_into(&mut buf);
        compressor.compress_next(&buf).unwrap();
    }
    compressor.done().unwrap();

    let compressed_data = compressor.into_inner().into_inner();

    let mut decompressor =
        SequentialPointRecordDecompressor::new(std::io::Cursor::new(compressed_data));
    decompressor.add_field_decompressor(LasPoint0Decompressor::default());

    for i in 0..n {
        decompressor.decompress_next(&mut buf).unwrap();
        let point = Point0::unpack_from(&buf);

        let expected_point = Point0 {
            x: i,
            y: i + 1000,
            z: i + 10000,
            intensity: (i + (1 << 15)) as u16,
            return_number: ((i >> 3) & 0x7) as u8,
            number_of_returns_of_given_pulse: (i & 0x7) as u8,
            scan_direction_flag: (i & 1) != 0,
            edge_of_flight_line: ((i + 1) & 1) != 0,
            classification: (i % 256) as u8,
            scan_angle_rank: (i % 128) as i8,
            user_data: ((i >> 4) % 256) as u8,
            point_source_id: (i * 30 % (1 << 16)) as u16,
        };

        assert_eq!(point, expected_point);
    }
}

#[test]
fn test_rgb() {
    let mut compressor = SequentialPointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));

    compressor.add_field_compressor(LasRGBCompressor::default());

    let n = 10000;

    let mut buf = [0u8; 6];
    for i in 0..n {
        let rgb = RGB {
            red: (i + 1000) % 65535,
            green: (i + 5000) % 65535,
            blue: (i + 10000) % 65535,
        };

        rgb.pack_into(&mut buf);
        compressor.compress_next(&buf).unwrap();
    }
    compressor.done().unwrap();
    let compressed_data = compressor.into_inner().into_inner();

    let mut decompressor =
        SequentialPointRecordDecompressor::new(std::io::Cursor::new(compressed_data));
    decompressor.add_field_decompressor(LasRGBDecompressor::default());

    for i in 0..n {
        let expected_rgb = RGB {
            red: (i + 1000) % 65535,
            green: (i + 5000) % 65535,
            blue: (i + 10000) % 65535,
        };

        decompressor.decompress_next(&mut buf).unwrap();
        let rgb = RGB::unpack_from(&buf);

        assert_eq!(rgb, expected_rgb);
    }
}

#[test]
fn test_gps_time() {
    let mut compressor = SequentialPointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(GpsTimeCompressor::default());

    let n = 10000;

    let mut buf = [0u8; std::mem::size_of::<i64>()];
    for i in 0..n {
        let gps_time = GpsTime {
            value: (i + 48741) % std::i64::MAX,
        };
        gps_time.pack_into(&mut buf);

        compressor.compress_next(&buf).unwrap();
    }
    compressor.done().unwrap();

    let compressed_data = compressor.into_inner().into_inner();

    let mut decompressor = SequentialPointRecordDecompressor::new(Cursor::new(compressed_data));
    decompressor.add_field_decompressor(GpsTimeDecompressor::default());

    for i in 0..n {
        let expected_gps_time = GpsTime {
            value: (i + 48741) % std::i64::MAX,
        };
        decompressor.decompress_next(&mut buf).unwrap();
        let gps_time = GpsTime::unpack_from(&buf);
        assert_eq!(expected_gps_time, gps_time);
    }
}

const LAS_HEADER_SIZE: u64 = 227;
const NUM_POINTS: usize = 1065;
const VLR_HEADER_SIZE: u64 = 54;

macro_rules! loop_test_on_buffer {
    ($test_name:ident, $source_las:expr, $point_size:expr, $point_start:expr, $field_compressors:expr, $field_decompressors:expr) => {
        #[test]
        fn $test_name() {
            use crate::record::RecordCompressor;
            use crate::record::RecordDecompressor;
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

            let mut compression_output = compressor.into_inner();
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
        $(vector.push(Box::new($x) as Box<dyn FieldCompressor<_> + Send>);)*
        vector
    }}
}

macro_rules! vec_of_boxed_buffer_decompressors {
    ($($x: expr),*) => {{
        let mut vector = Vec::new();
        $(vector.push(Box::new($x) as Box<dyn FieldDecompressor<_> + Send>);)*
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
