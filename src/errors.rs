use crate::las::laszip::{CompressorType, LazItemType};
use std::fmt;

#[derive(Debug)]
pub enum LasZipError {
    UnknownLazItem(u16),
    UnsupportedLazItemVersion(LazItemType, u16),
    UnknownCompressorType(u16),
    UnsupportedCompressorType(CompressorType),
    IoError(std::io::Error),
    BufferLenNotMultipleOfPointSize {
        buffer_len: usize,
        point_size: usize,
    },
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
        }
    }
}
