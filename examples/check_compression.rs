use laz::checking::LasChecker;
use std::fs::File;
use std::io::Seek;
use std::io::{BufReader, Cursor, Read, SeekFrom};
use std::path::Path;

use laz::las::file::QuickHeader;
use laz::las::laszip::{LasZipDecompressor, LazItemRecordBuilder};

#[cfg(not(feature = "parallel"))]
use laz::las::laszip::LasZipCompressor;

#[cfg(feature = "parallel")]
use laz::las::laszip::{par_compress_buffer, LazVlr, ParLasZipCompressor};

#[cfg(feature = "parallel")]
fn par_check_compression<T: AsRef<Path>>(las_path: T, greedy: bool) {
    let mut las_file = BufReader::new(File::open(las_path).unwrap());
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
        for chunk in all_points.chunks(points_per_iter * laz_vlr.items_size() as usize) {
            compressor.compress_many(chunk).unwrap();
        }
        compressor.done().unwrap();
    }

    compression_out_put.set_position(0);
    let mut decompressor = LasZipDecompressor::new(compression_out_put, laz_vlr).unwrap();

    las_file.seek(SeekFrom::Start(0)).unwrap();
    let mut checker = LasChecker::new(&mut las_file).unwrap();

    let mut decompressed_point = vec![0u8; las_header.point_size as usize];
    for _ in 0..las_header.num_points as usize {
        decompressor
            .decompress_one(&mut decompressed_point)
            .unwrap();
        checker.check(&decompressed_point);
    }
}

#[cfg(feature = "parallel")]
fn main() {
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

    if Path::new(&args[1]).is_dir() {
        let las_globber = glob::glob(&format!("{}/**/*.las", &args[1])).unwrap();

        for las_entry in las_globber {
            let las_path = las_entry.unwrap();
            println!("Checking {}", las_path.display());
            par_check_compression(las_path, greedy);
        }
    } else {
        par_check_compression(&args[1], greedy);
    }
}

#[cfg(not(feature = "parallel"))]
fn check_compression<T: AsRef<Path>>(las_path: T) {
    let mut las_file = BufReader::new(File::open(las_path).unwrap());
    let las_header = QuickHeader::read_from(&mut las_file).unwrap();
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

    let mut out = compressor.into_inner();
    println!("Compressed to {} bytes", out.get_ref().len());
    out.set_position(0);
    let mut decompressor = LasZipDecompressor::new(out, vlr).unwrap();

    las_file.seek(SeekFrom::Start(0)).unwrap();
    let mut checker = LasChecker::new(&mut las_file).unwrap();
    println!("Decompression");
    let mut decompress_buf = vec![0u8; las_header.point_size as usize];
    for _ in 0..las_header.num_points {
        decompressor
            .decompress_one(&mut decompress_buf)
            .expect("Failed to decompress point");
        checker.check(&decompress_buf);
    }
}

#[cfg(not(feature = "parallel"))]
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        println!("Usage: {} LAS_PATH", args[0]);
        std::process::exit(1);
    };

    if Path::new(&args[1]).is_dir() {
        let las_globber = glob::glob(&format!("{}/**/*.las", &args[1])).unwrap();

        for las_entry in las_globber {
            let las_path = las_entry.unwrap();
            println!("Checking {}", las_path.display());
            check_compression(&las_path);
        }
    } else {
        check_compression(&args[1]);
    }
}
