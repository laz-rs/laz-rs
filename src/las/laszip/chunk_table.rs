//! Module with all the things related to LAZ chunk tables
use crate::compressors::IntegerCompressorBuilder;
use crate::decoders::ArithmeticDecoder;
use crate::decompressors::IntegerDecompressorBuilder;
use crate::encoders::ArithmeticEncoder;
use crate::{LasZipError, LazVlr};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::Index;
use std::slice::SliceIndex;

/// An entry describe one chunk and contains 2 information:
///
/// - The number of bytes in the compressed chunk
/// - The number of points in the compressed
#[derive(Copy, Clone, Debug)]
pub(super) struct ChunkTableEntry {
    pub(super) point_count: u64,
    pub(super) byte_count: u64,
}

/// The ChunkTable contains chunk entries for a LAZ file.
///
/// The ChunkTable has two ways of being stored in a LAZ file
/// depending on if the chunks are fixed-size variable-sized
///
/// fixed-size chunks -> Only the number of bytes of the chunk is stored
/// variable-size chunks -> Both the number of points and the number of bytes are stored
pub(super) struct ChunkTable(pub(super) Vec<ChunkTableEntry>);

impl ChunkTable {
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
        if vlr.uses_variably_sized_chunks() {
            ChunkTable::read_as_variably_sized(&mut src)
        } else {
            ChunkTable::read_as_fixed_size(&mut src, vlr.chunk_size().into())
        }
    }

    /// Reads the chunk table that contains both the `point_count` and `bytes_size`.
    ///
    /// This of course will only give correct results if the chunk table stored in the source
    /// contains both these information. Which is the case for **variable-sized** chunks.
    pub(super) fn read_as_variably_sized<R: Read + Seek>(mut src: R) -> crate::Result<Self> {
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
    pub(super) fn read_as_fixed_size<R: Read + Seek>(
        mut src: R,
        point_count: u64,
    ) -> crate::Result<Self> {
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
    fn read_offset<R: Read + Seek>(src: &mut R) -> std::io::Result<Option<(u64, u64)>> {
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
    fn read<R: Read + Seek>(mut src: &mut R, contains_point_count: bool) -> std::io::Result<Self> {
        let _version = src.read_u32::<LittleEndian>()?;
        let number_of_chunks = src.read_u32::<LittleEndian>()?;

        let mut decompressor = IntegerDecompressorBuilder::new()
            .bits(32)
            .contexts(2)
            .build_initialized();
        let mut decoder = ArithmeticDecoder::new(&mut src);
        decoder.read_init_bytes()?;

        let mut chunk_table = ChunkTable::with_capacity(number_of_chunks as usize);
        let mut last_entry = ChunkTableEntry {
            point_count: 0,
            byte_count: 0,
        };
        for _ in 1..=number_of_chunks {
            let mut current_entry = ChunkTableEntry {
                point_count: 0,
                byte_count: 0,
            };
            if contains_point_count {
                current_entry.point_count = u64::from_le(decompressor.decompress(
                    &mut decoder,
                    last_entry.point_count as i32,
                    0,
                )? as u64);
            }
            current_entry.byte_count = u64::from_le(decompressor.decompress(
                &mut decoder,
                last_entry.byte_count as i32,
                1,
            )? as u64);

            chunk_table.0.push(current_entry);
            last_entry = current_entry;
        }
        Ok(chunk_table)
    }
}

impl ChunkTable {
    fn with_capacity(capacity: usize) -> Self {
        let vec = Vec::<ChunkTableEntry>::with_capacity(capacity);
        Self { 0: vec }
    }

    pub(super) fn len(&self) -> usize {
        return self.0.len();
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

/// Write the chunk table
///
/// This function encodes and write the chunk table in the stream
pub fn write_chunk_table<W: Write>(
    mut stream: &mut W,
    chunk_table: &Vec<usize>,
) -> std::io::Result<()> {
    // Write header
    stream.write_u32::<LittleEndian>(0)?;
    stream.write_u32::<LittleEndian>(chunk_table.len() as u32)?;

    let mut encoder = ArithmeticEncoder::new(&mut stream);
    let mut compressor = IntegerCompressorBuilder::new()
        .bits(32)
        .contexts(2)
        .build_initialized();

    let mut predictor = 0;
    for chunk_size in chunk_table {
        compressor.compress(&mut encoder, predictor, (*chunk_size) as i32, 1)?;
        predictor = (*chunk_size) as i32;
    }
    encoder.done()?;
    Ok(())
}
