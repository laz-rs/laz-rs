use crate::laszip::details::record_decompressor_from_laz_items;
use crate::laszip::{ChunkTable, CompressorType};
use crate::{LasZipCompressor, LasZipError, LazCompressor, LazVlr};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

pub(crate) fn prepare_compressor_for_appending<W, F, F2, Compressor>(
    mut data: W,
    vlr: LazVlr,
    compressor_creator: F,
    get_mut_dest_of_compressor: F2,
) -> crate::Result<(Compressor, ChunkTable)>
where
    W: Write + Read + Seek,
    F: FnOnce(W, LazVlr) -> crate::Result<Compressor>,
    F2: FnOnce(&mut Compressor) -> &mut W,
    Compressor: LazCompressor,
{
    // Technically we could support PointWise compressor
    // But it's old and rare so not much point to do so
    if vlr.compressor != CompressorType::PointWiseChunked
        && vlr.compressor != CompressorType::LayeredChunked
    {
        return Err(LasZipError::UnsupportedCompressorType(vlr.compressor));
    }

    let start_of_data = data.seek(SeekFrom::Current(0))?;
    let mut chunk_table = ChunkTable::read_from(&mut data, &vlr)?;

    let compressor = if !vlr.uses_variable_size_chunks() && !chunk_table.is_empty() {
        // In PointWiseChunked, we don't know if the last chunk is complete or not
        // so we read it, rewrite it so the compressor is in the right state to append points
        let size_of_all_other_chunks = chunk_table.chunk_position(chunk_table.len() - 1).unwrap(); // We know the chunk table is not empty
        let size_of_last_chunk = chunk_table[chunk_table.len() - 1].byte_count;
        let mut last_chunk_data = vec![0u8; size_of_last_chunk as usize];

        let last_chunk_pos = size_of_all_other_chunks.try_into().unwrap();
        data.seek(SeekFrom::Current(last_chunk_pos))?;
        data.read_exact(&mut last_chunk_data)?;

        let mut last_chunk_data = Cursor::new(last_chunk_data);
        let mut decompressor =
            record_decompressor_from_laz_items(vlr.items(), &mut last_chunk_data)?;

        let mut last_chunk_decompressed_data =
            vec![0u8; (chunk_table[chunk_table.len() - 1].point_count * vlr.items_size()) as usize];

        // We cannot trust the point count of the chunk entry
        let i = decompressor.decompress_until_end_of_file(&mut last_chunk_decompressed_data)?;
        let to_be_recompressed = &last_chunk_decompressed_data[..i];

        // seek to beginning of data, so that the compressor can be properly initialized
        data.seek(SeekFrom::Start(start_of_data))?;
        let mut compressor = compressor_creator(data, vlr)?;
        // Explicitly reserve the offset so that the compressor knows where the
        // offset is.
        compressor.reserve_offset_to_chunk_table()?;

        // rewrite the last chunk
        get_mut_dest_of_compressor(&mut compressor).seek(SeekFrom::Current(last_chunk_pos))?;
        compressor.compress_many(to_be_recompressed)?;
        let _ = chunk_table.pop();
        compressor
    } else {
        // Variable size chunks -> we don't need to re-read the last chunk since each chunk can have
        // its own size
        //
        // This branch also handles empty chunk table
        if let Some(end_of_last_chunk) = chunk_table.chunk_position(chunk_table.len()) {
            data.seek(SeekFrom::Start(end_of_last_chunk))?;
        }
        compressor_creator(data, vlr)?
    };

    Ok((compressor, chunk_table))
}

/// Struct that handles appending compressed points to a LAZ file.
pub struct LasZipAppender<'a, W: Write + Send + 'a> {
    saved_chunk_table: ChunkTable,
    compressor: LasZipCompressor<'a, W>,
}

impl<'a, W> LasZipAppender<'a, W>
where
    W: Read + Write + Seek + Send + 'a,
{
    /// data must be positioned at the start of point data
    pub fn new(data: W, vlr: LazVlr) -> crate::Result<Self> {
        let (compressor, chunk_table) = prepare_compressor_for_appending(
            data,
            vlr,
            LasZipCompressor::new,
            LasZipCompressor::get_mut,
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

impl<'a, W> LasZipAppender<'a, W>
where
    W: Write + Seek + Send + 'a,
{
    /// Compress the point and write the compressed data to the destination given when
    /// the compressor was constructed
    ///
    /// The data is written in the buffer is expected to be exactly
    /// as it would have been in a LAS File, that is:
    ///
    /// - The fields/dimensions are in the same order as the LAS spec says
    /// - The data in the buffer is in Little Endian order
    pub fn compress_one(&mut self, input: &[u8]) -> std::io::Result<()> {
        self.compressor.compress_one(input)
    }

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

    /// Compresses multiple chunks
    ///
    /// # Important
    ///
    /// This **must** be called **only** when writing **variable-size** chunks.
    /// This will **panic** otherwise.
    pub fn compress_chunks<Chunks, Item>(&mut self, chunks: Chunks) -> std::io::Result<()>
    where
        Item: AsRef<[u8]> + Send,
        Chunks: IntoIterator<Item = Item>,
    {
        self.compressor.compress_chunks(chunks)
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
        self.compressor.finish_current_chunk()
    }
}
