use std::error::Error;

use clap::Clap;
use glob::GlobError;
use indicatif::{ProgressBar, ProgressStyle};

use laz::las::file::read_header_and_vlrs;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom, Write};

fn progress_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {percent} {msg}")
        .progress_chars("##-")
}

trait DecompressorCreator<'a, R: Read + Seek + Send> {
    type Decompressor: LazDecompressor;

    fn create(source: &'a mut R, vlr: LazVlr) -> Self::Decompressor;
}

trait CompressorCreator<'a, R: Write + Seek + Send> {
    type Compressor: LazCompressor;

    fn create(source: &'a mut R, vlr: LazVlr) -> Self::Compressor;
}

#[cfg(not(feature = "parallel"))]
mod details {
    use std::io::{Read, Seek, Write};

    pub struct SimpleDecompressorCreator;

    impl<'a, R: Read + Seek + Send + 'a> super::DecompressorCreator<'a, R>
        for SimpleDecompressorCreator
    {
        type Decompressor = laz::LasZipDecompressor<'a, &'a mut R>;

        fn create(source: &'a mut R, vlr: laz::LazVlr) -> Self::Decompressor {
            laz::LasZipDecompressor::new(source, vlr).unwrap()
        }
    }

    pub struct SimpleCompressorCreator;

    impl<'a, R: Write + Seek + Send + 'a> super::CompressorCreator<'a, R> for SimpleCompressorCreator {
        type Compressor = laz::LasZipCompressor<'a, &'a mut R>;

        fn create(source: &'a mut R, vlr: laz::LazVlr) -> Self::Compressor {
            laz::LasZipCompressor::new(source, vlr).unwrap()
        }
    }
}

#[cfg(feature = "parallel")]
mod details {
    use std::io::{Read, Seek, Write};

    pub struct ParDecompressorCreator;

    impl<'a, R: Read + Seek + Send + 'a> super::DecompressorCreator<'a, R> for ParDecompressorCreator {
        type Decompressor = laz::ParLasZipDecompressor<&'a mut R>;

        fn create(source: &'a mut R, vlr: laz::LazVlr) -> Self::Decompressor {
            laz::ParLasZipDecompressor::new(source, vlr).unwrap()
        }
    }

    pub struct ParCompressorCreator;

    impl<'a, R: Write + Seek + Send + 'a> super::CompressorCreator<'a, R> for ParCompressorCreator {
        type Compressor = laz::ParLasZipCompressor<&'a mut R>;

        fn create(source: &'a mut R, vlr: laz::LazVlr) -> Self::Compressor {
            laz::ParLasZipCompressor::new(source, vlr).unwrap()
        }
    }
}

use crate::details::*;

#[cfg(feature = "parallel")]
type DefaultCompressorCreator = ParCompressorCreator;
#[cfg(feature = "parallel")]
type DefaultDecompressorCreator = ParDecompressorCreator;

#[cfg(not(feature = "parallel"))]
type DefaultCompressorCreator = SimpleCompressorCreator;
#[cfg(not(feature = "parallel"))]
type DefaultDecompressorCreator = SimpleDecompressorCreator;

#[derive(Clap)]
struct Arguments {
    path: String,
    num_points_per_iter: Option<i64>,
}

use laz::las::laszip::{LazCompressor, LazDecompressor};
use laz::LazVlr;

fn run_check_2<Decompressor1, Decompressor2, Compressor>(
    las_path: &String,
    laz_path: &String,
    args: &Arguments,
) -> laz::Result<()>
where
    Decompressor1: for<'a> DecompressorCreator<'a, BufReader<File>>,
    Decompressor2: for<'a> DecompressorCreator<'a, Cursor<Vec<u8>>>,
    Compressor: for<'a> CompressorCreator<'a, Cursor<Vec<u8>>>,
{
    let mut las_file = BufReader::new(File::open(las_path)?);
    let (las_header, _) = read_header_and_vlrs(&mut las_file)?;

    let mut laz_file = BufReader::new(File::open(laz_path)?);
    let (laz_header, laz_vlr) = read_header_and_vlrs(&mut laz_file)?;
    let laz_vlr = laz_vlr.expect("Expected a laszip VLR for laz file");

    assert_eq!(las_header.point_size, laz_header.point_size);
    assert_eq!(las_header.num_points, laz_header.num_points);

    let progress = ProgressBar::new(las_header.num_points as u64);
    progress.set_draw_delta(las_header.num_points as u64 / 100);
    progress.set_style(progress_style());

    assert!(args.num_points_per_iter.unwrap() > 0);
    let num_points_per_iter = args.num_points_per_iter.unwrap() as usize;

    let point_size = las_header.point_size as usize;
    let mut our_point = vec![0u8; point_size * num_points_per_iter];
    let mut expected_point = vec![0u8; point_size * num_points_per_iter];
    let mut num_points_left = las_header.num_points as usize;

    progress.tick();
    progress.set_message("[1/3] Checking decompression");
    {
        let mut decompressor = Decompressor1::create(&mut laz_file, laz_vlr.clone());
        while num_points_left > 0 {
            let num_points_to_read = num_points_per_iter.min(num_points_left);

            let our_points = &mut our_point[..num_points_to_read * point_size];
            let expected_points = &mut expected_point[..num_points_to_read * point_size];

            las_file.read_exact(expected_points)?;
            decompressor.decompress_many(our_points)?;

            assert_eq!(our_points, expected_points);

            num_points_left -= num_points_to_read;
            progress.inc(num_points_to_read as u64);
        }
    }

    // Check our compression
    progress.set_position(0);
    progress.set_message("[2/3] Compressing");
    progress.tick();
    las_file.seek(SeekFrom::Start(las_header.offset_to_points as u64))?;
    num_points_left = las_header.num_points as usize;
    let mut compressed_data = Cursor::new(Vec::<u8>::new());
    {
        let mut compressor = Compressor::create(&mut compressed_data, laz_vlr.clone());
        while num_points_left > 0 {
            let num_points_to_read = num_points_per_iter.min(num_points_left);

            let our_points = &mut our_point[..num_points_to_read * point_size];
            las_file.read_exact(our_points)?;
            compressor.compress_many(our_points)?;

            num_points_left -= num_points_to_read;
            progress.inc(num_points_to_read as u64);
        }
        compressor.done()?;
    }

    compressed_data.set_position(0);
    progress.set_message("[3/3] Checking decompression");
    progress.tick();
    las_file.seek(SeekFrom::Start(las_header.offset_to_points as u64))?;
    num_points_left = las_header.num_points as usize;
    {
        let mut decompressor = Decompressor2::create(&mut compressed_data, laz_vlr.clone());
        while num_points_left > 0 {
            let num_points_to_read = num_points_per_iter.min(num_points_left);

            let our_points = &mut our_point[..num_points_to_read * point_size];
            let expected_points = &mut expected_point[..num_points_to_read * point_size];

            las_file.read_exact(expected_points)?;
            decompressor.decompress_many(our_points)?;

            assert_eq!(our_points, expected_points);

            num_points_left -= num_points_to_read;
            progress.inc(num_points_to_read as u64);
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = Arguments::parse();

    if cfg!(feature = "parallel") {
        if args.num_points_per_iter.is_none() {
            args.num_points_per_iter = Some(890_908);
        }
    } else {
        if args.num_points_per_iter.is_none() {
            args.num_points_per_iter = Some(1);
        }
    }

    let laz_globber = glob::glob(&format!("{}/**/*.laz", &args.path))?;
    let las_globber = glob::glob(&format!("{}/**/*.las", &args.path))?;

    let las_paths = las_globber
        .into_iter()
        .map(|result| result.map(|path| path.to_str().unwrap().to_owned()))
        .collect::<Result<Vec<String>, GlobError>>()?;

    let laz_paths = laz_globber
        .into_iter()
        .map(|result| result.map(|path| path.to_str().unwrap().to_owned()))
        .collect::<Result<Vec<String>, GlobError>>()?;

    assert_eq!(laz_paths.len(), las_paths.len());
    let global_bar = ProgressBar::new(laz_paths.len() as u64);
    global_bar.set_style(progress_style());
    for (las_path, laz_path) in las_paths.into_iter().zip(laz_paths.into_iter()) {
        global_bar.set_message(format!("Checking {}", &las_path));
        // TODO impl Error for LasZipError

        run_check_2::<DefaultDecompressorCreator, DefaultDecompressorCreator, DefaultCompressorCreator>(
            &las_path, &laz_path, &args,
        )
        .unwrap();
        global_bar.inc(1);
        global_bar.println(format!("{}: Ok", &las_path))
    }
    global_bar.finish_with_message("Done.");

    Ok(())
}
