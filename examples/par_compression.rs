#[cfg(feature = "parallel")]
fn main() {
    use laz::las::laszip::{par_compress_all, LazItemRecordBuilder};
    use laz::las::file::QuickHeader;
    use std::io::{BufReader, Seek, SeekFrom, Cursor};
    use std::fs::File;
    let args: Vec<String> = std::env::args().collect();

    let mut las_file = BufReader::new(File::open(&args[1]).unwrap());
    let las_header = QuickHeader::read_from(&mut las_file).unwrap();

    las_file.seek(SeekFrom::Start(las_header.offset_to_points as u64)).unwrap();

    let all_points = vec![0u8; las_header.point_size as usize * las_header.num_points as usize];
    let laz_items = LazItemRecordBuilder::default_for_point_format_id(las_header.point_format_id, 0);


    let mut compression_out_put = Cursor::new(Vec::<u8>::new());
    par_compress_all(&mut compression_out_put, &all_points, laz_items).unwrap();
}

#[cfg(not(feature = "parallel"))]
fn main() {
    println!("laz-rs wasn't compiled with parallel feature");
}