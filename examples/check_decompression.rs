use glob;
use std::path::Path;

use std::io::BufReader;

#[cfg(feature = "parallel")]
use laz::checking::LasChecker;
#[cfg(feature = "parallel")]
use laz::las::file::read_header_and_vlrs;
#[cfg(feature = "parallel")]
use laz::las::laszip::par_decompress_all_from_file_greedy;

use std::fs::File;

#[cfg(feature = "parallel")]
#[derive(Copy, Clone)]
enum NumPointsPerIter {
    All,
    Value(u64),
}

#[cfg(feature = "parallel")]
#[derive(Copy, Clone)]
struct ProgramArgs<'a> {
    laz_path: &'a str,
    las_path: &'a str,
    greedy: bool,
    num_points_per_iter: NumPointsPerIter,
}

#[cfg(feature = "parallel")]
impl<'a> ProgramArgs<'a> {
    fn new(args: &'a Vec<String>) -> Self {
        let mut num_points_per_iter = NumPointsPerIter::Value(1_145_647);
        let mut greedy = false;
        let mut las_path = "";
        let mut laz_path = "";
        let mut positional_arg_pos = 0;

        let mut args_iter = args.iter();
        args_iter.next();

        while let Some(arg) = args_iter.next() {
            if arg == "--num_points_per_iter" {
                match args_iter.next() {
                    Some(value_str) => {
                        if value_str.trim_start().chars().next().unwrap() == '-' {
                            num_points_per_iter = NumPointsPerIter::All
                        } else {
                            num_points_per_iter =
                                NumPointsPerIter::Value(value_str.parse::<u64>().unwrap())
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
                    laz_path = &arg;
                    positional_arg_pos += 1;
                } else if positional_arg_pos == 1 {
                    las_path = &arg;
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
fn run_check(program_args: &ProgramArgs) {
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
            NumPointsPerIter::Value(v) => v as usize,
        };
        let mut num_points_left = laz_header.num_points as usize;

        let mut decompressed_points =
            vec![0u8; num_points_per_iter * laz_header.point_size as usize];
        let mut checker = LasChecker::from_path(&program_args.las_path).unwrap();

        while num_points_left > 0 {
            let num_points_to_read = std::cmp::min(num_points_left, num_points_per_iter);
            let points =
                &mut decompressed_points[..num_points_to_read * laz_header.point_size as usize];

            decompressor.decompress_many(points).unwrap();
            checker.check(points);

            num_points_left -= num_points_to_read;
        }
    }
}

#[cfg(feature = "parallel")]
fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let program_args = ProgramArgs::new(&args);

    if Path::new(&args[1]).is_dir() && Path::new(&args[2]).is_dir() {
        let laz_globber = glob::glob(&format!("{}/**/*.laz", &args[1])).unwrap();
        let las_globber = glob::glob(&format!("{}/**/*.las", &args[2])).unwrap();

        for (las_entry, laz_entry) in las_globber.zip(laz_globber) {
            let laz_path = laz_entry.unwrap();
            let las_path = las_entry.unwrap();

            println!("{:?} - {:?}", las_path, laz_path);

            let mut args: ProgramArgs = program_args;

            args.laz_path = laz_path.to_str().unwrap();
            args.las_path = las_path.to_str().unwrap();

            if las_path.file_stem().unwrap() == las_path.file_stem().unwrap() {
                run_check(&args);
            }
        }
    } else if Path::new(&args[1]).is_file() & &Path::new(&args[2]).is_file() {
        run_check(&program_args);
    } else {
        println!("Arguments must both be either path to file or path to directory");
    }
}

#[cfg(not(feature = "parallel"))]
fn main() {
    use laz::checking::check_decompression;

    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        println!("Usage: {} LAZ_PATH LAS_PATH", args[0]);
        std::process::exit(1);
    }

    if Path::new(&args[1]).is_dir() && Path::new(&args[2]).is_dir() {
        let laz_globber = glob::glob(&format!("{}/**/*.laz", &args[1])).unwrap();
        let las_globber = glob::glob(&format!("{}/**/*.las", &args[2])).unwrap();

        for (las_entry, laz_entry) in las_globber.zip(laz_globber) {
            let laz_path = laz_entry.unwrap();
            let las_path = las_entry.unwrap();

            println!("{:?} - {:?}", las_path, laz_path);

            if las_path.file_stem().unwrap() == las_path.file_stem().unwrap() {
                let laz_file = BufReader::new(File::open(laz_path).unwrap());
                let las_file = BufReader::new(File::open(las_path).unwrap());

                check_decompression(laz_file, las_file);
            }
        }
    } else if Path::new(&args[1]).is_file() && Path::new(&args[2]).is_file() {
        let laz_file = BufReader::new(File::open(&args[1]).unwrap());
        let las_file = BufReader::new(File::open(&args[2]).unwrap());

        check_decompression(laz_file, las_file);
    } else {
        println!("Arguments must both be either path to file or path to directory");
    }
}
