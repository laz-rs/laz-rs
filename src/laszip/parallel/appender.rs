use crate::laszip::sequential::appender::prepare_compressor_for_appending;
use crate::laszip::ChunkTable;
use crate::{LazVlr, ParLasZipCompressor};
use rayon::iter::IntoParallelIterator;
use std::io::{Read, Seek, SeekFrom, Write};

/// Struct that handles appending compressed points to a LAZ file in parallel
pub struct ParLasZipAppender<W> {
    saved_chunk_table: ChunkTable,
    compressor: ParLasZipCompressor<W>,
}

impl<W> ParLasZipAppender<W>
where
    W: Read + Write + Seek + Send,
{
    /// data must be positioned at the start of point data
    pub fn new(data: W, vlr: LazVlr) -> crate::Result<Self> {
        let (compressor, chunk_table) = prepare_compressor_for_appending(
            data,
            vlr,
            ParLasZipCompressor::new,
            ParLasZipCompressor::get_mut,
        )?;

        Ok(Self {
            saved_chunk_table: chunk_table,
            compressor,
        })
    }

    /// Tells the compressor that no more points will be compressed
    ///
    /// - Compresses & writes the rest of the points to form the last chunk
    /// - Writes the chunk table
    /// - update the offset to the chunk_table
    pub fn done(&mut self) -> crate::Result<()> {
        self.compressor.done()?;

        // The compressor wrote a chunk table that only corresponds to added chunks
        // We have to write the chunk table that also have the original chunks

        // 1. Get position of chunk table
        let pos = self.compressor.chunk_table_position_offset() as u64;
        self.compressor.get_mut().seek(SeekFrom::Start(pos))?;
        let (_, chunk_table_pos) = ChunkTable::read_offset(self.compressor.get_mut())?
            .expect("Somehow, the chunk table was not written");

        self.saved_chunk_table.extend(self.compressor.chunk_table());

        // 2 .Overwrite with the correct chunk table
        let write_point_count = self.compressor.vlr().uses_variable_size_chunks();
        let dest = self.compressor.get_mut();
        dest.seek(SeekFrom::Start(chunk_table_pos))?;
        self.saved_chunk_table.write(dest, write_point_count)?;

        Ok(())
    }

    pub fn get_mut(&mut self) -> &mut W {
        self.compressor.get_mut()
    }
}

impl<W> ParLasZipAppender<W>
where
    W: Write + Seek + Send,
{
    /// Compresses many points using multiple threads.
    ///
    /// # Important
    ///
    /// This **must** be called **only** when writing **fixed-size** chunks.
    /// This will **panic** otherwise.
    ///
    /// # Note
    ///
    /// For this function to actually use multiple threads, the `points`
    /// buffer shall hold more points that the vlr's `chunk_size`.
    pub fn compress_many(&mut self, points: &[u8]) -> std::io::Result<()> {
        self.compressor.compress_many(points)
    }

    /// Compresses multiple chunks using multiple threads.
    ///
    /// # Important
    ///
    /// This **must** be called **only** when writing **variable-size** chunks.
    /// This will **panic** otherwise.
    ///
    /// # Note
    ///
    /// For this function to actually use multiple threads, there should be more than one chunk.
    /// buffer shall hold more points that the vlr's `chunk_size`.
    pub fn compress_chunks<Chunks, Item>(&mut self, chunks: Chunks) -> std::io::Result<()>
    where
        Item: AsRef<[u8]> + Send,
        Chunks: IntoParallelIterator<Item = Item>,
    {
        self.compressor.compress_chunks(chunks)
    }
}
