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
//! # fn main() -> Result<(), LasZipError> {
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
//! # fn main() -> Result<(), LasZipError> {
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
//! let mut compressor = LasZipCompressor::from_laz_vlr(&mut compressed_output, vlr)?;
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
//! # fn main() -> Result<(), LasZipError> {
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


pub(crate) mod compressors;
pub(crate) mod decoders;
pub(crate) mod decompressors;
pub(crate) mod encoders;
pub(crate) mod models;

pub mod checking;
pub mod errors;
pub mod las;
pub mod packers;
#[macro_use]
pub mod record;

pub use errors::LasZipError;
pub use las::laszip::{LasZipCompressor, LasZipDecompressor, LazItemType, LazItem, LazVlr, LazVlrBuilder, LazItemRecordBuilder};


use crate::las::laszip::compress_all as rust_compress_all;
use std::io::Cursor;
use std::cell::RefCell;

thread_local! {
    pub static LAST_ERROR_MESSAGE: RefCell<Option<String>> = RefCell::new(None);
}

#[repr(C)]
pub struct BytesBuffer {
    ptr: *mut u8,
    size: usize,
    capacity: usize,
}

#[no_mangle]
pub extern "C" fn free_bytes_buffer(bb: BytesBuffer) {
    unsafe {
        if !bb.ptr.is_null() {
            drop(Vec::from_raw_parts(bb.ptr, bb.size, bb.capacity));
        }
    }
}

#[no_mangle]
pub extern "C" fn compress_all(
    in_uncompressed_points: *const u8,
    uncompressed_points_buffer_size: usize,
    laszip_vlr_record_data: *const u8,
    record_data_size: usize,
) -> * mut BytesBuffer {

    let in_uncompressed_points = unsafe {
        std::slice::from_raw_parts(in_uncompressed_points, uncompressed_points_buffer_size)
    };

    let laszip_vlr_record_data = unsafe {
        std::slice::from_raw_parts(laszip_vlr_record_data, record_data_size)
    };

    let laz_vlr = match crate::las::laszip::LazVlr::from_buffer(laszip_vlr_record_data) {
        Ok(vlr) => vlr,
        Err(e) => {
            LAST_ERROR_MESSAGE.with(|value| value.replace_with(|_old| Some(format!("{}", e))));
            return std::ptr::null_mut();
        }
    };
    let mut compression_output = Cursor::new(Vec::<u8>::new());
    if let Err(e) = rust_compress_all(&mut compression_output, in_uncompressed_points, laz_vlr) {
        LAST_ERROR_MESSAGE.with(|value| value.replace_with(|_old| Some(format!("{}", e))));
        std::ptr::null_mut()
    } else {
        let mut vec = compression_output.into_inner();
        let bb = Box::new(BytesBuffer {
            ptr: vec.as_mut_ptr(),
            size: vec.len(),
            capacity: vec.capacity(),
        });
        std::mem::forget(vec);
        Box::into_raw(bb)
    }
}

