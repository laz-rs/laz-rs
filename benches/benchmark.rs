#[macro_use]
extern crate criterion;
extern crate laz;

use criterion::Criterion;

use laz::las::v2;
use laz::las::file::{QuickHeader};
use std::io::{Seek, SeekFrom, Read, Cursor, BufReader};
use laz::record::{SequentialPointRecordCompressor, RecordCompressor};
use std::fs::File;

/*
fn point0_v2_compression_benchmark(c: &mut Criterion) {
    c.bench_function("point0_v2_compression", |b| {
        let mut test_file = std::io::BufReader::new(std::fs::File::open("tests/data/point10.laz").unwrap());
        let hdr = QuickHeader::read_from(&mut test_file).unwrap();
        test_file.seek(SeekFrom::Start(hdr.offset_to_points as u64)).unwrap();
        let mut points_data = Vec::<u8>::new();
        test_file.read_to_end(&mut points_data).unwrap();

        let mut raw_pts_iter = points_data.windows(hdr.point_size as usize).cycle();
        let mut encoder = ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new()));

        let mut point0_compressor = LasPoint0Compressor::default();

        point0_compressor.compress_first(encoder.out_stream().get_mut(), raw_pts_iter.next().unwrap());

       b.iter(|| point0_compressor.compress_with(&mut encoder, raw_pts_iter.next().unwrap()).unwrap());
    });
}
*/

struct RawPointsData {
    point_size: usize,
    points_data: Vec<u8>,
}

impl RawPointsData {
    fn cycling_iterator(&self) -> std::iter::Cycle<std::slice::ChunksExact<u8>> {
        self.points_data.chunks_exact(self.point_size).cycle()
    }
}

fn get_raw_points_data(path: &str) -> RawPointsData {
    let mut test_file = BufReader::new(File::open(path).unwrap());
    let hdr = QuickHeader::read_from(&mut test_file).unwrap();
    test_file.seek(SeekFrom::Start(hdr.offset_to_points as u64)).unwrap();
    let mut points_data = Vec::<u8>::new();
    test_file.read_to_end(&mut points_data).unwrap();
    RawPointsData {
        point_size: hdr.point_size as usize,
        points_data,
    }
}

fn point_0_v2_record_compression_benchmark(c: &mut Criterion) {
    let raw_points_data = get_raw_points_data("tests/data/point10.las");

    let mut record_compressor = SequentialPointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    record_compressor.add_field_compressor(v2::LasPoint0Compressor::default());

    c.bench_function("point_0_v2_compression", move |b| {
        let mut raw_pts_iter = raw_points_data.cycling_iterator();
        b.iter(|| record_compressor.compress_next(raw_pts_iter.next().unwrap()));
    });
}


fn point_1_v2_record_compression_benchmark(c: &mut Criterion) {
    let raw_points_data = get_raw_points_data("tests/data/point-time.las");

    let mut record_compressor = SequentialPointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    record_compressor.add_field_compressor(v2::LasPoint0Compressor::default());
    record_compressor.add_field_compressor(v2::GpsTimeCompressor::default());

    c.bench_function("point_1_v2_compression", move |b| {
        let mut raw_pts_iter = raw_points_data.cycling_iterator();
        b.iter(|| record_compressor.compress_next(raw_pts_iter.next().unwrap()));
    });
}

fn point_2_v2_record_compression_benchmark(c: &mut Criterion) {
    let raw_points_data = get_raw_points_data("tests/data/point-color.las");

    let mut record_compressor = SequentialPointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    record_compressor.add_field_compressor(v2::LasPoint0Compressor::default());
    record_compressor.add_field_compressor(v2::LasRGBCompressor::default());

    c.bench_function("point_1_v2_compression", move |b| {
        let mut raw_pts_iter = raw_points_data.cycling_iterator();
        b.iter(|| record_compressor.compress_next(raw_pts_iter.next().unwrap()));
    });
}

fn point_3_v2_record_compression_benchmark(c: &mut Criterion) {
    let raw_points_data = get_raw_points_data("tests/data/point-time-color.las");

    let mut record_compressor = SequentialPointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    record_compressor.add_field_compressor(v2::LasPoint0Compressor::default());
    record_compressor.add_field_compressor(v2::GpsTimeCompressor::default());
    record_compressor.add_field_compressor(v2::LasRGBCompressor::default());

    c.bench_function("point_1_v2_compression", move |b| {
        let mut raw_pts_iter = raw_points_data.cycling_iterator();
        b.iter(|| record_compressor.compress_next(raw_pts_iter.next().unwrap()));
    });
}


criterion_group!(version_2_point_formats,
 point_0_v2_record_compression_benchmark,
 point_1_v2_record_compression_benchmark,
 point_2_v2_record_compression_benchmark,
 point_3_v2_record_compression_benchmark
 );
criterion_main!(version_2_point_formats);