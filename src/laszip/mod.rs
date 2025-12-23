//! Module with the important struct that people wishing
//! to compress or decompress LAZ data can use
//!
//! It defines the LaszipCompressor & LaszipDecompressor
//! as well as the Laszip VLr data  and how to build it
pub use chunk_table::{ChunkTable, ChunkTableEntry};
pub use sequential::{
    compress_buffer, decompress_buffer, LasZipAppender, LasZipCompressor, LasZipDecompressor,
};
pub use vlr::{
    CompressorType, DefaultVersion, LazItem, LazItemRecordBuilder, LazItemType, LazVlr,
    LazVlrBuilder, Version1, Version2, Version3,
};

#[cfg(feature = "parallel")]
pub(crate) use vlr::DecompressedChunkSize;

mod chunk_table;
mod details;
#[cfg(feature = "parallel")]
pub mod parallel;
mod sequential;
mod vlr;

#[deprecated(since = "0.6.0", note = "Please use laz::LazVlr::USER_ID")]
pub const LASZIP_USER_ID: &str = LazVlr::USER_ID;
#[deprecated(since = "0.6.0", note = "Please use laz::LazVlr::RECORD_ID")]
pub const LASZIP_RECORD_ID: u16 = LazVlr::RECORD_ID;
#[deprecated(since = "0.6.0", note = "Please use laz::LazVlr::DESCRIPTION")]
pub const LASZIP_DESCRIPTION: &str = LazVlr::DESCRIPTION;

pub trait LazDecompressor {
    fn decompress_one(&mut self, point: &mut [u8]) -> crate::Result<()>;

    fn decompress_many(&mut self, points: &mut [u8]) -> crate::Result<()>;

    fn seek(&mut self, index: u64) -> crate::Result<()>;
}

pub trait LazCompressor {
    fn compress_one(&mut self, point: &[u8]) -> crate::Result<()>;

    fn compress_many(&mut self, points: &[u8]) -> crate::Result<()>;

    fn reserve_offset_to_chunk_table(&mut self) -> crate::Result<()>;

    fn done(&mut self) -> crate::Result<()>;
}

pub trait LazCompressorOwned<W>: LazCompressor {
    fn into_inner(self) -> W;

    fn inner(&self) -> &W;

    fn inner_mut(&mut self) -> &mut W;
}

#[cfg(test)]
mod test {
    use std::io::{Cursor, Seek, SeekFrom};

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

    #[test]
    fn test_compress_empty_buffer() {
        let vlr = super::LazVlr::from_laz_items(
            LazItemRecordBuilder::new()
                .add_item(LazItemType::Point10)
                .build(),
        );
        let header = vec![42; 10];
        let mut write = Cursor::new(header.clone());
        write.seek(SeekFrom::Start(header.len() as u64)).unwrap();
        compress_buffer(&mut write, &[], vlr).unwrap();
        let data = write.into_inner();
        assert!(data.starts_with(&header));
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
