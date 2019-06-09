use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt};

use crate::decoders::ArithmeticDecoder;
use crate::formats::{RecordDecompressor, RecordCompressor};
use crate::las;

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

enum LazItemType {
    Byte = 0,
    Point10 = 6,
    GpsTime = 7,
    RGB12 = 8,
}

#[derive(Debug)]
struct LazItem {
    item_type: u16,
    size: u16,
    version: u16,
}

impl LazItem {
    fn read_from<R: Read>(src: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            item_type: src.read_u16::<LittleEndian>()?,
            size: src.read_u16::<LittleEndian>()?,
            version: src.read_u16::<LittleEndian>()?,
        })
    }
    fn item_type(&self) -> Option<LazItemType> {
        match self.item_type {
            0 => Some(LazItemType::Byte),
            6 => Some(LazItemType::Point10),
            7 => Some(LazItemType::GpsTime),
            8 => Some(LazItemType::RGB12),
            _ => None,
        }
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

#[derive(Debug)]
pub struct LazVlr {
    compressor: u16,
    coder: u16,

    version: Version,
    options: u32,
    chunk_size: u32,

    num_points: i64,
    num_bytes: i64,

    items: Vec<LazItem>,
}


impl LazVlr {
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
            num_points: src.read_i64::<LittleEndian>()?,
            num_bytes: src.read_i64::<LittleEndian>()?,
            items: read_laz_items_from(&mut src)?,
        })
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
            // TODO convert a proper error
            let t = record_item.item_type().expect("Invalid LazItemType in VLR");
            match t {
                LazItemType::Byte => record_decompressor.add_field(las::extra_bytes::ExtraBytesDecompressor::new(record_item.size as usize)),
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
    record_compressor: RecordCompressor<W>,
    chunk_point_written: u32,
}