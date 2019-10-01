use laz::checking::check_decompression;
use std::fs::File;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let laz_file = std::io::BufReader::new(File::open(&args[1]).unwrap());
    let las_file = std::io::BufReader::new(File::open(&args[2]).unwrap());

    check_decompression(laz_file, las_file);
}
