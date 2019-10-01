use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};

use laz::las::file::QuickHeader;
use laz::las::laszip::{LasZipCompressor, LasZipDecompressor, LazItemRecordBuilder};
use laz::las::{Point0, Point1, Point2, Point3, Point6, Point7, Point8};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut las_file = BufReader::new(File::open(&args[1]).unwrap());
    let las_header = QuickHeader::read_from(&mut las_file).unwrap();
    println!("{:?}", las_header);
    las_file
        .seek(SeekFrom::Start(las_header.offset_to_points as u64))
        .unwrap();

    let laz_items = match las_header.point_format_id {
        0 => LazItemRecordBuilder::default_version_of::<Point0>(0),
        1 => LazItemRecordBuilder::default_version_of::<Point1>(0),
        2 => LazItemRecordBuilder::default_version_of::<Point2>(0),
        3 => LazItemRecordBuilder::default_version_of::<Point3>(0),
        6 => LazItemRecordBuilder::default_version_of::<Point6>(0),
        7 => LazItemRecordBuilder::default_version_of::<Point7>(0),
        8 => LazItemRecordBuilder::default_version_of::<Point8>(0),
        _ => panic!(
            "Point format id: {} is not supported",
            las_header.point_format_id
        ),
    };
    let mut compressor =
        LasZipCompressor::from_laz_items(Cursor::new(Vec::<u8>::new()), laz_items).unwrap();

    let mut point_buf = vec![0u8; las_header.point_size as usize];
    for _ in 0..las_header.num_points {
        las_file.read_exact(&mut point_buf).unwrap();
        compressor
            .compress_one(&point_buf)
            .expect("Failed to decompress point");
    }
    compressor.done().expect("Error calling done on compressor");
    let vlr = compressor.vlr().clone();

    let mut out = compressor.into_stream();
    println!("Compressed to {} bytes", out.get_ref().len());
    out.set_position(0);
    let mut decompressor = LasZipDecompressor::new(out, vlr).unwrap();

    las_file
        .seek(SeekFrom::Start(las_header.offset_to_points as u64))
        .unwrap();

    println!("Decompression");
    let mut decompress_buf = vec![0u8; las_header.point_size as usize];
    for _ in 0..las_header.num_points {
        las_file.read_exact(&mut point_buf).unwrap();
        decompressor
            .decompress_one(&mut decompress_buf)
            .expect("Failed to decompress point");
        assert_eq!(&decompress_buf, &point_buf);
    }
}
