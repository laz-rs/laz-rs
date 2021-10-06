use std::io::{Seek, SeekFrom, Write};

use byteorder::{LittleEndian, WriteBytesExt};

use crate::laszip::chunk_table::{ChunkTable, ChunkTableEntry};
use crate::record::RecordCompressor;

use super::{chunk_table, details, LazItem, LazVlr};

/// Struct that handles the compression of the points into the given destination
///
/// This supports both **variable-size** and **fixed-size** chunks.
/// Its the [`LazVlr`] that controls which type of chunks you want to write.
///
///
/// # Fixed-Size
///
/// - Use [`compress_one`] and/or [`compress_many`].
/// - The compressor will take care of managing the chunking.
/// - Use [`done`] when you have compressed all the points you wanted.
///
/// # Variable-Size
///
/// - Use [`compress_one`] and/or [`compress_many`] to compress points.
/// - Use [`finish_current_chunk`] achieve variable-size chunks.
/// - Use [`done`] when you have compressed all the points you wanted.
///
/// Or
///
/// - Use [`compress_chunks`] to compress chunks.
/// - Use [`done`] when you have compressed all the points you wanted.
///
/// [`compress_one`]: Self::compress_one
/// [`compress_many`]: Self::compress_many
/// [`compress_chunks`]: Self::compress_chunks
/// [`finish_current_chunk`]: Self::finish_current_chunk
/// [`done`]: Self::done
pub struct LasZipCompressor<'a, W: Write + Send + 'a> {
    vlr: LazVlr,
    /// Compressor used for the current chunk
    record_compressor: Box<dyn RecordCompressor<W> + Send + 'a>,
    /// Position where LasZipCompressor started
    start_pos: u64,
    /// Table of chunks written so far
    chunk_table: ChunkTable,
    /// Entry for the chunk we are currently compressing
    current_chunk_entry: ChunkTableEntry,
    /// Position (offset from beginning)
    /// where the current chunk started
    chunk_start_pos: u64,
}

impl<'a, W: Write + Seek + Send + 'a> LasZipCompressor<'a, W> {
    /// Creates a compressor using the provided vlr.
    pub fn new(output: W, vlr: LazVlr) -> crate::Result<Self> {
        let record_compressor = details::record_compressor_from_laz_items(&vlr.items(), output)?;
        Ok(Self {
            vlr,
            record_compressor,
            chunk_start_pos: 0,
            start_pos: 0,
            chunk_table: ChunkTable::default(),
            current_chunk_entry: ChunkTableEntry::default(),
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

        // Since in variable-size chunks mode the vlr.chunk_size() is
        // u32::max this should not interfere.
        if self.current_chunk_entry.point_count == self.vlr.chunk_size() as u64 {
            self.finish_current_chunk_impl()?;
        }

        self.record_compressor.compress_next(&input)?;
        self.current_chunk_entry.point_count += 1;
        Ok(())
    }

    /// Compress all the points contained in the `input` slice
    pub fn compress_many(&mut self, input: &[u8]) -> std::io::Result<()> {
        for point in input.chunks_exact(self.vlr.items_size() as usize) {
            self.compress_one(point)?;
        }
        Ok(())
    }

    /// Compresses multiple chunks
    ///
    /// # Important
    ///
    /// This **must** be called **only** when writing **variable-size** chunks.
    pub fn compress_chunks<Chunks, Item>(&mut self, chunks: Chunks) -> std::io::Result<()>
    where
        Item: AsRef<[u8]>,
        Chunks: IntoIterator<Item = Item>,
    {
        debug_assert!(self.vlr.uses_variable_size_chunks());
        for chunks in chunks.into_iter() {
            let chunk_points = chunks.as_ref();
            self.compress_many(chunk_points)?;
            self.finish_current_chunk_impl()?;
        }
        Ok(())
    }

    /// Must be called when you have compressed all your points.
    pub fn done(&mut self) -> std::io::Result<()> {
        self.record_compressor.done()?;
        self.update_chunk_table()?;
        let stream = self.record_compressor.get_mut();
        chunk_table::update_chunk_table_offset(stream, SeekFrom::Start(self.start_pos))?;
        self.chunk_table.write_to(stream, &self.vlr)?;
        Ok(())
    }

    /// Finished the current chunks.
    ///
    /// All points compressed with the previous calls to [`compress_one`] and [`compress_many`]
    /// will form one chunk. And the subsequent calls to [`compress_one`] and [`compress_many`]
    /// will form a new chunk.
    ///
    /// # Important
    ///
    /// Only call this when writing **variable-size** chunks.
    ///
    ///
    /// [`compress_one`]: Self::compress_one
    /// [`compress_many`]: Self::compress_many
    pub fn finish_current_chunk(&mut self) -> std::io::Result<()> {
        debug_assert!(
            self.vlr.uses_variable_size_chunks(),
            "finish_current_chunk called on a file which is not in variable-size chunks mode"
        );
        self.finish_current_chunk_impl()
    }

    /// Reserves and prepares the offset to chunk table that will be
    /// updated when [done] is called.
    ///
    /// This method will automatically be called on the first point being compressed,
    /// but for some scenarios, manually calling this might be useful.
    ///
    /// [done]: Self::done
    pub fn reserve_offset_to_chunk_table(&mut self) -> std::io::Result<()> {
        debug_assert_eq!(self.chunk_start_pos, 0);
        let stream = self.record_compressor.get_mut();
        self.start_pos = stream.seek(SeekFrom::Current(0))?;
        stream.write_i64::<LittleEndian>(-1)?;
        self.chunk_start_pos = self.start_pos + std::mem::size_of::<i64>() as u64;
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

    #[inline]
    fn update_chunk_table(&mut self) -> std::io::Result<()> {
        let current_pos = self
            .record_compressor
            .get_mut()
            .seek(SeekFrom::Current(0))?;
        self.current_chunk_entry.byte_count = current_pos - self.chunk_start_pos;
        self.chunk_start_pos = current_pos;
        self.chunk_table.push(self.current_chunk_entry);
        Ok(())
    }

    #[inline]
    fn finish_current_chunk_impl(&mut self) -> std::io::Result<()> {
        self.record_compressor.done()?;
        self.record_compressor.reset();
        self.record_compressor
            .set_fields_from(&self.vlr.items())
            .unwrap();
        self.update_chunk_table()?;
        self.current_chunk_entry = ChunkTableEntry::default();
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
    debug_assert_eq!(uncompressed_points.len() % laz_vlr.items_size() as usize, 0);
    let mut compressor = LasZipCompressor::new(dst, laz_vlr)?;
    compressor.compress_many(uncompressed_points)?;
    compressor.done()?;
    Ok(())
}
