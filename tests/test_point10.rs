use std::io::Cursor;

use laz::decoders::ArithmeticDecoder;
use laz::encoders::ArithmeticEncoder;
use laz::formats::{RecordCompressor, RecordDecompressor};
use laz::las::gps::{GpsTime, GpsTimeCompressor, GpsTimeDecompressor};
use laz::las::point10::{Point10, Point10Compressor, Point10Decompressor};
use laz::las::rgb::{RGBCompressor, RGBDecompressor, RGB};
use laz::packers::Packable;

#[test]
fn test_compression_decompression_of_point_10() {
    let mut compressor = RecordCompressor::new(ArithmeticEncoder::new(std::io::Cursor::new(
        Vec::<u8>::new(),
    )));
    compressor.add_field_compressor(Point10Compressor::new());

    let n: i32 = 10000;
    let mut buf = [0u8; 20];
    for i in 0..n {
        let point = Point10 {
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

        Point10::pack(point, &mut buf);
        compressor.compress(&buf);
    }
    compressor.done();

    let compressed_data = compressor.into_stream().into_inner();

    let mut decompressor = RecordDecompressor::new(ArithmeticDecoder::new(std::io::Cursor::new(
        compressed_data,
    )));
    decompressor.add_field(Point10Decompressor::new());

    for i in 0..n {
        decompressor.decompress(&mut buf);
        let point = Point10::unpack(&buf);

        let expected_point = Point10 {
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
    let mut compressor =
        RecordCompressor::new(ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new())));

    compressor.add_field_compressor(RGBCompressor::new());

    let n = 10000;

    let mut buf = [0u8; 6];
    for i in 0..n {
        let rgb = RGB {
            red: (i + 1000) % 65535,
            green: (i + 5000) % 65535,
            blue: (i + 10000) % 65535,
        };

        RGB::pack(rgb, &mut buf);
        compressor.compress(&buf);
    }
    compressor.done();
    let compressed_data = compressor.into_stream().into_inner();

    let mut decompressor = RecordDecompressor::new(ArithmeticDecoder::new(std::io::Cursor::new(
        compressed_data,
    )));
    decompressor.add_field(RGBDecompressor::new());

    for i in 0..n {
        let expected_rgb = RGB {
            red: (i + 1000) % 65535,
            green: (i + 5000) % 65535,
            blue: (i + 10000) % 65535,
        };

        decompressor.decompress(&mut buf);
        let rgb = RGB::unpack(&buf);

        assert_eq!(rgb, expected_rgb);
    }
}

#[test]
fn test_gps_time() {
    let mut compressor =
        RecordCompressor::new(ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new())));
    compressor.add_field_compressor(GpsTimeCompressor::new());

    let n = 10000;

    let mut buf = [0u8; std::mem::size_of::<i64>()];
    for i in 0..n {
        let gps_time = GpsTime {
            value: (i + 48741) % std::i64::MAX,
        };
        GpsTime::pack(gps_time, &mut buf);

        compressor.compress(&buf);
    }
    compressor.done();

    let compressed_data = compressor.into_stream().into_inner();

    let mut decompressor =
        RecordDecompressor::new(ArithmeticDecoder::new(Cursor::new(compressed_data)));
    decompressor.add_field(GpsTimeDecompressor::new());

    for i in 0..n {
        let expected_gps_time = GpsTime {
            value: (i + 48741) % std::i64::MAX,
        };
        decompressor.decompress(&mut buf);
        let gps_time = GpsTime::unpack(&buf);
        assert_eq!(expected_gps_time, gps_time);
    }
}
