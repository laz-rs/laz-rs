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
//! let vlr = LazVlrBuilder::new()
//!           .with_laz_items(items)
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
//! [`LasZipCompressor`]: las/laszip/struct.LasZipCompressor.html
//! [`LasZipDecompressor`]: las/laszip/struct.LasZipDecompressor.html
//!
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
//!
//!
//! [`ParLasZipCompressor`]: las/laszip/struct.ParLasZipCompressor.html
//! [`ParLasZipDecompressor`]: las/laszip/struct.ParLasZipDecompressor.html
//! [`par_compress_buffer`]: las/laszip/fn.par_compress_buffer.html
//! [`par_decompress_buffer`]: las/laszip/fn.par_decompress_buffer.html

pub(crate) mod compressors;
pub(crate) mod decoders;
pub(crate) mod decompressors;
pub(crate) mod encoders;
pub(crate) mod models;

mod byteslice;
pub mod checking;
pub mod errors;
pub mod las;
pub mod packers;
#[macro_use]
pub mod record;

pub use errors::LasZipError;
pub use las::laszip::{compress_buffer, decompress_buffer};

#[cfg(feature = "parallel")]
pub use las::laszip::{
    par_compress_buffer, par_decompress_buffer, ParLasZipCompressor, ParLasZipDecompressor,
};

pub use las::laszip::{
    read_chunk_table, write_chunk_table, LasZipCompressor, LasZipDecompressor, LazItem,
    LazItemRecordBuilder, LazItemType, LazVlr, LazVlrBuilder,
};

pub type Result<T> = std::result::Result<T, LasZipError>;

use las::file::read_vlrs_and_get_laszip_vlr;
use las::file::QuickHeader;
use wasm_bindgen::prelude::*;
use std::io::Seek;
extern crate console_error_panic_hook;

#[wasm_bindgen]
pub fn get_header(buf: js_sys::Uint8Array)  -> std::result::Result<QuickHeader, JsValue> {
    // initialize debugging
    console_error_panic_hook::set_once();

    // copy header bytes into wasm memory
    let mut body = vec![0; buf.length() as usize];
    buf.copy_to(&mut body[..]);
    // cursor to wrap the bytes
    let mut cursor = std::io::Cursor::new(body);
    let hdr = QuickHeader::read_from(&mut cursor).unwrap();
    Ok(hdr)
}

#[wasm_bindgen]
pub struct WasmLasZipDecompressor {
    decompressor: las::laszip::LasZipDecompressor<'static, std::io::Cursor<Vec<u8>>>,
}

impl WasmLasZipDecompressor {
    pub fn new(source: Vec<u8>) -> Self {        
        let mut cursor = std::io::Cursor::new(source);

        let hdr = QuickHeader::read_from(&mut cursor).unwrap();
        cursor.seek(std::io::SeekFrom::Start(hdr.header_size as u64));
        let laz_vlr = read_vlrs_and_get_laszip_vlr(&mut cursor, &hdr);
        cursor.seek(std::io::SeekFrom::Start(hdr.offset_to_points as u64));
        let decomp = las::laszip::LasZipDecompressor::new(cursor, laz_vlr.expect("Compressed data, but no Laszip Vlr found")).unwrap();
        
        Self {
            decompressor: decomp,
        }
    }

    pub fn decompress_many(&mut self, out: &mut [u8]) -> std::io::Result<()> {
        Ok(self.decompressor.decompress_many(out)?)
    }
}

#[wasm_bindgen]
pub fn init_decompressor(buf: js_sys::Uint8Array)  -> WasmLasZipDecompressor  {
    WasmLasZipDecompressor::new(buf.to_vec())
}

#[wasm_bindgen]
pub fn decompress_many(decompressor: &mut WasmLasZipDecompressor, out: &mut [u8]) {
    decompressor.decompressor.decompress_many(out);
}