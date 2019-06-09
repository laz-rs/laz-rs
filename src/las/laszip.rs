use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt};

use crate::decoders::ArithmeticDecoder;
use crate::encoders::ArithmeticEncoder;
use crate::formats::{RecordDecompressor, RecordCompressor};
use crate::errors::Errors;
use crate::las;
use std::convert::TryFrom;

const SUPPORTED_VERSION: u32 = 2;
const DEFAULT_CHUNK_SIZE: usize = 5000;

#[derive(Debug)]
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
}

#[derive(Debug, Copy, Clone)]
pub enum LazItemType {
    Byte(u16),
    //0
    Point10,
    //6
    GpsTime,
    //7
    RGB12,//8
}

impl From<LazItemType> for u16 {
    fn from(t: LazItemType) -> Self {
        match t {
            LazItemType::Byte(_) => 0,
            LazItemType::Point10 => 6,
            LazItemType::GpsTime => 7,
            LazItemType::RGB12 => 8,
        }
    }
}


#[derive(Debug)]
pub struct LazItem {
    // coded on a u16
    item_type: LazItemType,
    size: u16,
    version: u16,
}

impl LazItem {
    fn read_from<R: Read>(src: &mut R) -> std::io::Result<Self> {
        let item_type = src.read_u16::<LittleEndian>()?;
        let size = src.read_u16::<LittleEndian>()?;
        // TODO forward Err
        let item_type = match item_type {
            0 => LazItemType::Byte(size),
            6 => LazItemType::Point10,
            7 => LazItemType::GpsTime,
            8 => LazItemType::RGB12,
            _ => panic!("Unknown LazItem type: {}", item_type)
        };
        Ok(Self {
            item_type,
            size,
            version: src.read_u16::<LittleEndian>()?,
        })
    }
    fn item_type(&self) -> LazItemType {
        self.item_type
    }
}

pub struct LazItemRecordBuilder {
    items: Vec<LazItem>
}

impl LazItemRecordBuilder {
    pub fn new() -> Self {
        Self { items: vec![] }
    }

    pub fn add_item(&mut self, item_type: LazItemType) -> &mut Self {
        let size = match item_type {
            LazItemType::Byte(n) => n,
            LazItemType::Point10 => 20,
            LazItemType::GpsTime => 8,
            LazItemType::RGB12 => 6,
        };

        self.items.push(
            LazItem {
                item_type,
                size,
                version: SUPPORTED_VERSION as u16,
            }
        );
        self
    }

    pub fn build(self) -> Vec<LazItem> {
        self.items
    }
}

fn read_laz_items_from<R: Read>(mut src: &mut R) -> std::io::Result<Vec<LazItem>> {
    let num_items = src.read_u16::<LittleEndian>()?;
    let mut items = Vec::<LazItem>::with_capacity(num_items as usize);
    for _ in 0..num_items {
        items.push(LazItem::read_from(&mut src)?)
    }
    Ok(items)
}

pub enum CompressorType {
    // TODO might need a better name
    None = 0,
    PointWise = 1,
    PointWiseChunked = 2,
    LayeredChunked = 3,
}

#[derive(Debug)]
pub struct LazVlr {
    compressor: u16,
    // 0 means ArithmeticCoder, its the only choice
    coder: u16,

    version: Version,
    options: u32,
    chunk_size: u32,

    // -1 if unused
    number_of_special_evlrs: i64,
    // -1 if unused
    offset_to_special_evlrs: i64,

    items: Vec<LazItem>,
}


impl LazVlr {
    pub fn new() -> Self {
        Self {
            compressor: CompressorType::PointWiseChunked as u16,
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

    pub fn from_buffer(record_data: &[u8]) -> std::io::Result<Self> {
        let mut cursor = std::io::Cursor::new(record_data);
        Self::read_from(&mut cursor)
    }

    pub fn read_from<R: Read>(mut src: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            compressor: src.read_u16::<LittleEndian>()?,
            coder: src.read_u16::<LittleEndian>()?,
            version: Version::read_from(&mut src)?,
            options: src.read_u32::<LittleEndian>()?,
            chunk_size: src.read_u32::<LittleEndian>()?,
            number_of_special_evlrs: src.read_i64::<LittleEndian>()?,
            offset_to_special_evlrs: src.read_i64::<LittleEndian>()?,
            items: read_laz_items_from(&mut src)?,
        })
    }

    pub fn compressor_type(&self) -> Option<CompressorType> {
        match self.compressor {
            0 => Some(CompressorType::None),
            1 => Some(CompressorType::PointWise),
            2 => Some(CompressorType::PointWiseChunked),
            3 => Some(CompressorType::LayeredChunked),
            _ => None
        }
    }
}


pub struct LasZipDecompressor<R: Read> {
    vlr: LazVlr,
    record_decompressor: RecordDecompressor<R>,
    chunk_points_read: u32,
}


impl<R: Read> LasZipDecompressor<R> {
    //TODO FIXME WOW FIND A BETTER NAME WTF
    pub fn new_1(mut source: R, laszip_vlr_record_data: &[u8]) -> Self {
        let vlr = LazVlr::from_buffer(laszip_vlr_record_data).unwrap();
        Self::new(source, vlr)
    }

    //TODO add point_size as params to have some check later
    pub fn new(mut source: R, vlr: LazVlr) -> Self {
        let mut record_decompressor = RecordDecompressor::new(ArithmeticDecoder::new(source));
        for record_item in &vlr.items {
            match record_item.item_type {
                LazItemType::Byte(_) => record_decompressor.add_field(las::extra_bytes::ExtraBytesDecompressor::new(record_item.size as usize)),
                LazItemType::Point10 => record_decompressor.add_field(las::point10::Point10Decompressor::new()),
                LazItemType::GpsTime => record_decompressor.add_field(las::gps::GpsTimeDecompressor::new()),
                LazItemType::RGB12 => record_decompressor.add_field(las::rgb::RGBDecompressor::new()),
            }
        }
        Self {
            vlr,
            record_decompressor,
            chunk_points_read: 0,
        }
    }

    pub fn decompress_one(&mut self, mut out: &mut [u8]) {
        if self.chunk_points_read == self.vlr.chunk_size {
            self.record_decompressor.reset();
            self.chunk_points_read = 0;
        }

        self.record_decompressor.decompress(&mut out);
        self.chunk_points_read += 1;
    }

    pub fn into_stream(self) -> R {
        self.record_decompressor.into_stream()
    }
}

pub struct LasZipCompressor<W: Write> {
    vlr: LazVlr,
    record_compressor: RecordCompressor<W>,
    first_point: bool,
    chunk_point_written: u32,
}

// TODO impl for W: Write + Seek update chunktale offset ?
// TODO chunkTable
impl<W: Write> LasZipCompressor<W> {
    pub fn from_laz_items(output: W, items: Vec<LazItem>) -> Self {
        let mut record_compressor = RecordCompressor::new(
            ArithmeticEncoder::new(output));
        for item in &items {
            match item.item_type {
                LazItemType::Byte(count) =>
                    record_compressor.add_field_compressor(
                        las::extra_bytes::ExtraBytesCompressor::new(count as usize)),
                LazItemType::Point10 =>
                    record_compressor.add_field_compressor(las::point10::Point10Compressor::new()),
                LazItemType::GpsTime =>
                    record_compressor.add_field_compressor(las::gps::GpsTimeCompressor::new()),
                LazItemType::RGB12 =>
                    record_compressor.add_field_compressor(las::rgb::RGBCompressor::new())
            }
        }
        let vlr = LazVlr::from_laz_items(items);

        Self {
            vlr,
            record_compressor,
            first_point: true,
            chunk_point_written: 0,
        }
    }

    pub fn compress_one(&mut self, input: &[u8]) {
        if self.first_point {
            //TODO borrow stream and write emtpy chunk offset size
            //self.record_compressor.
            self.first_point = false;
        }

        if self.chunk_point_written == self.vlr.chunk_size {
            self.record_compressor.reset();
            self.chunk_point_written = 0;
            //TODO update chunk table
        }

        self.record_compressor.compress(&input);

        self.chunk_point_written += 1;
    }

    pub fn done(&mut self) {
        self.record_compressor.done();
        //TODO write chunktable
    }
}