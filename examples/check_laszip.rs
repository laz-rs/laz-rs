use byteorder::{LittleEndian, ReadBytesExt};
use laz::las::laszip::{
    LasZipCompressor, LasZipDecompressor, LazItemRecordBuilder, LazItemType, LazVlr,
};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};

const LAS_HEADER_SIZE: u64 = 227;
const VLR_HEADER_SIZE: u64 = 54;
const OFFSET_TO_LASZIP_VLR_DATA: u64 = LAS_HEADER_SIZE + VLR_HEADER_SIZE;
const POINT_SIZE: usize = 34;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut laz_file = std::io::BufReader::new(File::open(&args[1]).unwrap());

    laz_file.seek(SeekFrom::Start(107)).unwrap();
    let num_points = laz_file.read_u32::<LittleEndian>().unwrap();
    println!("Num points: {}", num_points);

    laz_file
        .seek(SeekFrom::Start(OFFSET_TO_LASZIP_VLR_DATA))
        .unwrap();
    let laz_vlr = LazVlr::read_from(&mut laz_file).unwrap();

    // Seek over chunk table offset
    laz_file
        .seek(SeekFrom::Current(std::mem::size_of::<u64>() as i64))
        .unwrap();

    let mut las_file = std::io::BufReader::new(File::open(&args[2]).unwrap());
    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();

    let mut buf = [0u8; POINT_SIZE];
    let mut expected_buff = [0u8; POINT_SIZE];

    let mut decompressor = LasZipDecompressor::new(&mut laz_file, laz_vlr).unwrap();
    let mut compressor = LasZipCompressor::from_laz_items(
        Cursor::new(Vec::<u8>::new()),
        LazItemRecordBuilder::new()
            .add_item(LazItemType::Point10)
            .add_item(LazItemType::GpsTime)
            .add_item(LazItemType::RGB12)
            .build(),
    );

    let mut my_laz_vlr = Cursor::new(Vec::<u8>::with_capacity(52));
    compressor.vlr().write_to(&mut my_laz_vlr).unwrap();

    for _i in 0..num_points {
        decompressor.decompress_one(&mut buf).unwrap();
        las_file.read_exact(&mut expected_buff).unwrap();
        assert_eq!(
            &expected_buff[0..20],
            &buf[0..20],
            "point10  decompression not ok"
        );
        assert_eq!(
            &expected_buff[20..28],
            &buf[20..28],
            "gps time decompression not ok"
        );
        assert_eq!(&expected_buff[28..], &buf[28..], "rgb decompression not ok");
        compressor.compress_one(&expected_buff).unwrap();
    }
    compressor.done().unwrap();

    let mut compression_output = compressor.into_stream();

    compression_output.set_position(0);
    my_laz_vlr.set_position(0);
    let my_laz_vlr = LazVlr::read_from(&mut my_laz_vlr).unwrap();

    let mut decompressor = LasZipDecompressor::new(&mut compression_output, my_laz_vlr).unwrap();

    las_file.seek(SeekFrom::Start(LAS_HEADER_SIZE)).unwrap();
    for i in 0..num_points {
        las_file.read_exact(&mut expected_buff).unwrap();
        decompressor.decompress_one(&mut buf).unwrap();

        assert_eq!(
            &expected_buff[0..20],
            &buf[0..20],
            "point 10 compression not ok: {}",
            i
        );
        assert_eq!(
            &expected_buff[20..28],
            &buf[20..28],
            "gps compression not ok: {}",
            i
        );
        assert_eq!(&expected_buff[28..], &buf[28..], "rgb compression not ok");
    }
}
