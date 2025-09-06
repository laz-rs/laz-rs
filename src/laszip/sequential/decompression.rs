use std::io::{Read, Seek, SeekFrom};

use crate::errors::LasZipError::MissingChunkTable;
use crate::las::selective::DecompressionSelection;
use crate::record::RecordDecompressor;
use crate::LasZipError;

use crate::laszip::chunk_table::ChunkTable;
use crate::laszip::{details, CompressorType, LazVlr};

/// SeekInfo aggregates the two information needed to be able to seek
/// to a point in a las file.
pub(super) struct SeekInfo {
    // offset to the first point
    pub(super) data_start: u64,
    pub(super) chunk_table: ChunkTable,
}

impl SeekInfo {
    pub(super) fn read_from<T: Read + Seek>(
        mut source: &mut T,
        vlr: &LazVlr,
    ) -> crate::Result<Self> {
        let chunk_table = ChunkTable::read_from(&mut source, vlr)?;
        let data_start = source.seek(SeekFrom::Current(0))?;

        Ok(Self {
            data_start,
            chunk_table,
        })
    }

    /// Returns the offset where the chunk table is written
    pub(super) fn offset_to_chunk_table(&self) -> u64 {
        self.data_start
            + self
                .chunk_table
                .as_ref()
                .iter()
                .map(|e| e.byte_count)
                .sum::<u64>()
    }
}

/// LasZip decompressor that decompresses points.
///
/// Supports both **fixed-size** and **variable-size** chunks.
pub struct LasZipDecompressor<'a, R: Read + Seek + 'a> {
    vlr: LazVlr,
    record_decompressor: Box<dyn RecordDecompressor<R> + Send + Sync + 'a>,
    // Contains which fields the user wants to decompress or not
    selection: DecompressionSelection,
    // Allowed to be None if the source was not seekable and
    // chunks are not of variable size
    seek_info: Option<SeekInfo>,
    current_chunk: usize,
    chunk_points_read: u64,
    num_points_in_chunk: u64,
}

impl<'a, R: Read + Seek + Send + Sync + 'a> LasZipDecompressor<'a, R> {
    /// Creates a new instance from a data source of compressed points
    /// and the LazVlr describing the compressed data.
    ///
    /// The created decompressor will decompress all data
    pub fn new(source: R, vlr: LazVlr) -> crate::Result<Self> {
        Self::selective(source, vlr, DecompressionSelection::all())
    }

    /// Creates a new decompressor, that will only decompress
    /// fields that are selected by the `selection`.
    pub fn selective(
        mut source: R,
        vlr: LazVlr,
        selection: DecompressionSelection,
    ) -> crate::Result<Self> {
        // The chunk table is not always mandatory when just reading data.
        let seek_info = match vlr.compressor {
            CompressorType::PointWise => {
                // Everything is in one chunk, so we don't need a table
                None
            }
            CompressorType::PointWiseChunked => {
                let result = SeekInfo::read_from(&mut source, &vlr);
                match (result, vlr.uses_variable_size_chunks()) {
                    (Ok(info), _) => Some(info),
                    (Err(_), false) => {
                        // The error is probably due to a seek error
                        // (Eg for a source that is not actually seekable).
                        // So we _may_ still be at the start of point data,
                        // we need to skip the chunk table offset otherwise
                        // decompression won't be correct.
                        let mut tmp = [0u8; ChunkTable::OFFSET_SIZE];
                        source.read_exact(&mut tmp)?;
                        None
                    }
                    (Err(err), true) => {
                        // For for point wise chunked with variable size chunks,
                        // we absolutely need the chunk table to be able to know when a
                        // chunk ends.
                        return Err(err);
                    }
                }
            }
            CompressorType::LayeredChunked => {
                // Layered Chunks embeds the point count at the start of the chunk
                // So its fine if we don't have a chunk table
                let seek_info = SeekInfo::read_from(&mut source, &vlr).ok();
                if seek_info.is_none() {
                    // Same as in PointWiseChunked
                    let mut tmp = [0u8; ChunkTable::OFFSET_SIZE];
                    source.read_exact(&mut tmp)?;
                }
                seek_info
            }
            _ => {
                return Err(LasZipError::UnsupportedCompressorType(vlr.compressor));
            }
        };

        let mut record_decompressor =
            details::record_decompressor_from_laz_items(&vlr.items(), source)?;
        record_decompressor.set_selection(selection);

        Ok(Self {
            vlr,
            record_decompressor,
            selection,
            seek_info,
            current_chunk: 0,
            chunk_points_read: 0,
            num_points_in_chunk: 1,
        })
    }

    /// Decompress the next point and write the uncompressed data to the out buffer.
    ///
    /// - The buffer should have at least enough byte to store the decompressed data
    /// - The data is written in the buffer exactly as it would have been in a LAS File
    ///   in Little Endian order,
    pub fn decompress_one(&mut self, mut out: &mut [u8]) -> std::io::Result<()> {
        if self.chunk_points_read == self.num_points_in_chunk {
            self.reset_for_new_chunk();
            self.current_chunk += 1;
        }

        self.record_decompressor.decompress_next(&mut out)?;
        self.chunk_points_read += 1;

        if self.chunk_points_read == 1 {
            if self.vlr.uses_variable_size_chunks() {
                self.num_points_in_chunk = match (&self.seek_info, self.vlr.compressor) {
                    (Some(seek_info), _) => seek_info.chunk_table[self.current_chunk].point_count,
                    (None, CompressorType::LayeredChunked) => {
                        self.record_decompressor.record_count()
                    }
                    (None, _) => {
                        // This should not be possible, the `new` method should ensure
                        // we have the chunk table if we need one
                        panic!("Variable-size chunks, but no chunk table");
                    }
                }
            } else if self.vlr.compressor == CompressorType::PointWise {
                self.num_points_in_chunk = u64::from(u32::MAX);
            } else {
                self.num_points_in_chunk = u64::from(self.vlr.chunk_size());
            }
        }
        Ok(())
    }

    /// Decompress as many points as the `out` slice can hold
    ///
    /// # Note
    ///
    /// If the `out` slice contains more space than there are points
    /// the function will still decompress and thus and error will occur
    pub fn decompress_many(&mut self, out: &mut [u8]) -> std::io::Result<()> {
        for point in out.chunks_exact_mut(self.vlr.items_size() as usize) {
            self.decompress_one(point)?;
        }
        Ok(())
    }

    /// Seeks to the point designed by the index
    ///
    /// # Important
    ///
    /// Seeking in compressed data has a higher cost than non compressed data
    /// because the stream has to be moved to the start of the chunk
    /// and then we have to decompress points in the chunk until we reach the
    /// one we want.
    pub fn seek(&mut self, point_idx: u64) -> crate::Result<()> {
        let SeekInfo {
            data_start,
            chunk_table,
        } = self.seek_info.as_ref().ok_or(MissingChunkTable)?;

        let chunk_info = chunk_table
            .chunk_of_point(point_idx)
            .map(|(idx, offset)| (idx, offset + data_start));

        if let Some((chunk_of_point, start_of_chunk)) = chunk_info {
            self.current_chunk = chunk_of_point as usize;
            let delta = point_idx % chunk_table[self.current_chunk].point_count;
            let seeked_point_belong_to_last_chunk = chunk_of_point == (chunk_table.len() - 1);
            // When the index of the point belongs to the last chunk
            // we try to be careful as that point may not exist, and there may be a
            // case where we would be reading the chunk table data as if it were point data.
            //
            // In layered chunked, this is not a potential issue as the record decompressor
            // will first read in memory the chunk data (so only point data).
            //
            // This is a safeguard, but we expect caller to not do this mistake
            // (They should have access to the number of points in the file, we don't)
            if seeked_point_belong_to_last_chunk
                && self.vlr.compressor != CompressorType::LayeredChunked
            {
                let mut tmp_out = vec![0u8; self.record_decompressor.record_size()];
                self.get_mut().seek(SeekFrom::Start(start_of_chunk))?;

                self.reset_for_new_chunk();
                let offset_to_chunk_table = self
                    .seek_info
                    .as_ref()
                    .map(SeekInfo::offset_to_chunk_table)
                    .unwrap();

                for _i in 0..delta {
                    self.decompress_one(&mut tmp_out)?;
                    let current_pos = self.get_mut().seek(SeekFrom::Current(0))?;

                    if current_pos >= offset_to_chunk_table {
                        self.get_mut().seek(SeekFrom::End(0))?;
                        return Ok(());
                    }
                }
            } else {
                self.get_mut().seek(SeekFrom::Start(start_of_chunk))?;
                self.reset_for_new_chunk();
                let mut tmp_out = vec![0u8; self.record_decompressor.record_size()];

                for _i in 0..delta {
                    self.decompress_one(&mut tmp_out)?;
                }
            }
        } else {
            // the requested point it out of bounds (ie higher than the number of
            // points compressed)

            // Seek to the end so that the next call to decompress causes en error
            // like "Failed to fill whole buffer (seeking past end is allowed by the Seek Trait)
            self.record_decompressor.get_mut().seek(SeekFrom::End(0))?;
        }
        Ok(())
    }

    /// Returns the vlr used.
    pub fn vlr(&self) -> &LazVlr {
        &self.vlr
    }

    /// Consumes the decompressor and returns the data source.
    pub fn into_inner(self) -> R {
        self.record_decompressor.box_into_inner()
    }

    pub fn get_mut(&mut self) -> &mut R {
        self.record_decompressor.get_mut()
    }

    /// Returns a reference to the data source.
    pub fn get(&self) -> &R {
        self.record_decompressor.get()
    }

    #[inline(always)]
    fn reset_for_new_chunk(&mut self) {
        self.chunk_points_read = 0;
        self.record_decompressor.reset();
        // we can safely unwrap here, as set_field would have failed in the ::new()
        self.record_decompressor
            .set_fields_from(&self.vlr.items())
            .unwrap();
        self.record_decompressor.set_selection(self.selection);
    }
}

impl<'a, R: Read + Seek + Send + Sync + 'a> crate::LazDecompressor for LasZipDecompressor<'a, R> {
    fn decompress_one(&mut self, point: &mut [u8]) -> crate::Result<()> {
        LasZipDecompressor::decompress_one(self, point)?;
        Ok(())
    }

    fn decompress_many(&mut self, points: &mut [u8]) -> crate::Result<()> {
        LasZipDecompressor::decompress_many(self, points)?;
        Ok(())
    }

    fn seek(&mut self, index: u64) -> crate::Result<()> {
        LasZipDecompressor::seek(self, index)?;
        Ok(())
    }
}

/// Decompresses all points from the buffer
///
/// The `compressed_points_data` slice must contain all the laszip data
/// that means:
///   1) The offset to the chunk table (i64)
///   2) the compressed points
///   3) the chunk table (optional)
///
///
/// This fn will decompress as many points as the `decompress_points` can hold.
///
/// # Important
///
/// In a LAZ file, the chunk table offset is counted from the start of the
/// LAZ file. Here since we only have the buffer points data, you must make
/// sure the offset is counted since the start of point data.
///
/// So you should update the value before calling this function.
/// Otherwise you will get an IoError like 'failed to fill whole buffer'
/// due to this function seeking past the end of the data.
pub fn decompress_buffer(
    compressed_points_data: &[u8],
    decompressed_points: &mut [u8],
    laz_vlr: LazVlr,
) -> crate::Result<()> {
    debug_assert_eq!(decompressed_points.len() % laz_vlr.items_size() as usize, 0);
    let src = std::io::Cursor::new(compressed_points_data);
    LasZipDecompressor::new(src, laz_vlr).and_then(|mut decompressor| {
        decompressor.decompress_many(decompressed_points)?;
        Ok(())
    })
}
