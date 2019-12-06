use laz::checking::check_decompression;
use std::fs::File;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        println!("Usage: {} LAZ_PATH LAS_PATH", args[0]);
        std::process::exit(1);
    }

    let laz_path = &args[1];
    let las_path =& args[2];
    let laz_file = std::io::BufReader::new(File::open(laz_path).unwrap());
    let las_file = std::io::BufReader::new(File::open(las_path).unwrap());

    check_decompression(laz_file, las_file);
}
