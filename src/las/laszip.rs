use std::io::{Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::compressors::IntegerCompressorBuilder;
use crate::decoders::ArithmeticDecoder;
use crate::encoders::ArithmeticEncoder;
use crate::formats::{RecordCompressor, RecordDecompressor};

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

    fn write_to<W: Write>(&self, dst: &mut W) -> std::io::Result<()> {
        dst.write_u8(self.major)?;
        dst.write_u8(self.minor)?;
        dst.write_u16::<LittleEndian>(self.revision)?;
        Ok(())
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
    RGB12, //8
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
    pub(crate) item_type: LazItemType,
    pub(crate) size: u16,
    pub(crate) version: u16,
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
            _ => panic!("Unknown LazItem type: {}", item_type),
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

pub struct LazItemRecordBuilder {
    items: Vec<LazItemType>,
}

impl LazItemRecordBuilder {
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
                let size = match *item_type {
                    LazItemType::Byte(n) => n,
                    LazItemType::Point10 => 20,
                    LazItemType::GpsTime => 8,
                    LazItemType::RGB12 => 6,
                };
                LazItem {
                    item_type: *item_type,
                    size,
                    version: SUPPORTED_VERSION as u16,
                }
            })
            .collect()
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

fn write_laz_items_to<W: Write>(laz_items: &Vec<LazItem>, mut dst: &mut W) -> std::io::Result<()> {
    dst.write_u16::<LittleEndian>(laz_items.len() as u16)?;
    for item in laz_items {
        item.write_to(&mut dst)?;
    }
    Ok(())
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

    pub fn write_to<W: Write>(&self, mut dst: &mut W) -> std::io::Result<()> {
        dst.write_u16::<LittleEndian>(self.compressor.into())?;
        dst.write_u16::<LittleEndian>(self.coder)?;
        self.version.write_to(&mut dst)?;
        dst.write_u32::<LittleEndian>(self.options)?;
        dst.write_u32::<LittleEndian>(self.chunk_size)?;
        dst.write_i64::<LittleEndian>(self.number_of_special_evlrs)?;
        dst.write_i64::<LittleEndian>(self.offset_to_special_evlrs)?;
        write_laz_items_to(&self.items, &mut dst)?;
        Ok(())
    }

    pub fn compressor_type(&self) -> Option<CompressorType> {
        match self.compressor {
            0 => Some(CompressorType::None),
            1 => Some(CompressorType::PointWise),
            2 => Some(CompressorType::PointWiseChunked),
            3 => Some(CompressorType::LayeredChunked),
            _ => None,
        }
    }
}
//TODO: would it be possible to extract some logic to a ChunkedCompressor & ChunkedDecompressor ?
pub struct LasZipDecompressor<R: Read> {
    vlr: LazVlr,
    record_decompressor: RecordDecompressor<R>,
    chunk_points_read: u32,
}

impl<R: Read> LasZipDecompressor<R> {
    //TODO FIXME WOW FIND A BETTER NAME WTF
    pub fn new_1(source: R, laszip_vlr_record_data: &[u8]) -> Self {
        let vlr = LazVlr::from_buffer(laszip_vlr_record_data).unwrap();
        Self::new(source, vlr)
    }

    //TODO add point_size as params to have some check later
    pub fn new(source: R, vlr: LazVlr) -> Self {
        /*
        if vlr.version.major != SUPPORTED_VERSION as u8 {
            panic!("Unsupported Laszip version: {}", vlr.version.major);
        }
        */

        let mut record_decompressor =
            RecordDecompressor::with_decoder(ArithmeticDecoder::new(source));
        record_decompressor.set_fields_from(&vlr.items);
        Self {
            vlr,
            record_decompressor,
            chunk_points_read: 0,
        }
    }

    pub fn decompress_one(&mut self, mut out: &mut [u8]) -> std::io::Result<()> {
        if self.chunk_points_read == self.vlr.chunk_size {
            self.record_decompressor.reset();
            self.record_decompressor.set_fields_from(&self.vlr.items);
            self.chunk_points_read = 0;
        }

        self.record_decompressor.decompress(&mut out)?;
        self.chunk_points_read += 1;
        Ok(())
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
    chunk_sizes: Vec<usize>,
    last_chunk_pos: u64,
}

// TODO impl for W: Write + Seek update chunktale offset ?
// TODO chunkTable
impl<W: Write + Seek> LasZipCompressor<W> {
    pub fn from_laz_items(output: W, items: Vec<LazItem>) -> Self {
        let mut record_compressor = RecordCompressor::with_encoder(ArithmeticEncoder::new(output));
        record_compressor.set_fields_from(&items);
        let vlr = LazVlr::from_laz_items(items);

        Self {
            vlr,
            record_compressor,
            first_point: true,
            chunk_point_written: 0,
            chunk_sizes: Vec::new(),
            last_chunk_pos: 0,
        }
    }

    pub fn compress_one(&mut self, input: &[u8]) -> std::io::Result<()> {
        if self.first_point {
            //TODO borrow stream and write emtpy chunk offset size
            self.last_chunk_pos = self
                .record_compressor
                .borrow_mut_stream()
                .seek(SeekFrom::Current(0))?;
            self.first_point = false;
        }

        if self.chunk_point_written == self.vlr.chunk_size {
            self.record_compressor.done()?;
            self.record_compressor.reset();
            self.record_compressor.set_fields_from(&self.vlr.items);
            self.chunk_point_written = 0;
            self.update_chunk_table()?;
        }

        self.record_compressor.compress(&input)?;
        self.chunk_point_written += 1;
        Ok(())
    }

    pub fn done(&mut self) -> std::io::Result<()> {
        self.record_compressor.done()?;
        self.update_chunk_table()?;
        self.write_chunk_table()
    }

    pub fn vlr(&self) -> &LazVlr {
        &self.vlr
    }

    pub fn into_stream(self) -> W {
        self.record_compressor.into_stream()
    }

    fn write_chunk_table(&mut self) -> std::io::Result<()> {
        // Write header
        let mut stream = self.record_compressor.borrow_mut_stream();
        stream.write_u32::<LittleEndian>(0)?;
        stream.write_u32::<LittleEndian>(self.chunk_sizes.len() as u32)?;

        let mut encoder = ArithmeticEncoder::new(&mut stream);
        let mut compressor = IntegerCompressorBuilder::new()
            .bits(32)
            .contexts(2)
            .build_initialized();

        let mut predictor = 0;
        for chunk_size in &self.chunk_sizes {
            compressor.compress(&mut encoder, predictor, (*chunk_size) as i32, 1)?;
            predictor = (*chunk_size) as i32;
        }
        encoder.done()?;

        Ok(())
    }

    fn update_chunk_table(&mut self) -> std::io::Result<()> {
        let current_pos = self
            .record_compressor
            .borrow_mut_stream()
            .seek(SeekFrom::Current(0))?;
        self.chunk_sizes
            .push((current_pos - self.last_chunk_pos) as usize);
        self.last_chunk_pos = current_pos;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;

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
