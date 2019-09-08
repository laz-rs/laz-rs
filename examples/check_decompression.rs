use laz::las::file::{QuickHeader, SimpleReader};
use laz::las::laszip::LasZipDecompressor;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let laz_file = std::io::BufReader::new(File::open(&args[1]).unwrap());
    let mut las_file = std::io::BufReader::new(File::open(&args[2]).unwrap());

    let las_header = QuickHeader::read_from(&mut las_file).unwrap();
    las_file
        .seek(SeekFrom::Start(las_header.offset_to_points as u64))
        .unwrap();

    let mut laz_reader = SimpleReader::new(laz_file).unwrap();

    laz_reader
        .src
        .seek(SeekFrom::Start(laz_reader.header.offset_to_points as u64))
        .unwrap();
    let header = laz_reader.header;
    let mut decompressor =
        LasZipDecompressor::new(laz_reader.src, laz_reader.laszip_vlr.unwrap()).unwrap();

    println!(
        "point sizes: {}, {}",
        header.point_size, las_header.point_size
    );
    println!("las hdr: {:?}", header);
    let mut decompressed_point = Vec::<u8>::new();
    decompressed_point.resize(header.point_size as usize, 0);
    let mut las_point = Vec::<u8>::new();
    las_point.resize(las_header.point_size as usize, 0);

    for i in 0..header.num_points {
        decompressor
            .decompress_one(&mut decompressed_point)
            .unwrap();
        las_file.read_exact(&mut las_point).unwrap();
        assert_eq!(las_point, decompressed_point, "Point: {} not equal", i);
    }
}
