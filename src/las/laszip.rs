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

pub const LASZIP_USER_ID: &str = "laszip encoded";
pub const LASZIP_RECORD_ID: u16 = 22204;
pub const LASZIP_DESCRIPTION: &str = "http://laszip.org";

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

/// The different type of data / fields found in the definition of LAS points
#[derive(Debug, Copy, Clone)]
pub enum LazItemType {
    /// ExtraBytes for LAS versions <= 1.3 & point format <= 5
    Byte(u16),
    /// Point10 is the Point format id 0 of LAS for versions <= 1.3 & point format <= 5
    Point10,
    /// GpsTime for LAS versions <= 1.3 & point format <= 5
    GpsTime,
    /// RGB for LAS versions <= 1.3 & point format <= 5
    RGB12,
    //WavePacket13,
    /// Point14 is the Point format id 6 of LAS for versions >= 1.4 & point format >= 6
    Point14,
    /// RGB for LAS versions >= 1.4
    RGB14,
    /// RGB + Nir for LAS versions >= 1.4
    RGBNIR14,
    //WavePacket14,
    /// ExtraBytes for LAS versions >= 1.4
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
    pub fn new(item_type: LazItemType, version: u16) -> Self {
        let size = item_type.size();
        Self {
            item_type,
            size,
            version,
        }
    }

    pub fn item_type(&self) -> LazItemType {
        self.item_type
    }

    pub fn size(&self) -> u16 {
        self.size
    }

    pub fn version(&self) -> u16 {
        self.version
    }

    fn read_from<R: Read>(src: &mut R) -> crate::Result<Self> {
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

pub struct LazItems(Vec<LazItem>);

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

    pub fn default_for_point_format_id(
        point_format_id: u8,
        num_extra_bytes: u16,
    ) -> crate::Result<Vec<LazItem>> {
        use crate::las::{Point1, Point2, Point3, Point7, Point8};
        match point_format_id {
            0 => Ok(LazItemRecordBuilder::default_version_of::<Point0>(
                num_extra_bytes,
            )),
            1 => Ok(LazItemRecordBuilder::default_version_of::<Point1>(
                num_extra_bytes,
            )),
            2 => Ok(LazItemRecordBuilder::default_version_of::<Point2>(
                num_extra_bytes,
            )),
            3 => Ok(LazItemRecordBuilder::default_version_of::<Point3>(
                num_extra_bytes,
            )),
            6 => Ok(LazItemRecordBuilder::default_version_of::<Point6>(
                num_extra_bytes,
            )),
            7 => Ok(LazItemRecordBuilder::default_version_of::<Point7>(
                num_extra_bytes,
            )),
            8 => Ok(LazItemRecordBuilder::default_version_of::<Point8>(
                num_extra_bytes,
            )),
            _ => Err(LasZipError::UnsupportedPointFormat(point_format_id)),
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
                let size = item_type.size();
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

fn read_laz_items_from<R: Read>(mut src: &mut R) -> crate::Result<Vec<LazItem>> {
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
///
/// This vlr contains information needed to compress or decompress
/// LAZ/LAS data. Such as the points per chunk, the fields & version
/// of the compression/decompression algorithm.
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
    pub fn from_laz_items(items: Vec<LazItem>) -> Self {
        let first_item = items
            .first()
            .expect("Vec<LazItem> should at least have one element");
        let compressor = match first_item.version {
            1 | 2 => CompressorType::PointWiseChunked,
            3 | 4 => CompressorType::LayeredChunked,
            _ => panic!("Unknown laz_item version"),
        };
        Self {
            compressor,
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
            items,
        }
    }

    /// Tries to read the Vlr information from the record_data buffer
    pub fn from_buffer(record_data: &[u8]) -> crate::Result<Self> {
        let mut cursor = std::io::Cursor::new(record_data);
        Self::read_from(&mut cursor)
    }

    /// Tries to read the Vlr information from the record_data source
    pub fn read_from<R: Read>(mut src: &mut R) -> crate::Result<Self> {
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

/// Builder struct to personalize the LazVlr
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

fn record_decompressor_from_laz_items<'a, R: Read + Seek + 'a>(
    items: &Vec<LazItem>,
    input: R,
) -> crate::Result<Box<dyn RecordDecompressor<R> + 'a>> {
    let first_item = items
        .get(0)
        .expect("There should be at least one LazItem to be able to create a RecordDecompressor");

    let mut decompressor = match first_item.version {
        1 | 2 => {
            let decompressor = SequentialPointRecordDecompressor::new(input);
            Box::new(decompressor) as Box<dyn RecordDecompressor<R>>
        }
        3 | 4 => {
            let decompressor = LayeredPointRecordDecompressor::new(input);
            Box::new(decompressor) as Box<dyn RecordDecompressor<R>>
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

fn record_compressor_from_laz_items<'a, W: Write + 'a>(
    items: &Vec<LazItem>,
    output: W,
) -> crate::Result<Box<dyn RecordCompressor<W> + 'a>> {
    let first_item = items
        .get(0)
        .expect("There should be at least one LazItem to be able to create a RecordCompressor");

    let mut compressor = match first_item.version {
        1 | 2 => {
            let compressor = SequentialPointRecordCompressor::new(output);
            Box::new(compressor) as Box<dyn RecordCompressor<W>>
        }
        3 | 4 => {
            let compressor = LayeredPointRecordCompressor::new(output);
            Box::new(compressor) as Box<dyn RecordCompressor<W>>
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

    if offset_to_chunk_table >= 0 && offset_to_chunk_table as u64 == current_pos {
        // In that case the compressor was probably stopped
        // before being able to write the chunk table
        None
    } else {
        Some(read_chunk_table_at_offset(src, offset_to_chunk_table))
    }
}

fn read_chunk_table_at_offset<R: Read + Seek>(
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

/// Struct that handles the decompression of the points written in a LAZ file
pub struct LasZipDecompressor<'a, R: Read + Seek + 'a> {
    vlr: LazVlr,
    record_decompressor: Box<dyn RecordDecompressor<R> + 'a>,
    chunk_points_read: u32,
    offset_to_chunk_table: i64,
    data_start: u64,
    chunk_table: Option<Vec<u64>>,
}

impl<'a, R: Read + Seek + 'a> LasZipDecompressor<'a, R> {
    /// Creates a new instance from a data source of compressed points
    /// and the `record data` of the laszip vlr
    pub fn new_with_record_data(source: R, laszip_vlr_record_data: &[u8]) -> crate::Result<Self> {
        let vlr = LazVlr::from_buffer(laszip_vlr_record_data)?;
        Self::new(source, vlr)
    }

    /// Creates a new instance from a data source of compressed points
    /// and the LazVlr describing the compressed data
    pub fn new(mut source: R, vlr: LazVlr) -> crate::Result<Self> {
        if vlr.compressor != CompressorType::PointWiseChunked
            && vlr.compressor != CompressorType::LayeredChunked
        {
            return Err(LasZipError::UnsupportedCompressorType(vlr.compressor));
        }

        let offset_to_chunk_table = source.read_i64::<LittleEndian>()?;
        let data_start = source.seek(SeekFrom::Current(0))?;
        let record_decompressor = record_decompressor_from_laz_items(&vlr.items, source)?;

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

    /// Decompress as many points as the `out` slice can hold
    ///
    /// # Note
    ///
    /// If the `out` slice contains more space than there are points
    /// the function will still decompress and thus and error will occur
    pub fn decompress_many(&mut self, out: &mut [u8]) -> std::io::Result<()> {
        for point in out.chunks_exact_mut(self.vlr.items_size() as usize) {
            self.decompress_one(point)?;
        }
        Ok(())
    }

    pub fn vlr(&self) -> &LazVlr {
        &self.vlr
    }

    pub fn into_stream(self) -> R {
        self.record_decompressor.box_into_stream()
    }

    // FIXME Seeking in Layered Compressed data is untested, make sure it works
    /// Seeks to the point designed by the index
    ///
    /// # Important
    ///
    /// Seeking in compressed data has a higher cost than non compressed data
    /// because the stream has to be moved to the start of the chunk
    /// and then we have to decompress points in the chunk until we reach the
    /// one we want.
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
        let chunk_sizes = read_chunk_table_at_offset(stream, self.offset_to_chunk_table)?;
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

fn write_chunk_table<W: Write>(
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
fn update_chunk_table_offset<W: Write + Seek>(
    dst: &mut W,
    offset_pos: SeekFrom,
) -> std::io::Result<()> {
    let start_of_chunk_table_pos = dst.seek(SeekFrom::Current(0))?;
    dst.seek(offset_pos)?;
    dst.write_i64::<LittleEndian>(start_of_chunk_table_pos as i64)?;
    dst.seek(SeekFrom::Start(start_of_chunk_table_pos))?;
    Ok(())
}

/// Struct that handles the compression of the points into the given destination
pub struct LasZipCompressor<'a, W: Write + 'a> {
    vlr: LazVlr,
    record_compressor: Box<dyn RecordCompressor<W> + 'a>,
    first_point: bool,
    chunk_point_written: u32,
    chunk_sizes: Vec<usize>,
    last_chunk_pos: u64,
    start_pos: u64,
}

// FIXME What laszip does for the chunk table is: if stream is not seekable: chunk table offset is -1
//  write the chunk table  as usual then after (so at the end of the stream write the chunk table
//  that means also support non seekable stream this is waht we have to do
impl<'a, W: Write + Seek + 'a> LasZipCompressor<'a, W> {
    /// Creates a new LasZipCompressor using the items provided,
    ///
    /// If you wish to use a different `chunk size` see [`from_laz_vlr`]
    ///
    /// [`from_laz_vlr`]: #method.from_laz_vlr
    pub fn from_laz_items(output: W, items: Vec<LazItem>) -> crate::Result<Self> {
        let vlr = LazVlr::from_laz_items(items);
        Self::from_laz_vlr(output, vlr)
    }

    /// Creates a compressor using the provided vlr.
    pub fn from_laz_vlr(output: W, vlr: LazVlr) -> crate::Result<Self> {
        let record_compressor = record_compressor_from_laz_items(&vlr.items, output)?;
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
        let stream = self.record_compressor.borrow_stream_mut();
        update_chunk_table_offset(stream, SeekFrom::Start(self.start_pos))?;
        write_chunk_table(stream, &self.chunk_sizes)?;
        Ok(())
    }

    /// Returns the vlr used by this compressor
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
pub fn compress_buffer<W: Write + Seek>(
    dst: &mut W,
    uncompressed_points: &[u8],
    laz_vlr: LazVlr,
) -> crate::Result<()> {
    let mut compressor = LasZipCompressor::from_laz_vlr(dst, laz_vlr)?;
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

/// Decompresses all points from the buffer
///
/// The `compressed_points_data` slice must contain all the laszip data
/// that means:
///   1) The offset to the chunk table (i64)
///   2) the compressed points
///   3) the chunk table (optional)
///
///
/// This fn will decompress as many points as the `decompress_points` can hold.
///
/// # Important
///
/// In a LAZ file, the chunk table offset is counted from the start of the
/// LAZ file. Here since we only have the buffer points data, you must make
/// sure the offset is counted since the start of point data.
///
/// So you should update the value before calling this function.
/// Otherwise you will get an IoError like 'failed to fill whole buffer'
/// due to this function seeking past the end of the data.
pub fn decompress_buffer(
    compressed_points_data: &[u8],
    decompressed_points: &mut [u8],
    laz_vlr: LazVlr,
) -> crate::Result<()> {
    let point_size = laz_vlr.items_size() as usize;
    if decompressed_points.len() % point_size != 0 {
        Err(LasZipError::BufferLenNotMultipleOfPointSize {
            buffer_len: decompressed_points.len(),
            point_size,
        })
    } else {
        let src = std::io::Cursor::new(compressed_points_data);
        LasZipDecompressor::new(src, laz_vlr).and_then(|mut decompressor| {
            decompressor.decompress_many(decompressed_points)?;
            Ok(())
        })
    }
}

/// Compresses all points in parallel
///
/// Just like [`compress_buffer`] but the compression is done in multiple threads
///
/// # Note
///
/// Point order [is conserved](https://github.com/rayon-rs/rayon/issues/551)
///
/// [`compress_buffer`]: fn.compress_buffer.html
#[cfg(feature = "parallel")]
pub fn par_compress_buffer<W: Write + Seek>(
    dst: &mut W,
    uncompressed_points: &[u8],
    laz_vlr: &LazVlr,
) -> crate::Result<()> {
    let start_pos = dst.seek(SeekFrom::Current(0))?;
    // Reserve the bytes for the chunk table offset that will be updated later
    dst.write_i64::<LittleEndian>(start_pos as i64)?;

    let chunk_sizes = par_compress(dst, uncompressed_points, laz_vlr)?;

    update_chunk_table_offset(dst, SeekFrom::Start(start_pos))?;
    write_chunk_table(dst, &chunk_sizes)?;
    Ok(())
}

/// Compresses the points contained in `uncompressed_points` writing the result in the `dst`
/// and returns the size of each chunk
///
/// Does not write nor update the offset to the chunk table
/// And does not write the chunk table
///
/// Returns the size of each compressed chunk of point written
#[cfg(feature = "parallel")]
pub fn par_compress<W: Write>(
    dst: &mut W,
    uncompressed_points: &[u8],
    laz_vlr: &LazVlr,
) -> crate::Result<Vec<usize>> {
    use rayon::iter::{IntoParallelIterator, ParallelIterator};
    use std::io::Cursor;

    let point_size = laz_vlr.items_size() as usize;
    if uncompressed_points.len() % point_size != 0 {
        Err(LasZipError::BufferLenNotMultipleOfPointSize {
            buffer_len: uncompressed_points.len(),
            point_size,
        })
    } else {
        let points_per_chunk = laz_vlr.chunk_size as usize;
        let chunk_size_in_bytes = points_per_chunk * point_size;

        // The last chunk may not have the same size,
        // the chunks() method takes care of that for us
        let all_slices = uncompressed_points
            .chunks(chunk_size_in_bytes)
            .collect::<Vec<_>>();

        let chunks = all_slices
            .into_par_iter()
            .map(|slc| {
                let mut record_compressor = record_compressor_from_laz_items(
                    &laz_vlr.items,
                    Cursor::new(Vec::<u8>::new()),
                )?;

                for raw_point in slc.chunks_exact(point_size) {
                    record_compressor.compress_next(raw_point)?;
                }
                record_compressor.done()?;

                Ok(record_compressor.box_into_stream())
            })
            .collect::<Vec<crate::Result<Cursor<Vec<u8>>>>>();

        let mut chunk_sizes = Vec::<usize>::with_capacity(chunks.len());
        for chunk_result in chunks {
            let chunk = chunk_result?;
            chunk_sizes.push(chunk.get_ref().len());
            dst.write_all(chunk.get_ref())?;
        }
        Ok(chunk_sizes)
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
    let point_size = laz_vlr.items_size() as usize;
    if decompressed_points.len() % point_size != 0 {
        Err(LasZipError::BufferLenNotMultipleOfPointSize {
            buffer_len: decompressed_points.len(),
            point_size,
        })
    } else {
        let mut cursor = std::io::Cursor::new(compressed_points_data);
        let offset_to_chunk_table = cursor.read_i64::<LittleEndian>()?;
        let chunk_sizes = read_chunk_table_at_offset(&mut cursor, offset_to_chunk_table)?;

        let compressed_points =
            &compressed_points_data[std::mem::size_of::<i64>()..offset_to_chunk_table as usize];
        par_decompress(
            compressed_points,
            decompressed_points,
            laz_vlr,
            &chunk_sizes,
        )
    }
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
    chunk_sizes: &[u64],
) -> crate::Result<()> {
    use crate::byteslice::ChunksIrregular;
    use rayon::iter::{IntoParallelIterator, ParallelIterator};

    let point_size = laz_vlr.items_size() as usize;
    let decompressed_chunk_size = laz_vlr.chunk_size as usize * point_size;
    let sizes = chunk_sizes
        .iter()
        .map(|s| *s as usize)
        .collect::<Vec<usize>>();
    let input_chunks_iter = ChunksIrregular::new(compressed_points, &sizes);
    let output_chunks_iter = decompressed_points.chunks_mut(decompressed_chunk_size as usize);

    // FIXME we collect into a Vec because zip cannot be made 'into_par_iter' by rayon
    //  (or at least i don't know how)
    let decompression_jobs: Vec<(&[u8], &mut [u8])> =
        input_chunks_iter.zip(output_chunks_iter).collect();
    decompression_jobs
        .into_par_iter()
        .map(|(chunk_in, chunk_out)| {
            let src = std::io::Cursor::new(chunk_in);
            let mut record_decompressor = record_decompressor_from_laz_items(laz_vlr.items(), src)?;
            for raw_point in chunk_out.chunks_exact_mut(point_size) {
                record_decompressor.decompress_next(raw_point)?;
            }
            Ok(())
        })
        .collect::<crate::Result<()>>()?;
    Ok(())
}

/// Decompress points from the file in parallel greedily
///
/// What is meant by 'greedy' here is that this function
/// will read in memory all the compressed points in order to decompress them
/// as opposed to reading a chunk of points when needed
///
/// This fn will decompress as many points as the `decompress_points` can hold.
/// (But will still load the whole point data in memory even if the
/// `decompress_points` cannot hold all the points), meaning that points that could
/// not fit in the `points_out` buffer will be lost.
///
/// Each chunk is sent for decompression in a thread.
///
///
/// `src` must be at the start of the LAZ point data
#[cfg(feature = "parallel")]
pub fn par_decompress_all_from_file_greedy(
    src: &mut std::io::BufReader<std::fs::File>,
    points_out: &mut [u8],
    laz_vlr: &LazVlr,
) -> crate::Result<()> {
    let point_size = laz_vlr.items_size() as usize;
    if points_out.len() % point_size != 0 {
        Err(LasZipError::BufferLenNotMultipleOfPointSize {
            buffer_len: points_out.len(),
            point_size,
        })
    } else {
        let chunk_table = read_chunk_table(src).ok_or(LasZipError::MissingChunkTable)??;

        let point_data_size = chunk_table.iter().copied().sum::<u64>();

        let mut compressed_points = vec![0u8; point_data_size as usize];
        src.read_exact(&mut compressed_points)?;
        par_decompress(&compressed_points, points_out, laz_vlr, &chunk_table)
    }
}

/// LasZip compressor that compresses using multiple threads
///
/// This works by forming complete chunks of points with the points
/// data passed when [`compress_many`] is called. These complete chunks are
/// compressed & written right away and points that are 'leftovers' are kept until
/// the next call to [`compress_many`] or [`done`].
///
/// [`compress_many`]: ./struct.ParLasZipCompressor.html#method.compress_many
/// [`done`]: ./struct.ParLasZipCompressor.html#method.done
#[cfg(feature = "parallel")]
pub struct ParLasZipCompressor<W> {
    vlr: LazVlr,
    chunk_table: Vec<usize>,
    table_offset: u64,
    // Stores uncompressed points from the last call to compress_many
    // that did not allow to make a full chunk of the requested vlr.chunk_size
    // They are prepended to the points data passed to the compress_many fn.
    // The rest is compressed when done is called, forming the last chunk
    rest: Vec<u8>,
    // Because our par_compress function expects a contiguous
    // slice with the points to be compressed, we need an internal buffer to
    // copy the rest + the points to be able to call that fn
    internal_buffer: Vec<u8>,
    dest: W,
}

#[cfg(feature = "parallel")]
impl<W: Write + Seek> ParLasZipCompressor<W> {
    /// Creates a new ParLasZipCompressor
    pub fn new(dest: W, vlr: LazVlr) -> crate::Result<Self> {
        let mut myself = Self {
            vlr,
            chunk_table: vec![],
            table_offset: 0,
            rest: vec![],
            internal_buffer: vec![],
            dest,
        };
        myself.reserve_chunk_table_offset()?;
        Ok(myself)
    }

    fn reserve_chunk_table_offset(&mut self) -> std::io::Result<()> {
        self.table_offset = self.dest.seek(SeekFrom::Current(0))?;
        self.dest
            .write_i64::<LittleEndian>(self.table_offset as i64)
    }

    /// Compresses many points using multiple threads
    ///
    /// For this function to actually use multiple threads, the `points`
    /// buffer shall hold more points that the vlr's `chunk_size`.
    pub fn compress_many(&mut self, points: &[u8]) -> std::io::Result<()> {
        let point_size = self.vlr.items_size() as usize;
        debug_assert_eq!(self.rest.len() % point_size, 0);

        let chunk_size_in_bytes = self.vlr.chunk_size() as usize * point_size;

        let num_chunk = (self.rest.len() + points.len()) / chunk_size_in_bytes;
        let num_bytes_not_fitting = (self.rest.len() + points.len()) % chunk_size_in_bytes;

        self.internal_buffer
            .resize(num_chunk * chunk_size_in_bytes, 0u8);

        if num_chunk > 0 {
            self.internal_buffer[..self.rest.len()].copy_from_slice(&self.rest);
            self.internal_buffer[self.rest.len()..]
                .copy_from_slice(&points[..points.len() - num_bytes_not_fitting]);

            self.rest.resize(num_bytes_not_fitting, 0u8);
            self.rest
                .copy_from_slice(&points[points.len() - num_bytes_not_fitting..]);
            let chunk_sizes =
                par_compress(&mut self.dest, &self.internal_buffer, &self.vlr).unwrap();
            chunk_sizes
                .iter()
                .copied()
                .map(|size| size as usize)
                .for_each(|size| self.chunk_table.push(size));
        } else {
            for b in points {
                self.rest.push(*b);
            }
        }
        Ok(())
    }

    /// Tells the compressor that no more points will be compressed
    ///
    /// - Compresses & writes the rest of the points to form the last chunk
    /// - Writes the chunk table
    /// - update the offset to the chunk_table
    pub fn done(&mut self) -> crate::Result<()> {
        let chunk_sizes = par_compress(&mut self.dest, &self.rest, &self.vlr)?;
        chunk_sizes
            .iter()
            .copied()
            .map(|size| size as usize)
            .for_each(|size| self.chunk_table.push(size));
        update_chunk_table_offset(&mut self.dest, SeekFrom::Start(self.table_offset))?;
        write_chunk_table(&mut self.dest, &self.chunk_table)?;
        Ok(())
    }

    pub fn vlr(&self) -> &LazVlr {
        &self.vlr
    }
}

#[cfg(feature = "parallel")]
/// Laszip decompressor, that can decompress data using multiple threads
pub struct ParLasZipDecompressor<R> {
    vlr: LazVlr,
    chunk_table: Vec<u64>,
    last_chunk_read: usize,
    rest: std::io::Cursor<Vec<u8>>,
    internal_buffer: Vec<u8>,
    outernal_buffer: Vec<u8>,
    source: R,
}

#[cfg(feature = "parallel")]
impl<R: Read + Seek> ParLasZipDecompressor<R> {
    /// Creates a new decompressor
    ///
    /// Fails if no chunk table could be found.
    pub fn new(mut source: R, vlr: LazVlr) -> crate::Result<Self> {
        let chunk_table = read_chunk_table(&mut source).ok_or(LasZipError::MissingChunkTable)??;

        Ok(Self {
            source,
            vlr,
            chunk_table,
            rest: std::io::Cursor::<Vec<u8>>::new(vec![]),
            internal_buffer: vec![],
            outernal_buffer: vec![],
            last_chunk_read: 0,
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

        if num_bytes_in_rest >= out.len() {
            self.rest.read(out)?;
        } else {
            let num_bytes_in_chunk =
                self.vlr.chunk_size() as usize * self.vlr.items_size() as usize;
            let num_chunks_to_decompress = ((out.len() - num_bytes_in_rest) as f32
                / num_bytes_in_chunk as f32)
                .ceil() as usize;

            let chunk_sizes = &self.chunk_table
                [self.last_chunk_read..self.last_chunk_read + num_chunks_to_decompress];
            let bytes_to_read = chunk_sizes.iter().copied().sum::<u64>() as usize;

            self.internal_buffer.resize(bytes_to_read, 0u8);
            self.outernal_buffer
                .resize(num_chunks_to_decompress * num_bytes_in_chunk, 0u8);
            self.source.read(&mut self.internal_buffer)?;

            if self.last_chunk_read + num_chunks_to_decompress < self.chunk_table.len() {
                par_decompress(
                    &self.internal_buffer,
                    &mut self.outernal_buffer,
                    &self.vlr,
                    &chunk_sizes,
                )?;
            } else {
                // The last chunk contains a number of points that is less or equal to the chunk_size
                // the strategy is to decompress that particular chunk separately until a end of file error appears
                par_decompress(
                    &self.internal_buffer,
                    &mut self.outernal_buffer,
                    &self.vlr,
                    &chunk_sizes[..chunk_sizes.len() - 1],
                )?;

                let last_chunk_start = bytes_to_read - (*chunk_sizes.last().unwrap() as usize);
                let last_chunk_source =
                    std::io::Cursor::new(&self.internal_buffer[last_chunk_start..]);
                let mut decompressor =
                    record_decompressor_from_laz_items(self.vlr.items(), last_chunk_source)?;
                debug_assert_eq!(
                    self.outernal_buffer[(num_chunks_to_decompress - 1) * num_bytes_in_chunk..]
                        .len()
                        % point_size,
                    0
                );
                let mut num_decompressed_bytes_in_last_chunk = 0;
                for point in self.outernal_buffer
                    [(num_chunks_to_decompress - 1) * num_bytes_in_chunk..]
                    .chunks_exact_mut(point_size)
                {
                    if let Err(error) = decompressor.decompress_next(point) {
                        if error.kind() == std::io::ErrorKind::UnexpectedEof {
                            break;
                        } else {
                            return Err(error.into());
                        }
                    } else {
                        num_decompressed_bytes_in_last_chunk += point_size;
                    }
                }
                self.outernal_buffer.resize(
                    (num_chunks_to_decompress - 1) * num_bytes_in_chunk
                        + num_decompressed_bytes_in_last_chunk,
                    0u8,
                );
            }

            let num_bytes_not_fitting =
                (self.outernal_buffer.len() + num_bytes_in_rest) - out.len();
            self.rest.read(&mut out[..num_bytes_in_rest])?;
            out[num_bytes_in_rest..].copy_from_slice(
                &self.outernal_buffer[..self.outernal_buffer.len() - num_bytes_not_fitting],
            );
            debug_assert_eq!(
                self.rest.position() as usize,
                self.rest.get_ref().len(),
                "The rest was not consumed"
            );
            {
                let rest_vec = self.rest.get_mut();
                rest_vec.resize(num_bytes_not_fitting, 0u8);
                rest_vec.copy_from_slice(
                    &self.outernal_buffer[self.outernal_buffer.len() - num_bytes_not_fitting..],
                );
                self.rest.set_position(0);
            }

            self.last_chunk_read += num_chunks_to_decompress;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
