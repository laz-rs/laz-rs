use super::{details, CompressorType, LazVlr};
use crate::record::RecordDecompressor;
use crate::LasZipError;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};

/// Struct that handles the decompression of the points written in a LAZ file
pub struct LasZipDecompressor<'a, R: Read + Seek + 'a> {
    vlr: LazVlr,
    record_decompressor: Box<dyn RecordDecompressor<R> + Send + 'a>,
    chunk_points_read: u32,
    offset_to_chunk_table: i64,
    data_start: u64,
    chunk_table: Option<Vec<u64>>,
}

impl<'a, R: Read + Seek + Send + 'a> LasZipDecompressor<'a, R> {
    /// Creates a new instance from a data source of compressed points
    /// and the LazVlr describing the compressed data
    pub fn new(mut source: R, vlr: LazVlr) -> crate::Result<Self> {
        if vlr.compressor != CompressorType::PointWiseChunked
            && vlr.compressor != CompressorType::LayeredChunked
        {
            return Err(LasZipError::UnsupportedCompressorType(vlr.compressor));
        }

        let offset_to_chunk_table = source.read_i64::<LittleEndian>()?;
        let data_start = source.seek(SeekFrom::Current(0))?;
        let record_decompressor =
            details::record_decompressor_from_laz_items(&vlr.items(), source)?;

        Ok(Self {
            vlr,
            record_decompressor,
            chunk_points_read: 0,
            offset_to_chunk_table,
            data_start,
            chunk_table: None,
        })
    }

    /// Creates a new instance from a data source of compressed points
    /// and the `record data` of the laszip vlr
    pub fn new_with_record_data(source: R, laszip_vlr_record_data: &[u8]) -> crate::Result<Self> {
        let vlr = LazVlr::from_buffer(laszip_vlr_record_data)?;
        Self::new(source, vlr)
    }

    /// Decompress the next point and write the uncompressed data to the out buffer.
    ///
    /// - The buffer should have at least enough byte to store the decompressed data
    /// - The data is written in the buffer exactly as it would have been in a LAS File
    ///     in Little Endian order,
    pub fn decompress_one(&mut self, mut out: &mut [u8]) -> std::io::Result<()> {
        if self.chunk_points_read == self.vlr.chunk_size() {
            self.reset_for_new_chunk();
        }
        self.record_decompressor.decompress_next(&mut out)?;
        self.chunk_points_read += 1;
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

    pub fn vlr(&self) -> &LazVlr {
        &self.vlr
    }

    /// Seeks to the point designed by the index
    ///
    /// # Important
    ///
    /// Seeking in compressed data has a higher cost than non compressed data
    /// because the stream has to be moved to the start of the chunk
    /// and then we have to decompress points in the chunk until we reach the
    /// one we want.
    pub fn seek(&mut self, point_idx: u64) -> std::io::Result<()> {
        if let Some(chunk_table) = &self.chunk_table {
            let chunk_of_point = point_idx / self.vlr.chunk_size() as u64;
            let delta = point_idx % self.vlr.chunk_size() as u64;

            if chunk_of_point == (chunk_table.len() - 1) as u64 {
                // the requested point fall into the last chunk,
                // but that does not mean that the point exists
                // so we have to be careful, we will do as we would normally,
                // but if we reach the chunk_table_offset that means the requested
                // point is out ouf bounds so will just seek to the end cf(the else in the if let below)
                // we do this to avoid decompressing data (ie the chunk table) thinking its a record

                if self.offset_to_chunk_table == -1 {
                    // If the offset is still -1 it means the chunk table could not
                    // be read :thinking:
                    unreachable!("unexpected offset to chunk table");
                }
                let mut tmp_out = vec![0u8; self.record_decompressor.record_size()];
                self.record_decompressor
                    .get_mut()
                    .seek(SeekFrom::Start(chunk_table[chunk_of_point as usize] as u64))?;

                self.reset_for_new_chunk();

                for _i in 0..delta {
                    self.decompress_one(&mut tmp_out)?;
                    let current_pos = self
                        .record_decompressor
                        .get_mut()
                        .seek(SeekFrom::Current(0))?;

                    if current_pos >= self.offset_to_chunk_table as u64 {
                        self.record_decompressor.get_mut().seek(SeekFrom::End(0))?;
                        return Ok(());
                    }
                }
            } else if let Some(start_of_chunk) = chunk_table.get(chunk_of_point as usize) {
                self.record_decompressor
                    .get_mut()
                    .seek(SeekFrom::Start(*start_of_chunk as u64))?;
                self.reset_for_new_chunk();
                let mut tmp_out = vec![0u8; self.record_decompressor.record_size()];

                for _i in 0..delta {
                    self.decompress_one(&mut tmp_out)?;
                }
            } else {
                // the requested point it out of bounds (ie higher than the number of
                // points compressed)

                // Seek to the end so that the next call to decompress causes en error
                // like "Failed to fill whole buffer (seeking past end is allowed by the Seek Trait)
                self.record_decompressor.get_mut().seek(SeekFrom::End(0))?;
            }
            Ok(())
        } else {
            self.read_chunk_table()?;
            self.seek(point_idx)
        }
    }

    fn reset_for_new_chunk(&mut self) {
        self.chunk_points_read = 0;
        self.record_decompressor.reset();
        //we can safely unwrap here, as set_field would have failed in the ::new()
        self.record_decompressor
            .set_fields_from(&self.vlr.items())
            .unwrap();
    }

    fn read_chunk_table(&mut self) -> std::io::Result<()> {
        let stream = self.record_decompressor.get_mut();
        let chunk_sizes = details::read_chunk_table_at_offset(stream, self.offset_to_chunk_table)?;
        let number_of_chunks = chunk_sizes.len();
        let mut chunk_starts = vec![0u64; number_of_chunks as usize];
        chunk_starts[0] = self.data_start;
        for i in 1..number_of_chunks {
            chunk_starts[i as usize] =
                chunk_sizes[(i - 1) as usize] + chunk_starts[(i - 1) as usize];
        }
        self.chunk_table = Some(chunk_starts);
        Ok(())
    }

    pub fn into_inner(self) -> R {
        self.record_decompressor.box_into_inner()
    }

    pub fn get_mut(&mut self) -> &mut R {
        self.record_decompressor.get_mut()
    }

    pub fn get(&self) -> &R {
        self.record_decompressor.get()
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
    let point_size = laz_vlr.items_size() as usize;
    if decompressed_points.len() % point_size != 0 {
        Err(LasZipError::BufferLenNotMultipleOfPointSize {
            buffer_len: decompressed_points.len(),
            point_size,
        })
    } else {
        let src = std::io::Cursor::new(compressed_points_data);
        LasZipDecompressor::new(src, laz_vlr).and_then(|mut decompressor| {
            decompressor.decompress_many(decompressed_points)?;
            Ok(())
        })
    }
}
