use std::env;
use std::fs;
use std::io;

use std::io::{Read};
use std::time::Instant;


#[cfg(not(feature = "parallel"))]
fn main() {
    let stream_mode=
    if env::args().last().unwrap() == "--stream" {
        true
    } else {
        false
    };

    let mut laz_file = io::BufReader::new(fs::File::open(env::args().nth(1).unwrap()).unwrap());

    let now = Instant::now();
    if stream_mode {
        let mut laz_data = Vec::<u8>::new();
        laz_file.read_to_end(&mut laz_data).unwrap();

        let mut laz_data = io::Cursor::new(laz_data);

        let (hdr, laz_vlr) = laz::las::file::read_header_and_vlrs(&mut laz_data).unwrap();
        let laz_vlr = laz_vlr.expect("No laszip VLR, is it really a LAZ file ?");
        let mut point_buf = vec![0u8; hdr.point_size as usize * hdr.num_points as usize];

        laz::LasZipDecompressor::new(&mut laz_data, laz_vlr)
            .and_then(|mut decompressor| {
                decompressor.decompress_many(&mut point_buf)?;
                Ok(())
            }).unwrap();
    } else {
        let (hdr, laz_vlr) = laz::las::file::read_header_and_vlrs(&mut laz_file).unwrap();
        let laz_vlr = laz_vlr.expect("No laszip VLR, is it really a LAZ file ?");

        let mut point_buf = vec![0u8; hdr.point_size as usize];
        let mut decompressor = laz::LasZipDecompressor::new(laz_file, laz_vlr).unwrap();
        for _ in 0..hdr.num_points {
            decompressor.decompress_one(&mut point_buf).unwrap();
        }
    }
    let duration = now.elapsed();
    println!("Decompressed in {}s {} ms", duration.as_secs(), duration.subsec_millis());
}
#[cfg(feature = "parallel")]
fn main() {
    use std::io::{Seek, SeekFrom};

    let mut laz_file = io::BufReader::new(fs::File::open(env::args().nth(1).unwrap()).unwrap());
    let (hdr, laz_vlr) = laz::las::file::read_header_and_vlrs(&mut laz_file).unwrap();
    let laz_vlr = laz_vlr.expect("No laszip VLR, is it really a LAZ file ?");

    let mut point_buf = vec![0u8; hdr.point_size as usize * hdr.num_points as usize];
    let now = Instant::now();
    laz::las::laszip::par_decompress_all_from_file_greedy(&mut laz_file, &mut point_buf, &laz_vlr).unwrap();
    let duration = now.elapsed();
    println!("Decompressed in {}s {} ms", duration.as_secs(), duration.subsec_millis());
}
