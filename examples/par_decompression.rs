use laz::las::file::read_header_and_vlrs;

#[cfg(feature = "parallel")]
fn main() {
    use laz::las::file::{point_format_id_compressed_to_uncompressd, SimpleReader};
    use laz::las::laszip::par_decompress_all_from_file_greedy;
    use std::fs::File;
    use std::io::BufReader;
    let mut args: Vec<String> = std::env::args().collect();

    if let Some(greedy_arg_pos) = args.iter().position(|arg| arg == "--greedy") {
        args.remove(greedy_arg_pos);
        if args.len() != 3 {
            println!("Usage: par_decompression LAZ_PATH LAS_PATH");
            std::process::exit(1);
        }

        let mut laz_file = BufReader::new(File::open(&args[1]).unwrap());
        let (laz_header, laz_vlr) = read_header_and_vlrs(&mut laz_file).unwrap();
        let laz_vlr = laz_vlr.expect("No LasZip Vlr in the laz file");

        let mut all_points =
            vec![0u8; laz_header.point_size as usize * laz_header.num_points as usize];

        par_decompress_all_from_file_greedy(&mut laz_file, &mut all_points, &laz_vlr).unwrap();

        let mut las_file =
            SimpleReader::new(BufReader::new(File::open(&args[2]).unwrap())).unwrap();
        assert_eq!(
            las_file.header.point_format_id,
            point_format_id_compressed_to_uncompressd(laz_header.point_format_id)
        );
        assert_eq!(las_file.header.num_points, laz_header.num_points);
        for decompressed_point in all_points.chunks_exact(laz_header.point_size as usize) {
            let las_point = las_file.read_next().unwrap().unwrap();
            assert_eq!(decompressed_point, las_point);
        }
    } else {
        if args.len() != 3 {
            println!("Usage: par_decompression LAZ_PATH LAS_PATH");
            std::process::exit(1);
        }

        let mut laz_file = BufReader::new(File::open(&args[1]).unwrap());
        let (laz_header, laz_vlr) = read_header_and_vlrs(&mut laz_file).unwrap();
        let laz_vlr = laz_vlr.expect("No LasZip Vlr in the laz file");

        let mut las_reader =
            SimpleReader::new(BufReader::new(File::open(&args[2]).unwrap())).unwrap();
        let mut decompressor = laz::ParLasZipDecompressor::new(laz_file, laz_vlr).unwrap();

        let mut laz_points = vec![];

        let num_points_per_iter = 1_158_989;
        let mut num_points_left = laz_header.num_points;

        while num_points_left > 0 {
            let num_points_to_read = std::cmp::min(num_points_left, num_points_per_iter);
            laz_points.resize(
                num_points_to_read as usize * laz_header.point_size as usize,
                0u8,
            );

            decompressor.decompress_many(&mut laz_points).unwrap();

            for decompressed_point in laz_points.chunks_exact(laz_header.point_size as usize) {
                let las_point = las_reader.read_next().unwrap().unwrap();
                assert_eq!(decompressed_point, las_point);
            }
            num_points_left -= num_points_to_read;
        }
    }
}

#[cfg(not(feature = "parallel"))]
fn main() {
    println!("laz-rs wasn't compiled with parallel feature");
}
