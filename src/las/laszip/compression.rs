use super::{details, LazVlr};
use crate::record::RecordCompressor;
use crate::{LasZipError, LazItem};
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{Seek, SeekFrom, Write};

/// Struct that handles the compression of the points into the given destination
pub struct LasZipCompressor<'a, W: Write + Send + 'a> {
    vlr: LazVlr,
    /// compressor used for the current chunk
    record_compressor: Box<dyn RecordCompressor<W> + Send + 'a>,
    /// How many points in the current chunk
    chunk_point_written: u32,
    /// Size in bytes of each chunks written so far
    chunk_sizes: Vec<usize>,
    /// Position (offset from beginning)
    /// where the current chunk started
    chunk_start_pos: u64,
    /// Position where LasZipCompressor started
    start_pos: u64,
}

impl<'a, W: Write + Seek + Send + 'a> LasZipCompressor<'a, W> {
    /// Creates a compressor using the provided vlr.
    pub fn new(output: W, vlr: LazVlr) -> crate::Result<Self> {
        let record_compressor = details::record_compressor_from_laz_items(&vlr.items(), output)?;
        Ok(Self {
            vlr,
            record_compressor,
            chunk_point_written: 0,
            chunk_sizes: vec![],
            chunk_start_pos: 0,
            start_pos: 0,
        })
    }

    /// Creates a new LasZipCompressor using the items provided,
    ///
    /// If you wish to use a different `chunk size` see [`from_laz_vlr`]
    ///
    /// [`from_laz_vlr`]: #method.from_laz_vlr
    pub fn from_laz_items(output: W, items: Vec<LazItem>) -> crate::Result<Self> {
        let vlr = LazVlr::from_laz_items(items);
        Self::new(output, vlr)
    }

    /// Reserves and prepares the offset to chunk table that will be
    /// updated when [done] is called.
    ///
    /// This method will automatically be called on the first point being compressed,
    /// but for some scenarios, manually calling this might be useful.
    ///
    /// [done]: Self::done
    pub fn reserve_offset_to_chunk_table(&mut self) -> std::io::Result<()> {
        let stream = self.record_compressor.get_mut();
        self.start_pos = stream.seek(SeekFrom::Current(0))?;
        stream.write_i64::<LittleEndian>(-1)?;
        self.chunk_start_pos = self.start_pos + std::mem::size_of::<i64>() as u64;
        Ok(())
    }

    /// Compress the point and write the compressed data to the destination given when
    /// the compressor was constructed
    ///
    /// The data is written in the buffer is expected to be exactly
    /// as it would have been in a LAS File, that is:
    ///
    /// - The fields/dimensions are in the same order than the LAS spec says
    /// - The data in the buffer is in Little Endian order
    pub fn compress_one(&mut self, input: &[u8]) -> std::io::Result<()> {
        if self.chunk_start_pos == 0 {
            self.reserve_offset_to_chunk_table()?;
        }

        if self.chunk_point_written == self.vlr.chunk_size() {
            self.record_compressor.done()?;
            self.record_compressor.reset();
            self.record_compressor
                .set_fields_from(&self.vlr.items())
                .unwrap();
            self.update_chunk_table()?;
            self.chunk_point_written = 0;
        }

        self.record_compressor.compress_next(&input)?;
        self.chunk_point_written += 1;
        Ok(())
    }

    /// Compress all the points contained in the `input` slice
    pub fn compress_many(&mut self, input: &[u8]) -> std::io::Result<()> {
        for point in input.chunks_exact(self.vlr.items_size() as usize) {
            self.compress_one(point)?;
        }
        Ok(())
    }

    /// Must be called when you have compressed all your points
    /// using the [`compress_one`] method
    ///
    /// [`compress_one`]: #method.compress_one
    pub fn done(&mut self) -> std::io::Result<()> {
        self.record_compressor.done()?;
        self.update_chunk_table()?;
        let stream = self.record_compressor.get_mut();
        details::update_chunk_table_offset(stream, SeekFrom::Start(self.start_pos))?;
        details::write_chunk_table(stream, &self.chunk_sizes)?;
        Ok(())
    }

    /// Returns the vlr used by this compressor
    pub fn vlr(&self) -> &LazVlr {
        &self.vlr
    }

    pub fn into_inner(self) -> W {
        self.record_compressor.box_into_inner()
    }

    pub fn get_mut(&mut self) -> &mut W {
        self.record_compressor.get_mut()
    }

    pub fn get(&self) -> &W {
        self.record_compressor.get()
    }

    fn update_chunk_table(&mut self) -> std::io::Result<()> {
        let current_pos = self
            .record_compressor
            .get_mut()
            .seek(SeekFrom::Current(0))?;
        self.chunk_sizes
            .push((current_pos - self.chunk_start_pos) as usize);
        self.chunk_start_pos = current_pos;
        Ok(())
    }
}

impl<'a, W: Write + Seek + Send + 'a> super::LazCompressor for LasZipCompressor<'a, W> {
    fn compress_many(&mut self, points: &[u8]) -> crate::Result<()> {
        self.compress_many(points)?;
        Ok(())
    }

    fn done(&mut self) -> crate::Result<()> {
        self.done()?;
        Ok(())
    }
}

/// Compresses all points
///
/// The data written will be a standard LAZ file data
/// that means its organized like this:
///  1) offset to the chunk_table (i64)
///  2) the points data compressed
///  3) the chunk table
///
/// `dst`: Where the compressed data will be written
///
/// `uncompressed_points`: byte slice of the uncompressed points to be compressed
pub fn compress_buffer<W: Write + Seek + Send>(
    dst: &mut W,
    uncompressed_points: &[u8],
    laz_vlr: LazVlr,
) -> crate::Result<()> {
    let mut compressor = LasZipCompressor::new(dst, laz_vlr)?;
    let point_size = compressor.vlr().items_size() as usize;
    if uncompressed_points.len() % point_size != 0 {
        Err(LasZipError::BufferLenNotMultipleOfPointSize {
            buffer_len: uncompressed_points.len(),
            point_size,
        })
    } else {
        compressor.compress_many(uncompressed_points)?;
        compressor.done()?;
        Ok(())
    }
}
