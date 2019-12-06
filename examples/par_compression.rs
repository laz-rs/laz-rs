#[cfg(feature = "parallel")]
fn main() {
    use laz::las::file::QuickHeader;
    use laz::las::laszip::{par_compress_all, LasZipDecompressor, LazItemRecordBuilder, LazVlr};
    use std::fs::File;
    use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 1 {
        println!("Usage: {} LAS_PATH", args[0]);
        std::process::exit(1);
    }

    let mut las_file = BufReader::new(File::open(&args[1]).unwrap());
    let las_header = QuickHeader::read_from(&mut las_file).unwrap();

    las_file
        .seek(SeekFrom::Start(las_header.offset_to_points as u64))
        .unwrap();

    let mut all_points = vec![0u8; las_header.point_size as usize * las_header.num_points as usize];
    las_file.read_exact(&mut all_points).unwrap();
    let laz_items =
        LazItemRecordBuilder::default_for_point_format_id(las_header.point_format_id, 0);
    let laz_vlr = LazVlr::from_laz_items(laz_items);

    let mut compression_out_put = Cursor::new(Vec::<u8>::new());
    par_compress_all(&mut compression_out_put, &all_points, &laz_vlr).unwrap();

    compression_out_put.set_position(0);
    let mut decompressor = LasZipDecompressor::new(compression_out_put, laz_vlr).unwrap();

    las_file
        .seek(SeekFrom::Start(las_header.offset_to_points as u64))
        .unwrap();
    let mut decompressed_point = vec![0u8; las_header.point_size as usize];
    for i in 0..las_header.num_points as usize {
        decompressor
            .decompress_one(&mut decompressed_point)
            .unwrap();
        assert_eq!(
            decompressed_point,
            &all_points
                [(i * las_header.point_size as usize)..((i + 1) * las_header.point_size as usize)],
            "Points {} are not equal",
            i
        );
    }
}

#[cfg(not(feature = "parallel"))]
fn main() {
    println!("laz-rs wasn't compiled with parallel feature");
}
