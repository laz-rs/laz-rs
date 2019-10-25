use laz::checking::check_decompression;
use std::fs::File;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let laz_path = args.get(1).expect("Path to laz file as first argument");
    let las_path = args.get(2).expect("Path to las file as second argument");
    let laz_file = std::io::BufReader::new(File::open(laz_path).unwrap());
    let las_file = std::io::BufReader::new(File::open(las_path).unwrap());

    check_decompression(laz_file, las_file);
}
