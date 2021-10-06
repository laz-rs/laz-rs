use std::io::{Read, Seek, SeekFrom};

use rayon::prelude::*;

use crate::byteslice::ChunksIrregularMut;
use crate::laszip::chunk_table::{ChunkTable, ChunkTableEntry};
use crate::laszip::details::record_decompressor_from_laz_items;
use crate::LazVlr;

#[cfg(feature = "parallel")]
/// Laszip decompressor, that can decompress data using multiple threads
///
/// Supports both fixed-size and variable-size chunks.
pub struct ParLasZipDecompressor<R> {
    vlr: LazVlr,
    /// Table of chunks read from the source.
    chunk_table: ChunkTable,
    last_chunk_read: isize,
    /// Position of the first compressed point.
    start_of_data: u64,
    // Same idea as in ParLasZipCompressor.
    rest: std::io::Cursor<Vec<u8>>,
    // `internal_buffer` is used to hold the chunks to be decompressed
    // after copying them from the source.
    // Each thread will receive a chunk of this buffer to decompress it.
    // It makes the decompression io-free, which means no performance penalty.
    // And it is ok to have such an internal buffer as
    // the compressed data is much much smaller that uncompressed data.
    internal_buffer: Vec<u8>,
    source: R,
}

#[cfg(feature = "parallel")]
impl<R: Read + Seek> ParLasZipDecompressor<R> {
    /// Creates a new decompressor
    ///
    /// Fails if no chunk table could be found.
    pub fn new(mut source: R, vlr: LazVlr) -> crate::Result<Self> {
        let chunk_table = ChunkTable::read_from(&mut source, &vlr)?;
        let start_of_data = source.seek(SeekFrom::Current(0))?;
        let biggest_chunk = chunk_table
            .as_ref()
            .into_iter()
            .map(|entry| entry.point_count)
            .max()
            .unwrap();
        let vec = Vec::<u8>::with_capacity(biggest_chunk as usize);
        let rest = std::io::Cursor::new(vec);

        Ok(Self {
            source,
            vlr,
            chunk_table,
            rest,
            internal_buffer: vec![],
            last_chunk_read: -1,
            start_of_data,
        })
    }

    /// Decompresses many points using multiple threads
    ///
    /// For this function to actually use multiple threads, the `points`
    /// buffer shall hold more points that the vlr's `chunk_size`.
    pub fn decompress_many(&mut self, out: &mut [u8]) -> crate::Result<()> {
        let point_size = self.vlr.items_size() as usize;
        assert_eq!(out.len() % point_size, 0);

        let num_bytes_in_rest = self.rest.get_ref().len() - self.rest.position() as usize;
        debug_assert!(num_bytes_in_rest % point_size == 0);

        let (out_rest, out_decompress) = if num_bytes_in_rest >= out.len() {
            (out, &mut [] as &mut [u8])
        } else {
            out.split_at_mut(num_bytes_in_rest)
        };

        // 1. Copy the data from our rest into the caller's buffer
        if !out_rest.is_empty() {
            self.rest.read(out_rest)?;
        }

        if out_decompress.is_empty() {
            return Ok(());
        }

        debug_assert_eq!(
            self.rest.position() as usize,
            self.rest.get_mut().len(),
            "Rest buffer not completely consumed"
        );
        self.rest.get_mut().clear();
        self.rest.set_position(0);

        // 2. Find out how many chunks we have to decompress
        // and load their compressed bytes in our internal buffer
        let num_requested_points_left = out_decompress.len() / point_size;
        let start_index = (self.last_chunk_read + 1) as usize;
        let mut num_points = 0usize;
        let mut num_chunks_to_decompress = 0;
        let mut num_bytes_to_read = 0usize;
        for entry in &self.chunk_table[start_index..] {
            num_points += entry.point_count as usize;
            num_chunks_to_decompress += 1;
            num_bytes_to_read += entry.byte_count as usize;
            if num_points >= num_requested_points_left {
                break;
            }
        }
        let end_index = start_index + num_chunks_to_decompress;

        debug_assert!(num_chunks_to_decompress >= 1);
        debug_assert!(num_points >= num_requested_points_left);
        // TODO if num_points >= num_requested_points_left then the user ask to decompress more points
        //      than there are

        // Read the necessary compressed bytes into our internal buffer
        self.internal_buffer.resize(num_bytes_to_read, 0u8);
        self.source.read(&mut self.internal_buffer)?;

        // 3. Decompress
        // The idea is that if we have `n` chunks to decompress
        // we decompress `n-1` 'normally', and we do something special for the
        // last chunk as we have to handle the `rest`.
        //
        // We start by splitting all the buffers in two, one for the so called `n-1` chunks
        // and one for the `n` chunk.
        let head_chunks_table = &self.chunk_table[start_index..end_index - 1];
        let tail_chunk_entry = self.chunk_table[end_index - 1];
        let num_bytes_in_head_chunks = num_bytes_to_read - tail_chunk_entry.byte_count as usize;
        let num_points_in_head_chunks = num_points - tail_chunk_entry.point_count as usize;
        let (head_chunks, tail_chunk) = self.internal_buffer.split_at(num_bytes_in_head_chunks);
        let (head_output, tail_output) =
            out_decompress.split_at_mut(num_points_in_head_chunks * point_size);

        // These are to make the borrow checker happy self is not `Send`.
        let rest = &mut self.rest;
        let vlr = &self.vlr;
        let chunk_table_len = self.chunk_table.len();
        let (res1, res2) = rayon::join(
            || -> crate::Result<()> {
                par_decompress(head_chunks, head_output, &vlr, head_chunks_table)
            },
            || -> crate::Result<()> {
                let mut last_src = std::io::Cursor::new(tail_chunk);
                let mut decompressor =
                    record_decompressor_from_laz_items(&vlr.items(), &mut last_src)?;
                // Decompress what we can in the caller's buffer
                decompressor.decompress_many(tail_output)?;
                // Then, decompress what we did not, into our rest buffer
                let num_bytes_left =
                    (tail_chunk_entry.point_count as usize * point_size) - tail_output.len();
                if !vlr.uses_variable_size_chunks() && end_index == chunk_table_len {
                    // When fixed-size chunks are used, for the last chunk, the number of point
                    // is unknown, so we have to decompress it until an end of file appears
                    rest.get_mut().resize(num_bytes_left, 0u8);
                    let num_actually_decompressed =
                        decompressor.decompress_until_end_of_file(rest.get_mut())?;
                    rest.get_mut().resize(num_actually_decompressed, 0u8);
                } else {
                    rest.get_mut().resize(num_bytes_left, 0u8);
                    decompressor.decompress_many(rest.get_mut())?;
                }
                rest.set_position(0);
                Ok(())
            },
        );
        res1?;
        res2?;

        self.last_chunk_read += num_chunks_to_decompress as isize;
        Ok(())
    }

    /// Seeks to the position of the point at the given index
    pub fn seek(&mut self, index: u64) -> crate::Result<()> {
        // Throw away what's in the rest buffer
        self.rest.set_position(0);
        self.rest.get_mut().clear();

        let chunk_of_point = (index / self.vlr.chunk_size() as u64) as usize;
        if chunk_of_point >= self.chunk_table.len() {
            let _ = self.source.seek(SeekFrom::End(0))?;
            return Ok(());
        }
        // Seek to the start of the points chunk
        // and read the chunk data
        let start_of_chunk_pos = self.start_of_data
            + self.chunk_table[..chunk_of_point]
                .iter()
                .map(|entry| entry.byte_count)
                .sum::<u64>();
        self.source.seek(SeekFrom::Start(start_of_chunk_pos))?;
        self.internal_buffer
            .resize(self.chunk_table[chunk_of_point].byte_count as usize, 0u8);
        self.source.read(&mut self.internal_buffer)?;

        // Completely decompress the chunk
        self.rest
            .get_mut()
            .resize(self.vlr.num_bytes_in_decompressed_chunk() as usize, 0u8);
        let mut decompressor = record_decompressor_from_laz_items(
            self.vlr.items(),
            std::io::Cursor::new(&self.internal_buffer),
        )?;
        let is_last_chunk = chunk_of_point == (self.chunk_table.len() - 1);
        if is_last_chunk {
            let num_bytes_decompressed =
                decompressor.decompress_until_end_of_file(self.rest.get_mut())?;
            let num_points_in_last_chunk = num_bytes_decompressed / self.vlr.items_size() as usize;
            let pos_in_chunk = index % self.vlr.chunk_size() as u64;
            if pos_in_chunk as usize >= num_points_in_last_chunk as usize {
                // Make the rest appear as fully consumed to
                // force EOF error on next decompression
                self.rest.set_position(self.rest.get_ref().len() as u64);
                return Ok(());
            }
        } else {
            decompressor.decompress_many(self.rest.get_mut())?;
        }
        // This effectively discard points that were
        // before the one we just seeked to
        let pos_in_chunk = index % self.vlr.chunk_size() as u64;
        self.rest.set_position(pos_in_chunk * self.vlr.items_size());
        self.last_chunk_read = chunk_of_point as isize;
        Ok(())
    }

    pub fn into_inner(self) -> R {
        self.source
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.source
    }

    pub fn get(&self) -> &R {
        &self.source
    }
}

impl<R: Read + Seek> crate::laszip::LazDecompressor for ParLasZipDecompressor<R> {
    fn decompress_many(&mut self, points: &mut [u8]) -> crate::Result<()> {
        self.decompress_many(points)
    }

    fn seek(&mut self, index: u64) -> crate::Result<()> {
        self.seek(index)
    }
}

/// Decompresses all points from the buffer in parallel.
///
/// Each chunk is sent for decompression in a thread.
///
/// Just like [`decompress_buffer`] but the decompression is done using multiple threads
///
/// # Important
///
/// All the points in the doc of [`decompress_buffer`] applies to this
/// fn with the addition that  the chunk table _IS_ mandatory
///
/// [`decompress_buffer`]: fn.decompress_buffer.html
#[cfg(feature = "parallel")]
pub fn par_decompress_buffer(
    compressed_points_data: &[u8],
    decompressed_points: &mut [u8],
    laz_vlr: &LazVlr,
) -> crate::Result<()> {
    debug_assert_eq!(decompressed_points.len() % laz_vlr.items_size() as usize, 0);
    let mut cursor = std::io::Cursor::new(compressed_points_data);
    let chunk_table = ChunkTable::read_from(&mut cursor, &laz_vlr)?;

    let num_point_bytes = chunk_table
        .as_ref()
        .iter()
        .map(|entry| entry.byte_count as usize)
        .sum::<usize>();

    let compressed_points = &compressed_points_data[std::mem::size_of::<i64>()..num_point_bytes];
    par_decompress(
        compressed_points,
        decompressed_points,
        laz_vlr,
        chunk_table.as_ref(),
    )
}

/// Actual the parallel decompression
///
/// `compressed_points` must contains only the bytes corresponding to the points
/// (so no offset, no chunk_table)
#[cfg(feature = "parallel")]
fn par_decompress(
    compressed_points: &[u8],
    decompressed_points: &mut [u8],
    laz_vlr: &LazVlr,
    chunk_table: &[ChunkTableEntry],
) -> crate::Result<()> {
    use crate::byteslice::ChunksIrregular;
    let sizes = chunk_table.iter().map(|entry| entry.byte_count as usize);
    let counts = chunk_table
        .iter()
        .map(|entry| (entry.point_count * laz_vlr.items_size()) as usize);
    let input_chunks_iter = ChunksIrregular::new(compressed_points, sizes);
    let output_chunks_iter = ChunksIrregularMut::new(decompressed_points, counts);

    // FIXME we collect into a Vec because zip cannot be made 'into_par_iter' by rayon
    //  (or at least i don't know how)
    let decompression_jobs: Vec<(&[u8], &mut [u8])> =
        input_chunks_iter.zip(output_chunks_iter).collect();
    decompression_jobs
        .into_par_iter()
        .map(|(chunk_in, chunk_out)| {
            let src = std::io::Cursor::new(chunk_in);
            let mut record_decompressor = record_decompressor_from_laz_items(laz_vlr.items(), src)?;
            record_decompressor.decompress_many(chunk_out)?;
            Ok(())
        })
        .collect::<crate::Result<()>>()?;
    Ok(())
}
