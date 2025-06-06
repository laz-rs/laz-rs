use crate::laszip::details::record_decompressor_from_laz_items;
use crate::laszip::{ChunkTable, CompressorType};
use crate::{LasZipCompressor, LasZipError, LazCompressor, LazVlr};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

/// `point_count` should be the current number of point in the file
/// we are trying to prepare for appending.
///
/// It is needed as trusting the last chunk to contain exactly the required bytes
/// for the point to be not always correct, (sometimes, there are slightly more bytes)
/// leading to this function decompressing one too many point (which is garbage data)
///
/// * If `point_count` is less than the actual number of points,
///   the rest of the points are going to be overwritten when appending
/// * If `point_count` is greater than the actual number of points,
///   the function will stop at when we know for sure no other points
///   could be present
pub(crate) fn prepare_compressor_for_appending<W, F, F2, Compressor>(
    mut data: W,
    vlr: LazVlr,
    point_count: u64,
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

    let start_of_data = data.stream_position()?;
    let mut chunk_table = ChunkTable::read_from(&mut data, &vlr)?;

    let mut data_to_recompress = vec![];
    if !chunk_table.is_empty() {
        let (chunk_index, chunk_start_pos) = chunk_table
            .chunk_of_point(point_count - 1)
            .unwrap_or_else(|| {
                (
                    chunk_table.len() - 1,
                    chunk_table.chunk_position(chunk_table.len() - 1).unwrap(),
                )
            });

        data.seek(SeekFrom::Current(chunk_start_pos.try_into().unwrap()))?;

        let points_before_chunk = chunk_table[..chunk_index]
            .iter()
            .map(|entry| entry.point_count)
            .sum::<u64>();
        // This cannot overflow as with the chunk_index will be the last known chunk
        // if point_count was out of count
        let mut points_to_read = point_count - points_before_chunk;

        if !vlr.uses_variable_size_chunks()
            && chunk_index == chunk_table.len() - 1
            && points_to_read == 0
        {
            // If we are here, that means we should skip the chunk
            // but, we chose not to completely trust the point count we received
            points_to_read = u64::from(vlr.chunk_size());
        }

        if points_to_read > 0 {
            let size_of_chunk = chunk_table[chunk_index].byte_count;
            // In PointWiseChunked, we don't know if the last chunk is complete or not
            // so we read it, rewrite it so the compressor is in the right state to append points
            let mut chunk_data = vec![0u8; size_of_chunk as usize];

            data.read_exact(&mut chunk_data)?;

            let mut chunk_data = Cursor::new(chunk_data);
            let mut decompressor =
                record_decompressor_from_laz_items(vlr.items(), &mut chunk_data)?;

            // Resize the output buffer so that we decompresse data until its either full,
            // or EOF was reached
            data_to_recompress.resize((points_to_read * vlr.items_size()) as usize, 0);
            let byte_len = decompressor.decompress_until_end_of_file(&mut data_to_recompress)?;
            data_to_recompress.resize(byte_len, 0);
        }

        // Drop chunk table entry/entries that are after the our point
        chunk_table.truncate(chunk_index);
    }

    // Seek to beginning of data, so that the compressor can be properly initialized
    data.seek(SeekFrom::Start(start_of_data))?;
    let mut compressor = compressor_creator(data, vlr)?;
    // Explicitly reserve the offset so that the compressor knows where the
    // offset is.
    compressor.reserve_offset_to_chunk_table()?;
    let last_chunk_pos = chunk_table.chunk_position(chunk_table.len()).unwrap();
    get_mut_dest_of_compressor(&mut compressor).seek(SeekFrom::Current(last_chunk_pos as i64))?;
    // Rewrite the last chunk we read
    if !data_to_recompress.is_empty() {
        compressor.compress_many(&data_to_recompress)?;
    }

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
    pub fn new(data: W, vlr: LazVlr, point_count: u64) -> crate::Result<Self> {
        let (compressor, chunk_table) = prepare_compressor_for_appending(
            data,
            vlr,
            point_count,
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

    pub fn get(&self) -> &W {
        self.compressor.get()
    }

    pub fn into_inner(self) -> W {
        self.compressor.into_inner()
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
