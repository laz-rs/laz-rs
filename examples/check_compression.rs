#[cfg(feature = "parallel")]
fn main() {
    use laz::las::file::QuickHeader;
    use laz::las::laszip::{par_compress_buffer, LasZipDecompressor, LazItemRecordBuilder, LazVlr};
    use laz::ParLasZipCompressor;
    use std::fs::File;
    use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};

    use laz::checking::LasChecker;
    use std::fs::File;
    use std::io::{BufReader, Read, Seek};

    let mut args: Vec<String> = std::env::args().collect();

    let greedy = if let Some(greedy_switch) = args.iter().position(|s| s == "--greedy") {
        args.remove(greedy_switch);
        true
    } else {
        false
    };

    if args.len() != 2 {
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
        LazItemRecordBuilder::default_for_point_format_id(las_header.point_format_id, 0).unwrap();
    let laz_vlr = LazVlr::from_laz_items(laz_items);

    let mut compression_out_put = Cursor::new(Vec::<u8>::new());

    if greedy {
        par_compress_buffer(&mut compression_out_put, &all_points, &laz_vlr).unwrap();
    } else {
        let points_per_iter = 1_158_989;
        let mut compressor =
            ParLasZipCompressor::new(&mut compression_out_put, laz_vlr.clone()).unwrap();
        for chunk in all_points.chunks(points_per_iter) {
            compressor.compress_many(chunk).unwrap();
        }
        compressor.done().unwrap();
    }

    compression_out_put.set_position(0);
    let mut decompressor = LasZipDecompressor::new(compression_out_put, laz_vlr).unwrap();

    las_file.seek(SeekFrom::Start(0)).unwrap();
    let mut checker = LasChecker::new(&mut las_file).unwrap();

    let mut decompressed_point = vec![0u8; las_header.point_size as usize];
    for i in 0..las_header.num_points as usize {
        decompressor
            .decompress_one(&mut decompressed_point)
            .unwrap();
        checker.check(&decompressed_point).unwrap();
    }
}

#[cfg(not(feature = "parallel"))]
fn main() {
    use std::fs::File;
    use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};

    use laz::las::file::QuickHeader;
    use laz::las::laszip::{LasZipCompressor, LasZipDecompressor, LazItemRecordBuilder};

    let args: Vec<String> = std::env::args().collect();
    let las_path = if let Some(path) = args.get(1) {
        path
    } else {
        println!("Usage: {} LAS_PATH", args[0]);
        std::process::exit(1);
    };

    let mut las_file = BufReader::new(File::open(las_path).unwrap());
    let las_header = QuickHeader::read_from(&mut las_file).unwrap();
    println!("{:?}", las_header);
    las_file
        .seek(SeekFrom::Start(las_header.offset_to_points as u64))
        .unwrap();

    let laz_items = LazItemRecordBuilder::default_for_point_format_id(
        las_header.point_format_id,
        las_header.num_extra_bytes(),
    )
    .unwrap();
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

