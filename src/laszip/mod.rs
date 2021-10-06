//! Module with the important struct that people wishing
//! to compress or decompress LAZ data can use
//!
//! It defines the LaszipCompressor & LaszipDecompressor
//! as well as the Laszip VLr data  and how to build it
pub use compression::{compress_buffer, LasZipCompressor};
pub use decompression::{decompress_buffer, LasZipDecompressor};
pub use vlr::{
    CompressorType, DefaultVersion, LazItem, LazItemRecordBuilder, LazItemType, LazItems, LazVlr,
    LazVlrBuilder, Version1, Version2, Version3,
};

mod chunk_table;
mod compression;
mod decompression;
mod details;
#[cfg(feature = "parallel")]
pub mod parallel;
mod vlr;

#[deprecated(since = "0.6.0", note = "Please use laz::LazVlr::USER_ID")]
pub const LASZIP_USER_ID: &str = LazVlr::USER_ID;
#[deprecated(since = "0.6.0", note = "Please use laz::LazVlr::RECORD_ID")]
pub const LASZIP_RECORD_ID: u16 = LazVlr::RECORD_ID;
#[deprecated(since = "0.6.0", note = "Please use laz::LazVlr::USER_ID")]
pub const LASZIP_DESCRIPTION: &str = LazVlr::DESCRIPTION;

pub trait LazDecompressor {
    fn decompress_many(&mut self, points: &mut [u8]) -> crate::Result<()>;

    fn seek(&mut self, index: u64) -> crate::Result<()>;
}

pub trait LazCompressor {
    fn compress_many(&mut self, points: &[u8]) -> crate::Result<()>;

    fn done(&mut self) -> crate::Result<()>;
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_create_laz_items() {
        assert_eq!(
            LazItemRecordBuilder::new()
                .add_item(LazItemType::Point10)
                .build()
                .len(),
            1
        );
    }

    macro_rules! test_manual_reserve_on {
        ($CompressorType:ty) => {
            // Check that by manually calling reserve, the result is the same than without calling it
            let vlr = super::LazVlr::from_laz_items(
                LazItemRecordBuilder::new()
                    .add_item(LazItemType::Point10)
                    .build(),
            );

            let point = vec![0u8; vlr.items_size() as usize];

            let data1 = {
                let mut compressor =
                    <$CompressorType>::new(std::io::Cursor::new(Vec::<u8>::new()), vlr.clone()).unwrap();
                compressor.compress_many(&point).unwrap();
                compressor.done().unwrap();
                compressor.into_inner().into_inner()
            };

            let data2 = {
                let mut compressor =
                    <$CompressorType>::new(std::io::Cursor::new(Vec::<u8>::new()), vlr.clone()).unwrap();
                compressor.reserve_offset_to_chunk_table().unwrap();
                compressor.compress_many(&point).unwrap();
                compressor.done().unwrap();
                compressor.into_inner().into_inner()
            };

            assert_eq!(data1, data2);
        };
    }

    #[test]
    fn test_manual_reserve() {
        test_manual_reserve_on!(LasZipCompressor<Cursor<Vec<u8>>>);
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn test_manual_reserve_par() {
        test_manual_reserve_on!(parallel::ParLasZipCompressor<Cursor<Vec<u8>>>);
    }
}
