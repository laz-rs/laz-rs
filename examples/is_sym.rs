use laz::record::{RecordCompressor, RecordDecompressor};
use laz::las::{gps, point10, rgb};
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};

const LAS_HEADER_SIZE: u64 = 227;
//const SIZEOF_CHUNK_TABLE: i64 = 8;
const NUM_POINTS: usize = 6737761;
fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut las_file = BufReader::new(File::open(&args[1]).unwrap());
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    const POINT_SIZE: usize = 34;

    let mut buf = [0u8; POINT_SIZE];
    let mut expected_buff = [0u8; POINT_SIZE];

    let mut compressor = RecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(point10::v2::Point10Compressor::new());
    compressor.add_field_compressor(gps::v2::GpsTimeCompressor::new());
    compressor.add_field_compressor(rgb::v2::RGBCompressor::new());

    for _i in 0..NUM_POINTS {
        las_file.read_exact(&mut expected_buff).unwrap();
        compressor.compress(&expected_buff).unwrap();
    }
    compressor.done().unwrap();

    println!("Going to decompress");

    let mut compression_output = compressor.into_stream();
    compression_output.set_position(0);
    let mut decompressor = RecordDecompressor::new(compression_output);
    decompressor.add_field_decompressor(point10::v2::Point10Decompressor::new());
    decompressor.add_field_decompressor(gps::v2::GpsTimeDecompressor::new());
    decompressor.add_field_decompressor(rgb::v2::RGBDecompressor::new());

    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    for i in 0..NUM_POINTS {
        //println!("=== {} ===", i);
        las_file.read_exact(&mut expected_buff).unwrap();
        decompressor
            .decompress(&mut buf)
            .expect(&format!("Failed to decompress point {}", i));;

        assert_eq!(expected_buff[..20], buf[..20]);
        assert_eq!(expected_buff[20..], buf[20..]);
    }
}
