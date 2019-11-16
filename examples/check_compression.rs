use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};

use laz::las::file::QuickHeader;
use laz::las::laszip::{LasZipCompressor, LasZipDecompressor, LazItemRecordBuilder};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut las_file = BufReader::new(File::open(&args[1]).unwrap());
    let las_header = QuickHeader::read_from(&mut las_file).unwrap();
    println!("{:?}", las_header);
    las_file
        .seek(SeekFrom::Start(las_header.offset_to_points as u64))
        .unwrap();

    let laz_items =
        LazItemRecordBuilder::default_for_point_format_id(
            las_header.point_format_id,
            las_header.num_extra_bytes()
        );
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
