#[cfg(feature = "parallel")]
fn main() {
    use laz::las::file::QuickHeader;
    use laz::las::file::{
        point_format_id_compressed_to_uncompressd, read_vlrs_and_get_laszip_vlr, SimpleReader,
    };
    use laz::las::laszip::par_decompress_all;
    use std::fs::File;
    use std::io::{BufReader, Seek, SeekFrom};
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage: {} LAZ_PATH LAS_PATH", args[0]);
        std::process::exit(1);
    }

    let mut laz_file = BufReader::new(File::open(&args[1]).unwrap());
    let laz_header = QuickHeader::read_from(&mut laz_file).unwrap();
    laz_file
        .seek(SeekFrom::Start(laz_header.header_size as u64))
        .unwrap();
    let laszip_vlr =
        read_vlrs_and_get_laszip_vlr(&mut laz_file, &laz_header).expect("no laszip vlr found");

    laz_file
        .seek(SeekFrom::Start(laz_header.offset_to_points as u64))
        .unwrap();

    let mut all_points = vec![0u8; laz_header.point_size as usize * laz_header.num_points as usize];

    par_decompress_all(&mut laz_file, &mut all_points, &laszip_vlr).unwrap();

    if let Some(las_path) = args.get(2) {
        let mut las_file =
            SimpleReader::new(BufReader::new(File::open(las_path).unwrap())).unwrap();
        assert_eq!(
            las_file.header.point_format_id,
            point_format_id_compressed_to_uncompressd(laz_header.point_format_id)
        );
        assert_eq!(las_file.header.num_points, laz_header.num_points);
        for i in 0..laz_header.num_points as usize {
            let las_point = las_file.read_next().unwrap().unwrap();
            let decompress_point = &all_points
                [(i * laz_header.point_size as usize)..(i + 1) * laz_header.point_size as usize];
            assert_eq!(decompress_point, las_point);
        }
    } else {
        println!("No LAS file path given, decompression result can't be checked");
    }
}

#[cfg(not(feature = "parallel"))]
fn main() {
    println!("laz-rs wasn't compiled with parallel feature");
}
