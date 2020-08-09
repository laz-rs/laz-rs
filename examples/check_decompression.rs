use laz::checking::LasChecker;
use laz::las::file::read_header_and_vlrs;

#[cfg(feature = "parallel")]
enum NumPointsPerIter {
    All,
    Value(u64),
}

#[cfg(feature = "parallel")]
struct ProgramArgs {
    laz_path: String,
    las_path: String,
    greedy: bool,
    num_points_per_iter: NumPointsPerIter,
}

#[cfg(feature = "parallel")]
impl ProgramArgs {
    fn new() -> Self {
        let mut num_points_per_iter = NumPointsPerIter::Value(1_145_647);
        let mut greedy = false;
        let mut las_path = "".to_owned();
        let mut laz_path: String = "".to_owned();
        let mut positional_arg_pos = 0;

        let mut args_iter = std::env::args();
        args_iter.next();

        while let Some(arg) = args_iter.next() {
            if arg == "--num_points_per_iter" {
                match args_iter.next() {
                    Some(value_str) => {
                        if value_str.trim_start().chars().next().unwrap() == '-' {
                            num_points_per_iter = NumPointsPerIter::All
                        } else {
                            num_points_per_iter = NumPointsPerIter::Value(value_str.parse::<u64>().unwrap())
                        }
                    }
                    None => {
                        println!("--num_points_per_iter expected a value, eg: '--num_points_per_iter 5_000_000");
                        std::process::exit(1);
                    }
                };
            } else if arg == "--greedy" {
                greedy = true;
            } else {
                if positional_arg_pos == 0 {
                    laz_path = arg.clone();
                    positional_arg_pos += 1;
                } else if positional_arg_pos == 1 {
                    las_path = arg.clone();
                    positional_arg_pos += 1;
                } else {
                    println!("Too many positional arguments");
                    std::process::exit(1);
                }
            }
        }

        if positional_arg_pos < 2 {
            println!("Usage: par_decompression LAZ_PATH LAS_PATH");
            std::process::exit(1);
        }

        ProgramArgs {
            las_path,
            laz_path,
            greedy,
            num_points_per_iter,
        }
    }
}

#[cfg(feature = "parallel")]
fn main() {
    use laz::las::laszip::par_decompress_all_from_file_greedy;
    use std::fs::File;
    use std::io::BufReader;

    let program_args = ProgramArgs::new();

    let mut laz_file = BufReader::new(File::open(program_args.laz_path).unwrap());
    let (laz_header, laz_vlr) = read_header_and_vlrs(&mut laz_file).unwrap();
    let laz_vlr = laz_vlr.expect("No LasZip Vlr in the laz file");

    if program_args.greedy {
        let mut all_points =
            vec![0u8; laz_header.point_size as usize * laz_header.num_points as usize];

        par_decompress_all_from_file_greedy(&mut laz_file, &mut all_points, &laz_vlr).unwrap();

        let mut checker = LasChecker::from_path(&program_args.las_path).unwrap();
        checker.check(&all_points);
    } else {
        let mut decompressor = laz::ParLasZipDecompressor::new(laz_file, laz_vlr).unwrap();

        let num_points_per_iter = match program_args.num_points_per_iter {
            NumPointsPerIter::All => laz_header.num_points as usize,
            NumPointsPerIter::Value(v) => v as usize
        };
        let mut num_points_left = laz_header.num_points as usize;

        let mut decompressed_points = vec![0u8; num_points_per_iter * laz_header.point_size as usize];
        let mut checker = LasChecker::from_path(&program_args.las_path).unwrap();

        while num_points_left > 0 {
            let num_points_to_read = std::cmp::min(num_points_left, num_points_per_iter);
            let points = &mut decompressed_points[..num_points_to_read * laz_header.point_size as usize];

            decompressor.decompress_many(points).unwrap();
            checker.check(points);

            num_points_left -= num_points_to_read;
        }
    }
}

#[cfg(not(feature = "parallel"))]
fn main() {
    use laz::checking::check_decompression;
    use std::fs::File;

    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        println!("Usage: {} LAZ_PATH LAS_PATH", args[0]);
        std::process::exit(1);
    }

    let laz_path = &args[1];
    let las_path = &args[2];
    let laz_file = std::io::BufReader::new(File::open(laz_path).unwrap());
    let las_file = std::io::BufReader::new(File::open(las_path).unwrap());

    check_decompression(laz_file, las_file);
}
