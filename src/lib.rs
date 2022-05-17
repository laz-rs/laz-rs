//! Port of the Martin Isenburg's laszip compression to Rust
//!
//! [`LasZipCompressor`] and [`LasZipDecompressor`] are the two types
//! that user wishing to compress and / or decompress LAZ data should use.
//!
//! # LasZipCompressor Examples
//!
//! ```
//! use laz::{LasZipError, LasZipCompressor, LazItemType, LazItemRecordBuilder};
//!
//! # fn main() -> laz::Result<()> {
//! // Here we use a Cursor but a std::fs::File will work just fine
//! let mut compressed_output = std::io::Cursor::new(vec![]);
//!
//! // LazItem may have multiple versions of the compression algorithm
//! // the builder selects a default one
//! let items = LazItemRecordBuilder::new()
//!             .add_item(LazItemType::Point10)
//!             .add_item(LazItemType::RGB12)
//!             .build();
//! let mut compressor = LasZipCompressor::from_laz_items(&mut compressed_output, items)?;
//!
//! let point = vec![0u8; 26];
//! compressor.compress_one(&point)?;
//! compressor.done()?; // don't forget to call done when you are...done compressing
//!
//! # Ok(())
//! # }
//! ```
//!
//!
//! LasZipCompressors can also be contructed from a LazVlr if you need to change the Chunk size
//! or if you have the LazVlr from the orignal LAZ file that you want to write back
//! ```
//! use laz::{LasZipError, LasZipCompressor, LazItemType, LazItemRecordBuilder, LazVlrBuilder};
//!
//! # fn main() -> laz::Result<()> {
//!
//! let mut compressed_output = std::io::Cursor::new(vec![]);
//! let items = LazItemRecordBuilder::new()
//!             .add_item(LazItemType::Point10)
//!             .add_item(LazItemType::RGB12)
//!             .build();
//! let vlr = LazVlrBuilder::from_laz_items(items)
//!           .with_chunk_size(5_000)
//!           .build();
//!
//! let mut compressor = LasZipCompressor::new(&mut compressed_output, vlr)?;
//!
//! let point = vec![0u8; 26];
//! compressor.compress_one(&point)?;
//! compressor.done()?;
//! # Ok(())
//! # }
//! ```
//!
//! To create a [`LasZipDecompressor`] you need to have the record_data found in the LAZ file.
//!
//! # LasZipDecompressor Examples
//!
//! ```
//! # const LAS_HEADER_SIZE: u64 = 227;
//! # const VLR_HEADER_SIZE: u64 = 54;
//! # const OFFSET_TO_LASZIP_VLR_DATA: u64 = LAS_HEADER_SIZE + VLR_HEADER_SIZE;
//! # const NUM_POINTS: usize = 1065;
//!
//! use laz::{LasZipError, LazVlr, LasZipDecompressor};
//! use std::fs::File;
//!
//! # fn read_first_point(path: &str, out: &mut [u8]) -> std::io::Result<()> {
//! #    let mut reader = laz::las::file::SimpleReader::new(File::open(path)?)?;
//! #    out.copy_from_slice(reader.read_next().unwrap()?);
//! #    Ok(())
//! # }
//! # fn seek_to_start_of_laszip_record_data(file: &mut File) -> std::io::Result<()> {
//! #    use std::io::{Seek, SeekFrom};
//! #    file.seek(SeekFrom::Start(OFFSET_TO_LASZIP_VLR_DATA))?;
//! #    Ok(())
//! # }
//! # fn main() -> laz::Result<()> {
//! let mut laz_file = File::open("tests/data/point10.laz")?;
//! seek_to_start_of_laszip_record_data(&mut laz_file)?;
//!
//! let vlr = LazVlr::read_from(&mut laz_file)?;
//! let mut decompression_output = vec![0u8; vlr.items_size() as usize];
//! let mut decompressor = LasZipDecompressor::new(&mut laz_file, vlr)?;
//!
//! let mut ground_truth = vec![0u8; decompression_output.len()];
//! read_first_point("tests/data/point10.las", &mut ground_truth)?;
//!
//! decompressor.decompress_one(&mut decompression_output)?;
//! assert_eq!(&decompression_output, &ground_truth);
//!
//! # Ok(())
//! # }
//! ```
//!
//!
//! # Parallelism
//!
//! This crates has an optional feature 'parallel'.
//! When using this feature, additional `Par` structs and `par_` methods are exposed.
//!
//! - [`ParLasZipCompressor`]
//! - [`ParLasZipDecompressor`]
//!
//! - [`par_compress_buffer`]
//! - [`par_decompress_buffer`]

pub use errors::LasZipError;
#[cfg(feature = "parallel")]
pub use laszip::parallel::{
    par_compress_buffer, par_decompress, par_decompress_buffer, ParLasZipCompressor,
    ParLasZipDecompressor,
};
pub use laszip::{compress_buffer, decompress_buffer};
pub use laszip::{
    LasZipCompressor, LasZipDecompressor, LazCompressor, LazDecompressor, LazItem,
    LazItemRecordBuilder, LazItemType, LazVlr, LazVlrBuilder,
};

pub(crate) mod compressors;
pub(crate) mod decoders;
pub(crate) mod decompressors;
pub(crate) mod encoders;
pub mod laszip;
pub(crate) mod models;
pub mod packers;
pub mod record;

mod byteslice;
pub mod errors;
pub mod las;

#[cfg(test)]
mod test;

/// Shortcut for `Result`s of this crate.
pub type Result<T> = std::result::Result<T, LasZipError>;
