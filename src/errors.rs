//! Definitions of error related thins.

use crate::las::laszip::{CompressorType, LazItemType};
use std::fmt;

/// Errors of this crate
#[derive(Debug)]
#[non_exhaustive]
pub enum LasZipError {
    /// The Laz item it not known
    UnknownLazItem(u16),
    /// The compression version used for the item is not supported
    UnsupportedLazItemVersion(LazItemType, u16),
    /// The type of compressor used is not known
    UnknownCompressorType(u16),
    /// The type of compressor exists but it is not supported
    UnsupportedCompressorType(CompressorType),
    /// The point format id is not supported
    UnsupportedPointFormat(u8),
    /// Wrapper around and io error from the std lib
    IoError(std::io::Error),
    BufferLenNotMultipleOfPointSize {
        buffer_len: usize,
        point_size: usize,
    },
    /// The chunk table could not be found in the file
    /// and it is required for the operation.
    MissingChunkTable,
}

impl From<std::io::Error> for LasZipError {
    fn from(e: std::io::Error) -> Self {
        LasZipError::IoError(e)
    }
}

impl fmt::Display for LasZipError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            LasZipError::UnknownLazItem(t) => write!(f, "Item with type code: {} is unknown", t),
            LasZipError::UnsupportedLazItemVersion(item_type, version) => write!(
                f,
                "Item {:?} with compression version: {} is not supported",
                item_type, version
            ),
            LasZipError::UnknownCompressorType(compressor_type) => {
                write!(f, "Compressor type {} is not valid", compressor_type)
            }
            LasZipError::UnsupportedCompressorType(compressor_type) => {
                write!(f, "Compressor type {:?} is not supported", compressor_type)
            }
            LasZipError::IoError(e) => write!(f, "IoError: {}", e),

            LasZipError::BufferLenNotMultipleOfPointSize {
                buffer_len: bl,
                point_size: ps,
            } => write!(
                f,
                "The len of the buffer ({}) is not a multiple of the point size {}",
                bl, ps
            ),
            LasZipError::UnsupportedPointFormat(id) => {
                write!(f, "Point format {} is not supported", id)
            }
            LasZipError::MissingChunkTable => write!(f, "The chunk table could not be found"),
        }
    }
}

impl std::error::Error for LasZipError {}
