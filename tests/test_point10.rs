use std::io::Cursor;

use laz::las::gps::{v2::GpsTimeCompressor, v2::GpsTimeDecompressor, GpsTime};
use laz::las::point0::{v2::LasPoint0Compressor, v2::LasPoint0Decompressor, Point0};
use laz::las::rgb::{v2::LasRGBCompressor, v2::LasRGBDecompressor, RGB};
use laz::packers::Packable;
use laz::record::{
    RecordCompressor, RecordDecompressor, SequentialPointRecordCompressor,
    SequentialPointRecordDecompressor,
};

#[test]
fn test_compression_decompression_of_point_10() {
    let mut compressor =
        SequentialPointRecordCompressor::<[u8], _>::new(std::io::Cursor::new(Vec::<u8>::new()));
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
    let mut compressor = SequentialPointRecordCompressor::<[u8], _>::new(Cursor::new(Vec::<u8>::new()));

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
    let mut compressor = SequentialPointRecordCompressor::<[u8], _>::new(Cursor::new(Vec::<u8>::new()));
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
