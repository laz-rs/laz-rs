use crate::compressors::IntegerCompressorBuilder;
use crate::decoders::ArithmeticDecoder;
use crate::decompressors::IntegerDecompressorBuilder;
use crate::encoders::ArithmeticEncoder;
use crate::record::{
    LayeredPointRecordCompressor, LayeredPointRecordDecompressor, RecordCompressor,
    RecordDecompressor, SequentialPointRecordCompressor, SequentialPointRecordDecompressor,
};
use crate::{LasZipError, LazItem};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Seek, SeekFrom, Write};

/// Reads the chunk table from the source
///
/// The source position is expected to be at the start of the point data
///
/// This functions set position of the `src` where the points actually starts
/// (that is, after the chunk table offset).
pub fn read_chunk_table<R: Read + Seek>(src: &mut R) -> Option<std::io::Result<Vec<u64>>> {
    let current_pos = match src.seek(SeekFrom::Current(0)) {
        Ok(p) => p,
        Err(e) => return Some(Err(e)),
    };

    let offset_to_chunk_table = match src.read_i64::<LittleEndian>() {
        Ok(p) => p,
        Err(e) => return Some(Err(e)),
    };

    if offset_to_chunk_table >= 0 && offset_to_chunk_table as u64 <= current_pos {
        // In that case the compressor was probably stopped
        // before being able to write the chunk table
        None
    } else {
        Some(read_chunk_table_at_offset(src, offset_to_chunk_table))
    }
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
pub(super) fn read_chunk_table_at_offset<R: Read + Seek>(
    mut src: &mut R,
    mut offset_to_chunk_table: i64,
) -> std::io::Result<Vec<u64>> {
    let current_pos = src.seek(SeekFrom::Current(0))?;
    if offset_to_chunk_table == -1 {
        // Compressor was writing to non seekable src
        src.seek(SeekFrom::End(-8))?;
        offset_to_chunk_table = src.read_i64::<LittleEndian>()?;
    }
    src.seek(SeekFrom::Start(offset_to_chunk_table as u64))?;

    let _version = src.read_u32::<LittleEndian>()?;
    let number_of_chunks = src.read_u32::<LittleEndian>()?;
    let mut chunk_sizes = vec![0u64; number_of_chunks as usize];

    let mut decompressor = IntegerDecompressorBuilder::new()
        .bits(32)
        .contexts(2)
        .build_initialized();
    let mut decoder = ArithmeticDecoder::new(&mut src);
    decoder.read_init_bytes()?;
    for i in 1..=number_of_chunks {
        chunk_sizes[(i - 1) as usize] = decompressor.decompress(
            &mut decoder,
            if i > 1 {
                chunk_sizes[(i - 2) as usize]
            } else {
                0
            } as i32,
            1,
        )? as u64;
    }
    src.seek(SeekFrom::Start(current_pos))?;
    Ok(chunk_sizes)
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

pub(super) fn record_decompressor_from_laz_items<'a, R: Read + Seek + Send + 'a>(
    items: &Vec<LazItem>,
    input: R,
) -> crate::Result<Box<dyn RecordDecompressor<R> + Send + 'a>> {
    let first_item = items
        .get(0)
        .expect("There should be at least one LazItem to be able to create a RecordDecompressor");

    let mut decompressor = match first_item.version {
        1 | 2 => {
            let decompressor = SequentialPointRecordDecompressor::new(input);
            Box::new(decompressor) as Box<dyn RecordDecompressor<R> + Send>
        }
        3 | 4 => {
            let decompressor = LayeredPointRecordDecompressor::new(input);
            Box::new(decompressor) as Box<dyn RecordDecompressor<R> + Send>
        }
        _ => {
            return Err(LasZipError::UnsupportedLazItemVersion(
                first_item.item_type,
                first_item.version,
            ));
        }
    };

    decompressor.set_fields_from(items)?;
    Ok(decompressor)
}

pub(super) fn record_compressor_from_laz_items<'a, W: Write + Send + 'a>(
    items: &Vec<LazItem>,
    output: W,
) -> crate::Result<Box<dyn RecordCompressor<W> + Send + 'a>> {
    let first_item = items
        .get(0)
        .expect("There should be at least one LazItem to be able to create a RecordCompressor");

    let mut compressor = match first_item.version {
        1 | 2 => {
            let compressor = SequentialPointRecordCompressor::new(output);
            Box::new(compressor) as Box<dyn RecordCompressor<W> + Send>
        }
        3 | 4 => {
            let compressor = LayeredPointRecordCompressor::new(output);
            Box::new(compressor) as Box<dyn RecordCompressor<W> + Send>
        }
        _ => {
            return Err(LasZipError::UnsupportedLazItemVersion(
                first_item.item_type,
                first_item.version,
            ));
        }
    };
    compressor.set_fields_from(items)?;
    Ok(compressor)
}
