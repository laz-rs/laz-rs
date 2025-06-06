//! Module with all the things related to LAZ chunk tables
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::Index;
use std::slice::SliceIndex;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::compressors::IntegerCompressorBuilder;
use crate::decoders::ArithmeticDecoder;
use crate::decompressors::IntegerDecompressorBuilder;
use crate::encoders::ArithmeticEncoder;
use crate::{LasZipError, LazVlr};

/// Indices of the contexts used for the IntegerCompressor/IntergerDecompressor
/// when decompressing/compressing parts of the chunk table
const POINT_COUNT_CONTEXT: u32 = 0;
const BYTE_COUNT_CONTEXT: u32 = 1;

/// An entry describe one chunk and contains 2 information:
///
/// - The number of bytes in the compressed chunk
/// - The number of points in the compressed
#[derive(Copy, Clone, Debug, Default)]
pub struct ChunkTableEntry {
    pub point_count: u64,
    pub byte_count: u64,
}

/// The ChunkTable contains chunk entries for a LAZ file.
///
/// The ChunkTable has two ways of being stored in a LAZ file
/// depending on if the chunks are fixed-size variable-sized
///
/// - fixed-size chunks -> Only the number of bytes of the chunk is stored
/// - variable-size chunks -> Both the number of points and the number of bytes are stored
#[derive(Default, Debug, Clone)]
pub struct ChunkTable(Vec<ChunkTableEntry>);

impl ChunkTable {
    /// Size in bytes of the offset to the chunk table.
    ///
    /// These bytes are the very first ones, located just after
    /// the `offset_to_point_data`
    pub const OFFSET_SIZE: usize = 8;

    /// Reads the chunk table from the source
    ///
    /// The source position **must** be at the start of the point data
    ///
    /// This functions set position of the `src` where the points actually starts
    /// (that is, after the chunk table offset).
    ///
    /// # Important
    ///
    /// When the chunks are `fixed-size`, each entry will have the same number of points,
    /// the `chunk_size` registered in the `vlr`.
    /// This means that for the **last** chunk, the [ChunkTableEntry]'s `point_count` won't be correct.
    ///
    /// For `variable-size` chunks the `point_count` of each entry is the one read
    /// from the source.
    pub fn read_from<R: Read + Seek>(mut src: R, vlr: &LazVlr) -> crate::Result<Self> {
        if vlr.uses_variable_size_chunks() {
            ChunkTable::read_as_variably_sized(&mut src)
        } else {
            ChunkTable::read_as_fixed_size(&mut src, vlr.chunk_size().into())
        }
    }

    /// Writes the chunk table to the `dst`.
    pub fn write_to<W: Write>(&self, mut dst: W, vlr: &LazVlr) -> std::io::Result<()> {
        self.write(&mut dst, vlr.uses_variable_size_chunks())
    }

    /// Reads the chunk table that contains both the `point_count` and `bytes_size`.
    ///
    /// This of course will only give correct results if the chunk table stored in the source
    /// contains both these information. Which is the case for **variable-sized** chunks.
    fn read_as_variably_sized<R: Read + Seek>(mut src: R) -> crate::Result<Self> {
        let (data_start, chunk_table_start) =
            Self::read_offset(&mut src)?.ok_or(LasZipError::MissingChunkTable)?;
        src.seek(SeekFrom::Start(chunk_table_start))?;
        let chunk_table = Self::read(&mut src, true)?;
        src.seek(SeekFrom::Start(data_start + 8))?;
        Ok(chunk_table)
    }

    /// Reads the chunk table that contains only `point_count`.
    ///
    /// This is for the case when chunks are  **fixed-size**.
    /// Each chunk entry will have the given `point_count` as the point_count.
    fn read_as_fixed_size<R: Read + Seek>(mut src: R, point_count: u64) -> crate::Result<Self> {
        let (data_start, chunk_table_start) =
            Self::read_offset(&mut src)?.ok_or(LasZipError::MissingChunkTable)?;
        src.seek(SeekFrom::Start(chunk_table_start))?;
        let mut chunk_table = Self::read(&mut src, false)?;
        src.seek(SeekFrom::Start(data_start + 8))?;

        for entry in &mut chunk_table.0 {
            entry.point_count = point_count;
        }
        Ok(chunk_table)
    }

    /// Reads the offset to the chunk table.
    ///
    /// `src` should be at the start of LAZ data.
    ///
    /// This function will leave the `src` wherever it read the correct offset.
    ///
    /// Returns the position where the stream was before reading the offset
    /// and the offset value itself
    pub(crate) fn read_offset<R: Read + Seek>(src: &mut R) -> std::io::Result<Option<(u64, u64)>> {
        let current_pos = src.seek(SeekFrom::Current(0))?;

        let mut offset_to_chunk_table = src.read_i64::<LittleEndian>()?;
        if offset_to_chunk_table <= current_pos as i64 {
            // The writer could not update the offset
            // so we have to find it at the end of the data
            src.seek(SeekFrom::End(-8))?;
            offset_to_chunk_table = src.read_i64::<LittleEndian>()?;

            if offset_to_chunk_table <= current_pos as i64 {
                return Ok(None);
            }
        }

        Ok(Some((current_pos, offset_to_chunk_table as u64)))
    }

    /// Actual implementation of the reading of the chunk table.
    ///
    /// The `src` position **must** be at the start of the chunk table
    ///
    /// This function *does not* put the src position at the actual start of points data.
    /// It leaves the position at the end of the chunk table.
    pub fn read<R: Read + Seek>(
        mut src: &mut R,
        contains_point_count: bool,
    ) -> std::io::Result<Self> {
        let _version = src.read_u32::<LittleEndian>()?;
        let number_of_chunks = src.read_u32::<LittleEndian>()?;

        if number_of_chunks == 0 {
            return Ok(Self::default());
        }

        let mut decompressor = IntegerDecompressorBuilder::new()
            .bits(32)
            .contexts(2)
            .build_initialized();
        let mut decoder = ArithmeticDecoder::new(&mut src);
        decoder.read_init_bytes()?;

        let mut chunk_table = ChunkTable::with_capacity(number_of_chunks as usize);
        let mut previous_entry = ChunkTableEntry::default();
        for _ in 1..=number_of_chunks {
            let mut current_entry = ChunkTableEntry {
                point_count: 0,
                byte_count: 0,
            };
            if contains_point_count {
                current_entry.point_count = u64::from_le(decompressor.decompress(
                    &mut decoder,
                    previous_entry.point_count as i32,
                    POINT_COUNT_CONTEXT,
                )? as u64);
            }
            current_entry.byte_count = u64::from_le(decompressor.decompress(
                &mut decoder,
                previous_entry.byte_count as i32,
                BYTE_COUNT_CONTEXT,
            )? as u64);

            chunk_table.0.push(current_entry);
            previous_entry = current_entry;
        }
        Ok(chunk_table)
    }

    /// Finds the chunk that the given point index belongs to.
    ///
    /// Returns `None` if the `point_index` does not belong to the chunk table.
    /// Returns a tuple (chunk_index, byte_offset)
    /// where byte_offset is the byte offset starting from the first chunk.
    ///
    /// # Warning
    ///
    /// In the case of Non-variable chunk size, when the point falls into the last
    /// chunk it does not mean the point actually exists.
    ///
    /// Eg with chunk_size 50_000 and a file with 75_000 points
    /// point index 75_001 falls into the last chunk, but since we only know
    /// that the chunk_size is 50_000, we cannot be sure at that point it exists.
    ///
    /// It is the responsibility of the caller to verify / handle that case.
    pub(crate) fn chunk_of_point(&self, point_idx: u64) -> Option<(usize, u64)> {
        let mut chunk_of_point = 0usize;
        let mut start_of_chunk = 0;
        let mut tmp_count = 0;
        for entry in &self.0 {
            tmp_count += entry.point_count;
            if tmp_count > point_idx {
                break;
            }
            start_of_chunk += entry.byte_count;
            chunk_of_point += 1;
        }

        if point_idx >= tmp_count {
            None
        } else {
            Some((chunk_of_point, start_of_chunk))
        }
    }

    /// Returns the position one must seek to, to reach the beginning of
    /// the given chunk index.
    ///
    /// This position is relative to the where the points actually start
    /// (i.e. after the chunk_table offset)
    ///
    /// Returns None if the chunk does not exist.
    pub(crate) fn chunk_position(&self, chunk_index: usize) -> Option<u64> {
        if chunk_index > self.len() {
            None
        } else {
            Some(
                self.0[0..chunk_index]
                    .iter()
                    .map(|entry| entry.byte_count)
                    .sum(),
            )
        }
    }

    pub(crate) fn write<W: Write>(
        &self,
        mut dst: &mut W,
        write_point_count: bool,
    ) -> std::io::Result<()> {
        // Write header
        dst.write_u32::<LittleEndian>(0)?;
        dst.write_u32::<LittleEndian>(self.len() as u32)?;

        if self.is_empty() {
            return Ok(());
        }

        let mut encoder = ArithmeticEncoder::new(&mut dst);
        let mut compressor = IntegerCompressorBuilder::new()
            .bits(32)
            .contexts(2)
            .build_initialized();

        let mut previous_entry = ChunkTableEntry::default();
        for current_entry in &self.0 {
            if write_point_count {
                compressor.compress(
                    &mut encoder,
                    previous_entry.point_count as i32,
                    current_entry.point_count as i32,
                    POINT_COUNT_CONTEXT,
                )?;
                previous_entry.point_count = current_entry.point_count;
            }
            compressor.compress(
                &mut encoder,
                previous_entry.byte_count as i32,
                current_entry.byte_count as i32,
                BYTE_COUNT_CONTEXT,
            )?;
            previous_entry.byte_count = current_entry.byte_count;
        }
        encoder.done()?;
        Ok(())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let vec = Vec::<ChunkTableEntry>::with_capacity(capacity);
        Self { 0: vec }
    }

    pub fn push(&mut self, entry: ChunkTableEntry) {
        self.0.push(entry);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn pop(&mut self) -> Option<ChunkTableEntry> {
        self.0.pop()
    }

    pub fn extend(&mut self, other: &ChunkTable) {
        self.0.extend(&other.0)
    }

    pub fn truncate(&mut self, len: usize) {
        self.0.truncate(len);
    }
}

impl AsRef<[ChunkTableEntry]> for ChunkTable {
    fn as_ref(&self) -> &[ChunkTableEntry] {
        &self.0
    }
}

impl<'a> IntoIterator for &'a ChunkTable {
    type Item = <std::slice::Iter<'a, ChunkTableEntry> as Iterator>::Item;
    type IntoIter = std::slice::Iter<'a, ChunkTableEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<I> Index<I> for ChunkTable
where
    I: SliceIndex<[ChunkTableEntry]>,
{
    type Output = <I as SliceIndex<[ChunkTableEntry]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.0[index]
    }
}

/// Updates the 'chunk table offset'
///
/// It is the first 8 byte (i64) of a Laszip compressed data
///
/// This function expects the position of the destination to be at the start of the chunk_table
/// (whether it is written or not).
///
/// This function also expects the i64 to have been already written/reserved
/// (even if its garbage bytes / 0s)
///
/// The position of the destination is untouched
///
/// `offset_pos`: position the function have to seek to, to be where the offset
///               should be updated.
pub(super) fn update_chunk_table_offset<W: Write + Seek>(
    dst: &mut W,
    offset_pos: SeekFrom,
) -> std::io::Result<()> {
    let start_of_chunk_table_pos = dst.seek(SeekFrom::Current(0))?;
    dst.seek(offset_pos)?;
    dst.write_i64::<LittleEndian>(start_of_chunk_table_pos as i64)?;
    dst.seek(SeekFrom::Start(start_of_chunk_table_pos))?;
    Ok(())
}
