//! Module with the important struct that people wishing
//! to compress or decompress LAZ data can use
//!
//! It defines the LaszipCompressor & LaszipDecompressor
//! as well as the Laszip VLr data  and how to build it

use std::io::{Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::compressors::IntegerCompressorBuilder;
use crate::decoders::ArithmeticDecoder;
use crate::decompressors::IntegerDecompressorBuilder;
use crate::encoders::ArithmeticEncoder;
pub use crate::errors::LasZipError;
use crate::las::nir::Nir;
use crate::las::point6::Point6;
use crate::las::rgb::RGB;
use crate::las::Point0;
use crate::record::{
    LayeredPointRecordCompressor, LayeredPointRecordDecompressor, RecordCompressor,
    RecordDecompressor, SequentialPointRecordCompressor, SequentialPointRecordDecompressor,
};


const DEFAULT_CHUNK_SIZE: usize = 50_000;

pub const LASZIP_USER_ID: &'static str = "laszip encoded";
pub const LASZIP_RECORD_ID: u16 = 22204;
pub const LASZIP_DESCRIPTION: &'static str = "http://laszip.org";

#[derive(Debug, Copy, Clone)]
struct Version {
    major: u8,
    minor: u8,
    revision: u16,
}

impl Version {
    fn read_from<R: Read>(src: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            major: src.read_u8()?,
            minor: src.read_u8()?,
            revision: src.read_u16::<LittleEndian>()?,
        })
    }

    fn write_to<W: Write>(&self, dst: &mut W) -> std::io::Result<()> {
        dst.write_u8(self.major)?;
        dst.write_u8(self.minor)?;
        dst.write_u16::<LittleEndian>(self.revision)?;
        Ok(())
    }
}

// The different type of data / fields found in the definition of LAS points
#[derive(Debug, Copy, Clone)]
pub enum LazItemType {
    Byte(u16),
    Point10,
    GpsTime,
    RGB12,
    //WavePacket13,
    Point14,
    RGB14,
    RGBNIR14,
    //WavePacket14,
    Byte14(u16),
}

impl LazItemType {
    fn size(&self) -> u16 {
        match self {
            LazItemType::Byte(size) => *size,
            LazItemType::Point10 => Point0::SIZE as u16,
            LazItemType::GpsTime => std::mem::size_of::<f64>() as u16,
            LazItemType::RGB12 => RGB::SIZE as u16,
            LazItemType::Point14 => Point6::SIZE as u16,
            LazItemType::RGB14 => RGB::SIZE as u16,
            LazItemType::RGBNIR14 => (RGB::SIZE + Nir::SIZE) as u16,
            LazItemType::Byte14(size) => *size,
        }
    }
}

impl From<LazItemType> for u16 {
    fn from(t: LazItemType) -> Self {
        match t {
            LazItemType::Byte(_) => 0,
            LazItemType::Point10 => 6,
            LazItemType::GpsTime => 7,
            LazItemType::RGB12 => 8,
            //LazItemType::WavePacket13 => 9,
            LazItemType::Point14 => 10,
            LazItemType::RGB14 => 11,
            LazItemType::RGBNIR14 => 12,
            //LazItemType::WavePacket14 => 13,
            LazItemType::Byte14(_) => 14,
        }
    }
}

/// Struct stored as part of the laszip's vlr record_data
///
/// This gives information about the dimension encoded and the version used
/// when encoding the data.
#[derive(Debug, Copy, Clone)]
pub struct LazItem {
    // coded on a u16
    pub(crate) item_type: LazItemType,
    pub(crate) size: u16,
    pub(crate) version: u16,
}

impl LazItem {
    pub(crate) fn new(item_type: LazItemType, version: u16) -> Self {
        let size = item_type.size();
        Self {
            item_type,
            size,
            version,
        }
    }

    fn read_from<R: Read>(src: &mut R) -> Result<Self, LasZipError> {
        let item_type = src.read_u16::<LittleEndian>()?;
        let size = src.read_u16::<LittleEndian>()?;
        let item_type = match item_type {
            0 => LazItemType::Byte(size),
            6 => LazItemType::Point10,
            7 => LazItemType::GpsTime,
            8 => LazItemType::RGB12,
            //9 => LazItemType::WavePacket13,
            10 => LazItemType::Point14,
            11 => LazItemType::RGB14,
            12 => LazItemType::RGBNIR14,
            //13 => LazItemType::WavePacket14,
            14 => LazItemType::Byte14(size),
            _ => return Err(LasZipError::UnknownLazItem(item_type)),
        };
        Ok(Self {
            item_type,
            size,
            version: src.read_u16::<LittleEndian>()?,
        })
    }

    fn write_to<W: Write>(&self, dst: &mut W) -> std::io::Result<()> {
        dst.write_u16::<LittleEndian>(self.item_type.into())?;
        dst.write_u16::<LittleEndian>(self.size)?;
        dst.write_u16::<LittleEndian>(self.version)?;
        Ok(())
    }
}

macro_rules! define_trait_for_version {
    ($trait_name:ident, $trait_fn_name:ident) => {
        pub trait $trait_name {
            fn $trait_fn_name(num_extra_bytes: u16) -> Vec<LazItem>;
        }
    };
}

define_trait_for_version!(DefaultVersion, default_version);
define_trait_for_version!(Version1, version_1);
define_trait_for_version!(Version2, version_2);
define_trait_for_version!(Version3, version_3);

pub struct LazItemRecordBuilder {
    items: Vec<LazItemType>,
}

impl LazItemRecordBuilder {
    pub fn default_version_of<PointFormat: DefaultVersion>(num_extra_bytes: u16) -> Vec<LazItem> {
        PointFormat::default_version(num_extra_bytes)
    }

    pub fn version_1_of<PointFormat: Version1>(num_extra_bytes: u16) -> Vec<LazItem> {
        PointFormat::version_1(num_extra_bytes)
    }

    pub fn version_2_of<PointFormat: Version2>(num_extra_bytes: u16) -> Vec<LazItem> {
        PointFormat::version_2(num_extra_bytes)
    }

    pub fn version_3_of<PointFormat: Version3>(num_extra_bytes: u16) -> Vec<LazItem> {
        PointFormat::version_3(num_extra_bytes)
    }

    pub fn default_for_point_format_id(point_format_id: u8, num_extra_bytes: u16) -> Vec<LazItem> {
        use crate::las::{Point1, Point2, Point3, Point7, Point8};
        match point_format_id {
            0 => LazItemRecordBuilder::default_version_of::<Point0>(num_extra_bytes),
            1 => LazItemRecordBuilder::default_version_of::<Point1>(num_extra_bytes),
            2 => LazItemRecordBuilder::default_version_of::<Point2>(num_extra_bytes),
            3 => LazItemRecordBuilder::default_version_of::<Point3>(num_extra_bytes),
            6 => LazItemRecordBuilder::default_version_of::<Point6>(num_extra_bytes),
            7 => LazItemRecordBuilder::default_version_of::<Point7>(num_extra_bytes),
            8 => LazItemRecordBuilder::default_version_of::<Point8>(num_extra_bytes),
            _ => panic!(
                "Point format id: {} is not supported",
                point_format_id
            ),
        }
    }

    pub fn new() -> Self {
        Self { items: vec![] }
    }

    pub fn add_item(&mut self, item_type: LazItemType) -> &mut Self {
        self.items.push(item_type);
        self
    }

    pub fn build(&self) -> Vec<LazItem> {
        self.items
            .iter()
            .map(|item_type| {
                let size= item_type.size();
                let version = match item_type {
                    LazItemType::Byte(_) => 2,
                    LazItemType::Point10 => 2,
                    LazItemType::GpsTime => 2,
                    LazItemType::RGB12 => 2,
                    LazItemType::Point14 => 3,
                    LazItemType::RGB14 => 3,
                    LazItemType::RGBNIR14 => 3,
                    LazItemType::Byte14(_) => 3,
                };
                LazItem {
                    item_type: *item_type,
                    size,
                    version,
                }
            })
            .collect()
    }
}

fn read_laz_items_from<R: Read>(mut src: &mut R) -> Result<Vec<LazItem>, LasZipError> {
    let num_items = src.read_u16::<LittleEndian>()?;
    let mut items = Vec::<LazItem>::with_capacity(num_items as usize);
    for _ in 0..num_items {
        items.push(LazItem::read_from(&mut src)?)
    }
    Ok(items)
}

fn write_laz_items_to<W: Write>(laz_items: &Vec<LazItem>, mut dst: &mut W) -> std::io::Result<()> {
    dst.write_u16::<LittleEndian>(laz_items.len() as u16)?;
    for item in laz_items {
        item.write_to(&mut dst)?;
    }
    Ok(())
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CompressorType {
    // TODO might need a better name
    None = 0,
    /// No chunks, or rather only 1 chunk with all the points
    PointWise = 1,
    /// Compress points into chunks with chunk_size points in each chunks
    PointWiseChunked = 2,
    /// Compress points into chunk, but also separate the different point dimension / fields
    /// into layers. This CompressorType is only use for point 6,7,8,9,10
    LayeredChunked = 3,
}

impl CompressorType {
    fn from_u16(t: u16) -> Option<Self> {
        match t {
            0 => Some(CompressorType::None),
            1 => Some(CompressorType::PointWise),
            2 => Some(CompressorType::PointWiseChunked),
            3 => Some(CompressorType::LayeredChunked),
            _ => None,
        }
    }
}

impl Default for CompressorType {
    fn default() -> Self {
        CompressorType::PointWiseChunked
    }
}

/// The data stored in the record_data of the Laszip Vlr
#[derive(Debug, Clone)]
pub struct LazVlr {
    // coded on u16
    compressor: CompressorType,
    // 0 means ArithmeticCoder, its the only choice
    coder: u16,

    version: Version,
    options: u32,
    /// Number of points per chunk
    chunk_size: u32,

    // -1 if unused
    number_of_special_evlrs: i64,
    // -1 if unused
    offset_to_special_evlrs: i64,

    items: Vec<LazItem>,
}

impl LazVlr {
    // TODO should impl default instead
    pub fn new() -> Self {
        Self {
            compressor: CompressorType::default(),
            coder: 0,
            version: Version {
                major: 2,
                minor: 2,
                revision: 0,
            },
            options: 0,
            chunk_size: DEFAULT_CHUNK_SIZE as u32,
            number_of_special_evlrs: -1,
            offset_to_special_evlrs: -1,
            items: vec![],
        }
    }

    pub fn from_laz_items(items: Vec<LazItem>) -> Self {
        let mut me = Self::new();
        me.items = items;
        me
    }

    /// Tries to read the Vlr information from the record_data buffer
    pub fn from_buffer(record_data: &[u8]) -> Result<Self, LasZipError> {
        let mut cursor = std::io::Cursor::new(record_data);
        Self::read_from(&mut cursor)
    }

    /// Tries to read the Vlr information from the record_data source
    pub fn read_from<R: Read>(mut src: &mut R) -> Result<Self, LasZipError> {
        let compressor_type = src.read_u16::<LittleEndian>()?;
        let compressor = match CompressorType::from_u16(compressor_type) {
            Some(c) => c,
            None => return Err(LasZipError::UnknownCompressorType(compressor_type)),
        };

        Ok(Self {
            compressor,
            coder: src.read_u16::<LittleEndian>()?,
            version: Version::read_from(&mut src)?,
            options: src.read_u32::<LittleEndian>()?,
            chunk_size: src.read_u32::<LittleEndian>()?,
            number_of_special_evlrs: src.read_i64::<LittleEndian>()?,
            offset_to_special_evlrs: src.read_i64::<LittleEndian>()?,
            items: read_laz_items_from(&mut src)?,
        })
    }

    /// Writes the Vlr to the source, this only write the 'record_data' the
    /// header should be written before-hand
    pub fn write_to<W: Write>(&self, mut dst: &mut W) -> std::io::Result<()> {
        dst.write_u16::<LittleEndian>(self.compressor as u16)?;
        dst.write_u16::<LittleEndian>(self.coder)?;
        self.version.write_to(&mut dst)?;
        dst.write_u32::<LittleEndian>(self.options)?;
        dst.write_u32::<LittleEndian>(self.chunk_size)?;
        dst.write_i64::<LittleEndian>(self.number_of_special_evlrs)?;
        dst.write_i64::<LittleEndian>(self.offset_to_special_evlrs)?;
        write_laz_items_to(&self.items, &mut dst)?;
        Ok(())
    }

    pub fn chunk_size(&self) -> u32 {
        self.chunk_size
    }

    pub fn items(&self) -> &Vec<LazItem> {
        &self.items
    }

    /// Returns the sum of the size of the laz_items, which should correspond to the
    /// expected size of points (uncompressed).
    pub fn items_size(&self) -> u64 {
        u64::from(self.items.iter().map(|item| item.size).sum::<u16>())
    }
}

impl Default for LazVlr {
    fn default() -> Self {
        Self {
            compressor: Default::default(),
            coder: 0,
            version: Version {
                major: 2,
                minor: 2,
                revision: 0,
            },
            options: 0,
            chunk_size: DEFAULT_CHUNK_SIZE as u32,
            number_of_special_evlrs: -1,
            offset_to_special_evlrs: -1,
            items: vec![],
        }
    }
}

pub struct LazVlrBuilder {
    laz_vlr: LazVlr,
}

impl LazVlrBuilder {
    pub fn new() -> Self {
        Self {
            laz_vlr: Default::default(),
        }
    }

    pub fn with_chunk_size(mut self, chunk_size: u32) -> Self {
        self.laz_vlr.chunk_size = chunk_size;
        self
    }

    pub fn with_laz_items(mut self, laz_items: Vec<LazItem>) -> Self {
        self.laz_vlr.items = laz_items;
        self
    }

    pub fn build(self) -> LazVlr {
        self.laz_vlr
    }
}

//TODO fix unwraps
pub fn record_decompressor_from_laz_items<R: Read + Seek + 'static>(
    items: &Vec<LazItem>,
    input: R,
) -> Box<dyn RecordDecompressor<R>> {
    let first_version = items[0].version;
    if !items.iter().all(|item| item.version == first_version) {
        // Technically we could mix version 1&2 and 3&4
        // we just cannot mix non-layered decompressor and layered-decompressor
        panic!("All laz items must have save version");
    }

    match first_version {
        1 | 2 => {
            let mut decompressor = SequentialPointRecordDecompressor::new(input);
            decompressor.set_fields_from(items).unwrap();
            Box::new(decompressor)
        }
        3 | 4 => {
            let mut decompressor = LayeredPointRecordDecompressor::new(input);
            decompressor.set_fields_from(items).unwrap();
            Box::new(decompressor)
        }
        _ => panic!("Unknown laz item version {}", first_version),
    }
}

pub fn record_compressor_from_laz_items<W: Write + 'static>(
    items: &Vec<LazItem>,
    output: W,
) -> Box<dyn RecordCompressor<W>> {
    let first_version = items[0].version;
    if !items.iter().all(|item| item.version == first_version) {
        // Technically we could mix version 1&2 and 3&4
        // we just cannot mix non-layered decompressor and layered-decompressor
        panic!("All laz items must have save version");
    }

    match first_version {
        1 | 2 => {
            let mut compressor = SequentialPointRecordCompressor::new(output);
            compressor.set_fields_from(items).unwrap();
            Box::new(compressor)
        }
        3 | 4 => {
            let mut compressor = LayeredPointRecordCompressor::new(output);
            compressor.set_fields_from(items).unwrap();
            Box::new(compressor)
        }
        _ => panic!("Unknown laz item version {}", first_version),
    }
}


fn read_chunk_table<R: Read + Seek>(mut src: &mut R, mut offset_to_chunk_table: i64) -> std::io::Result<Vec<u64>> {
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

//TODO possible to make the Seek trait optional ?
/// Struct that handles the decompression of the points inside the source
pub struct LasZipDecompressor<R: Read + Seek + Sized + 'static> {
    vlr: LazVlr,
    record_decompressor: Box<dyn RecordDecompressor<R>>,
    chunk_points_read: u32,
    offset_to_chunk_table: i64,
    data_start: u64,
    chunk_table: Option<Vec<u64>>,
}

impl<R: Read + Seek + Sized + 'static> LasZipDecompressor<R> {
    pub fn new_with_record_data(
        source: R,
        laszip_vlr_record_data: &[u8],
    ) -> Result<Self, LasZipError> {
        let vlr = LazVlr::from_buffer(laszip_vlr_record_data)?;
        Self::new(source, vlr)
    }

    pub fn new(mut source: R, vlr: LazVlr) -> Result<Self, LasZipError> {
        if vlr.compressor != CompressorType::PointWiseChunked
            && vlr.compressor != CompressorType::LayeredChunked
        {
            return Err(LasZipError::UnsupportedCompressorType(vlr.compressor));
        }

        let offset_to_chunk_table = source.read_i64::<LittleEndian>()?;
        let data_start = source.seek(SeekFrom::Current(0))?;
        let record_decompressor = record_decompressor_from_laz_items(&vlr.items, source);

        Ok(Self {
            vlr,
            record_decompressor,
            chunk_points_read: 0,
            offset_to_chunk_table,
            data_start,
            chunk_table: None,
        })
    }

    /// Decompress the next point and write the uncompressed data to the out buffer.
    ///
    /// - The buffer should have at least enough byte to store the decompressed data
    /// - The data is written in the buffer exactly as it would have been in a LAS File
    ///     in Little Endian order,
    pub fn decompress_one(&mut self, mut out: &mut [u8]) -> std::io::Result<()> {
        if self.chunk_points_read == self.vlr.chunk_size {
            self.reset_for_new_chunk();
        }
        self.record_decompressor.decompress_next(&mut out)?;
        self.chunk_points_read += 1;
        Ok(())
    }

    pub fn into_stream(self) -> R {
        self.record_decompressor.box_into_stream()
    }

    // FIXME Seeking in Layered Compressed data is untested, make sure it works
    pub fn seek(&mut self, point_idx: u64) -> std::io::Result<()> {
        if let Some(chunk_table) = &self.chunk_table {
            let chunk_of_point = point_idx / self.vlr.chunk_size as u64;
            let delta = point_idx % self.vlr.chunk_size as u64;

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
                    .borrow_stream_mut()
                    .seek(SeekFrom::Start(chunk_table[chunk_of_point as usize] as u64))?;

                self.reset_for_new_chunk();

                for _i in 0..delta {
                    self.decompress_one(&mut tmp_out)?;
                    let current_pos = self
                        .record_decompressor
                        .borrow_stream_mut()
                        .seek(SeekFrom::Current(0))?;

                    if current_pos >= self.offset_to_chunk_table as u64 {
                        self.record_decompressor
                            .borrow_stream_mut()
                            .seek(SeekFrom::End(0))?;
                        return Ok(());
                    }
                }
            } else if let Some(start_of_chunk) = chunk_table.get(chunk_of_point as usize) {
                self.record_decompressor
                    .borrow_stream_mut()
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
                self.record_decompressor
                    .borrow_stream_mut()
                    .seek(SeekFrom::End(0))?;
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
            .set_fields_from(&self.vlr.items)
            .unwrap();
    }

    fn read_chunk_table(&mut self) -> std::io::Result<()> {
        let stream = self.record_decompressor.borrow_stream_mut();
        let chunk_sizes = read_chunk_table(stream, self.offset_to_chunk_table)?;
        let number_of_chunks = chunk_sizes.len();
        let mut chunk_starts = vec![0u64; number_of_chunks as usize];
        chunk_starts[0] = self.data_start;
        for i in 1..number_of_chunks {
            chunk_starts[i as usize] =
                chunk_sizes[(i - 1) as usize] + chunk_starts[(i - 1) as usize];
            /*if (chunk_starts[i] <= chunk_starts[i-1]) {
                //err
            }*/
        }
        self.chunk_table = Some(chunk_starts);
        Ok(())
    }
}


fn write_chunk_table<W: Write>(mut stream: &mut W, chunk_table: &Vec<usize>) -> std::io::Result<()> {
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


/// Updates the 'chunk table offset' is the first 8 byte (i64) of a Laszip compressed data
///
/// This function expects the position of the destination to be at the start of the chunk_table
/// (whether it is written or not).
///
/// This function also expects the i64 to have been already written/reserved
/// (even if its garbage bytes / 0s)
///
/// The position of the destination is untouched
fn update_chunk_table_offset<W: Write + Seek>(dst: &mut W, offset_pos: SeekFrom) -> std::io::Result<()> {
    let start_of_chunk_table_pos = dst.seek(SeekFrom::Current(0))?;
    dst.seek(offset_pos)?;
    dst.write_i64::<LittleEndian>(start_of_chunk_table_pos as i64)?;
    dst.seek(SeekFrom::Start(start_of_chunk_table_pos))?;
    Ok(())
}

/// Struct that handles the compression of the points into the given destination
pub struct LasZipCompressor<W: Write> {
    vlr: LazVlr,
    record_compressor: Box<dyn RecordCompressor<W>>,
    first_point: bool,
    chunk_point_written: u32,
    chunk_sizes: Vec<usize>,
    last_chunk_pos: u64,
    start_pos: u64,
}

// FIXME What laszip does for the chunk table is: if stream is not seekable: chunk table offset is -1
//  write the chunk table  as usual then after (so at the end of the stream write the chunk table
//  that means also support non seekable stream this is waht we have to do
impl<W: Write + Seek + 'static> LasZipCompressor<W> {
    pub fn from_laz_items(output: W, items: Vec<LazItem>) -> Result<Self, LasZipError> {
        let vlr = LazVlr::from_laz_items(items);
        Self::from_laz_vlr(output, vlr)
    }

    pub fn from_laz_vlr(output: W, vlr: LazVlr) -> Result<Self, LasZipError> {
        let record_compressor = record_compressor_from_laz_items(&vlr.items, output);
        Ok(Self {
            vlr,
            record_compressor,
            first_point: true,
            chunk_point_written: 0,
            chunk_sizes: vec![],
            last_chunk_pos: 0,
            start_pos: 0,
        })
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
        if self.first_point {
            let stream = self.record_compressor.borrow_stream_mut();
            self.start_pos = stream.seek(SeekFrom::Current(0))?;
            stream.write_i64::<LittleEndian>(-1)?;
            self.last_chunk_pos = self.start_pos + std::mem::size_of::<i64>() as u64;
            self.first_point = false;
        }

        if self.chunk_point_written == self.vlr.chunk_size {
            self.record_compressor.done()?;
            self.record_compressor.reset();
            self.record_compressor
                .set_fields_from(&self.vlr.items)
                .unwrap();
            self.update_chunk_table()?;
            self.chunk_point_written = 0;
        }

        self.record_compressor.compress_next(&input)?;
        self.chunk_point_written += 1;
        Ok(())
    }

    pub fn done(&mut self) -> std::io::Result<()> {
        self.record_compressor.done()?;
        self.update_chunk_table()?;
        let stream = self.record_compressor.borrow_stream_mut();
        update_chunk_table_offset(stream, SeekFrom::Start(self.start_pos))?;
        write_chunk_table(stream, &self.chunk_sizes)?;
        Ok(())
    }

    pub fn vlr(&self) -> &LazVlr {
        &self.vlr
    }

    pub fn into_stream(self) -> W {
        self.record_compressor.box_into_stream()
    }

    pub fn get_mut(&mut self) -> &mut W {
        self.record_compressor.borrow_stream_mut()
    }


    fn update_chunk_table(&mut self) -> std::io::Result<()> {
        let current_pos = self
            .record_compressor
            .borrow_stream_mut()
            .seek(SeekFrom::Current(0))?;
        self.chunk_sizes
            .push((current_pos - self.last_chunk_pos) as usize);
        self.last_chunk_pos = current_pos;
        Ok(())
    }
}


#[cfg(feature = "parallel")]
pub fn par_compress_all<W: Write + Seek>(dst: &mut W, points: &[u8], items: Vec<LazItem>) -> std::io::Result<()> {
    use rayon::iter::{ParallelIterator, IntoParallelRefIterator};
    use std::io::Cursor;

    let start_pos = dst.seek(SeekFrom::Current(0))?;

    let point_size = items.iter().map(|item| item.size).sum::<u16>() as usize;
    assert_eq!(points.len() % point_size, 0); //TODO proper error
    let points_per_chunk = DEFAULT_CHUNK_SIZE; //TODO make user ba able to chose it
    let chunk_size_in_bytes = points_per_chunk * point_size;
    let number_of_chunks = points.len() / chunk_size_in_bytes;

    let mut all_slices = (0..number_of_chunks)
        .map(|i| &points[(i * chunk_size_in_bytes)..((i + 1) * chunk_size_in_bytes)])
        .collect::<Vec<&[u8]>>();

    if points.len() % chunk_size_in_bytes != 0 {
        all_slices.push(&points[number_of_chunks * chunk_size_in_bytes..]);
    }

    let chunks = all_slices
        .par_iter()
        .map(|slc| {
            let mut record_compressor = record_compressor_from_laz_items(
                &items, Cursor::new(Vec::<u8>::new()),
            );

            for raw_point in slc.windows(point_size) {
                record_compressor.compress_next(raw_point)?;
            }
            record_compressor.done()?;
            Ok(record_compressor.box_into_stream())
        })
        .collect::<Vec<std::io::Result<Cursor<Vec<u8>>>>>();

    // Reserve the bytes for the chunk table offset that will be updated later
    dst.write_i64::<LittleEndian>(0)?;
    let mut chunk_sizes = Vec::<usize>::with_capacity(chunks.len());
    for chunk_result in chunks {
        let chunk = chunk_result?;
        chunk_sizes.push(chunk.get_ref().len());
        dst.write_all(chunk.get_ref())?;
    }
    update_chunk_table_offset(dst, SeekFrom::Start(start_pos))?;
    write_chunk_table(dst, &chunk_sizes)
}

#[cfg(feature = "parallel")]
pub fn par_decompress_all<R: Read + Seek>(src: &mut R, points_out: &mut [u8], laz_vlr: &LazVlr) -> std::io::Result<()> {
    use std::io::Cursor;
    use rayon::iter::{ParallelIterator, IntoParallelIterator};

    let point_size = laz_vlr.items.iter().map(|item| item.size).sum::<u16>() as usize; //TODO make it a function
    assert_eq!(points_out.len() % point_size, 0);
    let num_points_to_decompress = points_out.len() / point_size;

    let mut num_chunks_to_read = num_points_to_decompress / laz_vlr.chunk_size as usize;
    if num_points_to_decompress % laz_vlr.chunk_size as usize != 0 {
        num_chunks_to_read += 1;
    }

    let offset_to_chunk_table = src.read_i64::<LittleEndian>()?;
    let chunk_sizes = read_chunk_table(src, offset_to_chunk_table)?;
    if num_chunks_to_read > chunk_sizes.len() {
        panic!("want to read more chunks than there are");
    }

    let chunks_data: Vec<Cursor<Vec<u8>>> = chunk_sizes[..num_chunks_to_read]
        .iter()
        .map(|size| {
            let mut chunk_bytes = vec![0u8; *size as usize];
            src.read_exact(&mut chunk_bytes)?;
            Ok(Cursor::new(chunk_bytes))
        })
        .collect::<std::io::Result<Vec<Cursor<Vec<u8>>>>>()?;


    let points_per_chunk = laz_vlr.chunk_size as usize;
    let chunk_size_in_bytes = points_per_chunk * point_size;


    let mut decompress_in_out = Vec::<(&mut [u8], Cursor<Vec<u8>>)>::with_capacity(chunks_data.len());
    for (slc_out, chunk_data) in points_out.chunks_mut(chunk_size_in_bytes).into_iter().zip(chunks_data) {
        decompress_in_out.push((slc_out, chunk_data));
    }

    decompress_in_out
        .into_par_iter()
        .map(|(slc_out, src)| {
            let mut record_decompressor = record_decompressor_from_laz_items(laz_vlr.items(), src);
            for raw_point in slc_out.chunks_exact_mut(point_size) {
                record_decompressor.decompress_next(raw_point)?;
            }
            Ok(())
        })
        .collect::<std::io::Result<()>>()
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_can_write_laz_vlr() {
        let vlr = LazVlr::new();
        let mut out = Cursor::new(Vec::<u8>::new());
        vlr.write_to(&mut out).unwrap();
    }

    #[test]
    fn test_create_laz_items() {
        assert_eq!(
            LazItemRecordBuilder::new()
                .add_item(LazItemType::Point10)
                .build()
                .len(),
            1
        );
    }
}
