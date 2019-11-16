use std::io::{Read, Seek, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::decoders;
use crate::encoders;
use crate::las;
use crate::las::laszip::{LasZipError, LazItem, LazItemType};

/***************************************************************************************************
                    Decompression Related Traits
***************************************************************************************************/

pub trait FieldDecompressor<R: Read> {
    fn size_of_field(&self) -> usize;

    fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()>;

    fn decompress_with(
        &mut self,
        decoder: &mut decoders::ArithmeticDecoder<R>,
        buf: &mut [u8],
    ) -> std::io::Result<()>;
}

pub trait LayeredFieldDecompressor<R: Read> {
    fn size_of_field(&self) -> usize;

    fn init_first_point(
        &mut self,
        src: &mut R,
        first_point: &mut [u8],
        context: &mut usize,
    ) -> std::io::Result<()>;

    fn decompress_field_with(
        &mut self,
        current_point: &mut [u8],
        context: &mut usize,
    ) -> std::io::Result<()>;

    fn read_layers_sizes(&mut self, src: &mut R) -> std::io::Result<()>;
    fn read_layers(&mut self, src: &mut R) -> std::io::Result<()>;
}

pub trait RecordDecompressor<R> {
    fn set_fields_from(&mut self, laz_items: &Vec<LazItem>) -> Result<(), LasZipError>;
    fn record_size(&self) -> usize;

    fn decompress_next(&mut self, out: &mut [u8]) -> std::io::Result<()>;
    fn reset(&mut self);

    fn borrow_stream_mut(&mut self) -> &mut R;

    fn into_stream(self) -> R;
    fn box_into_stream(self: Box<Self>) -> R;
}

/***************************************************************************************************
                    Record Decompressors implementations
***************************************************************************************************/

/// PointRecordDecompressor decompresses data using PointFieldDecompressor.
/// The Points data is organized as follow;
///
/// 1) 1 Raw Point (as per ASPRS LAS definition)
/// 2) n compressed Points
pub struct SequentialPointRecordDecompressor<'a, R: Read> {
    field_decompressors: Vec<Box<dyn FieldDecompressor<R> + 'a>>,
    decoder: decoders::ArithmeticDecoder<R>,
    is_first_decompression: bool,
    record_size: usize,
}

impl<'a, R: Read> SequentialPointRecordDecompressor<'a, R> {
    pub fn new(input: R) -> Self {
        Self {
            field_decompressors: vec![],
            decoder: decoders::ArithmeticDecoder::new(input),
            is_first_decompression: true,
            record_size: 0,
        }
    }

    pub fn add_field_decompressor<T: FieldDecompressor<R> + 'a>(&mut self, field: T) {
        self.record_size += field.size_of_field();
        self.field_decompressors.push(Box::new(field));
    }
    pub fn add_boxed_decompressor(&mut self, d: Box<dyn FieldDecompressor<R>>) {
        self.record_size += d.size_of_field();
        self.field_decompressors.push(d);
    }
}

impl<'a, R: Read> RecordDecompressor<R> for SequentialPointRecordDecompressor<'a, R> {
    fn set_fields_from(&mut self, laz_items: &Vec<LazItem>) -> Result<(), LasZipError> {
        for record_item in laz_items {
            match record_item.version {
                1 => match record_item.item_type {
                    LazItemType::Point10 => {
                        self.add_field_decompressor(las::v1::LasPoint0Decompressor::default())
                    }
                    LazItemType::GpsTime => {
                        self.add_field_decompressor(las::v1::LasGpsTimeDecompressor::default())
                    }
                    LazItemType::RGB12 => {
                        self.add_field_decompressor(las::v1::LasRGBDecompressor::default())
                    }
                    LazItemType::Byte(_) => self.add_field_decompressor(
                        las::v1::LasExtraByteDecompressor::new(record_item.size as usize),
                    ),
                    _ => {
                        return Err(LasZipError::UnsupportedLazItemVersion(
                            record_item.item_type,
                            record_item.version,
                        ));
                    }
                },
                2 => match record_item.item_type {
                    LazItemType::Point10 => {
                        self.add_field_decompressor(las::v2::LasPoint0Decompressor::default())
                    }
                    LazItemType::GpsTime => {
                        self.add_field_decompressor(las::v2::GpsTimeDecompressor::default())
                    }
                    LazItemType::RGB12 => {
                        self.add_field_decompressor(las::v2::LasRGBDecompressor::default())
                    }
                    LazItemType::Byte(_) => self.add_field_decompressor(
                        las::v2::LasExtraByteDecompressor::new(record_item.size as usize),
                    ),
                    _ => {
                        return Err(LasZipError::UnsupportedLazItemVersion(
                            record_item.item_type,
                            record_item.version,
                        ));
                    }
                },
                _ => {
                    return Err(LasZipError::UnsupportedLazItemVersion(
                        record_item.item_type,
                        record_item.version,
                    ));
                }
            }
        }
        Ok(())
    }

    fn record_size(&self) -> usize {
        self.record_size
    }

    fn decompress_next(&mut self, out: &mut [u8]) -> std::io::Result<()> {
        if self.is_first_decompression {
            let mut field_start = 0;
            for field in &mut self.field_decompressors {
                let field_end = field_start + field.size_of_field();
                field.decompress_first(
                    &mut self.decoder.in_stream(),
                    &mut out[field_start..field_end],
                )?;
                field_start = field_end;
            }

            self.is_first_decompression = false;

            // the decoder needs to be told that it should read the
            // init bytes after the first record has been read
            self.decoder.read_init_bytes()?;
        } else {
            let mut field_start = 0;
            for field in &mut self.field_decompressors {
                let field_end = field_start + field.size_of_field();
                field.decompress_with(&mut self.decoder, &mut out[field_start..field_end])?;
                field_start = field_end;
            }
        }
        Ok(())
    }

    fn reset(&mut self) {
        self.decoder.reset();
        self.is_first_decompression = true;
        self.field_decompressors.clear();
        self.record_size = 0;
    }

    fn borrow_stream_mut(&mut self) -> &mut R {
        self.decoder.in_stream()
    }

    fn into_stream(self) -> R {
        self.decoder.into_stream()
    }

    fn box_into_stream(self: Box<Self>) -> R {
        self.decoder.into_stream()
    }
}

/// LayeredPointRecordDecompressor decompresses data using LayeredPointFieldDecompressor.
/// The Points data is organized in layer as follow:
///
/// 1) 1 Raw Point (as per ASPRS LAS definition)
/// 2) Number of remaining points in the chunk
/// 3) Number of bytes for each layer of the chunk
/// 4) Data of the layers
pub struct LayeredPointRecordDecompressor<'a, R: Read + Seek> {
    field_decompressors: Vec<Box<dyn LayeredFieldDecompressor<R> + 'a>>,
    input: R,
    is_first_decompression: bool,
    record_size: usize,
    context: usize,
}

impl<'a, R: Read + Seek> LayeredPointRecordDecompressor<'a, R> {
    pub fn new(input: R) -> Self {
        Self {
            field_decompressors: vec![],
            input,
            is_first_decompression: true,
            record_size: 0,
            context: 0,
        }
    }

    pub fn add_field_decompressor<T: 'static + LayeredFieldDecompressor<R>>(&mut self, field: T) {
        self.record_size += field.size_of_field();
        self.field_decompressors.push(Box::new(field));
    }
}

impl<'a, R: Read + Seek> RecordDecompressor<R> for LayeredPointRecordDecompressor<'a, R> {
    fn set_fields_from(&mut self, laz_items: &Vec<LazItem>) -> Result<(), LasZipError> {
        for record_item in laz_items {
            match record_item.version {
                3 => match record_item.item_type {
                    LazItemType::Point14 => {
                        self.add_field_decompressor(las::v3::LasPoint6Decompressor::default())
                    }
                    LazItemType::RGB14 => {
                        self.add_field_decompressor(las::v3::LasRGBDecompressor::default())
                    }
                    LazItemType::RGBNIR14 => {
                        self.add_field_decompressor(las::v3::LasRGBDecompressor::default());
                        self.add_field_decompressor(las::v3::LasNIRDecompressor::default());
                    }
                    LazItemType::Byte14(count) => self.add_field_decompressor(
                        las::v3::LasExtraByteDecompressor::new(count as usize),
                    ),
                    _ => {
                        return Err(LasZipError::UnsupportedLazItemVersion(
                            record_item.item_type,
                            record_item.version,
                        ));
                    }
                },
                _ => {
                    return Err(LasZipError::UnsupportedLazItemVersion(
                        record_item.item_type,
                        record_item.version,
                    ));
                }
            }
        }
        Ok(())
    }

    fn record_size(&self) -> usize {
        self.field_decompressors
            .iter()
            .map(|decompressor| decompressor.size_of_field())
            .sum()
    }

    fn decompress_next(&mut self, out: &mut [u8]) -> std::io::Result<()> {
        if self.is_first_decompression {
            let mut field_start = 0;
            for field in &mut self.field_decompressors {
                let field_end = field_start + field.size_of_field();
                field.init_first_point(
                    &mut self.input,
                    &mut out[field_start..field_end],
                    &mut self.context,
                )?;
                field_start = field_end;
            }

            let _count = self.input.read_u32::<LittleEndian>()?;
            for field in &mut self.field_decompressors {
                field.read_layers_sizes(&mut self.input)?;
            }
            for field in &mut self.field_decompressors {
                field.read_layers(&mut self.input)?;
            }
            self.is_first_decompression = false;
        } else {
            let mut field_start = 0;
            self.context = 0;
            for field in &mut self.field_decompressors {
                let field_end = field_start + field.size_of_field();
                field.decompress_field_with(&mut out[field_start..field_end], &mut self.context)?;
                field_start = field_end;
            }
        }
        Ok(())
    }

    fn reset(&mut self) {
        self.is_first_decompression = true;
        self.field_decompressors.clear();
    }

    fn borrow_stream_mut(&mut self) -> &mut R {
        &mut self.input
    }

    fn into_stream(self) -> R {
        self.input
    }

    fn box_into_stream(self: Box<Self>) -> R {
        self.input
    }
}

/***************************************************************************************************
                    Compression related Traits
***************************************************************************************************/
pub trait FieldCompressor<W: Write> {
    fn size_of_field(&self) -> usize;

    fn compress_first(&mut self, dst: &mut W, buf: &[u8]) -> std::io::Result<()>;

    fn compress_with(
        &mut self,
        encoder: &mut encoders::ArithmeticEncoder<W>,
        buf: &[u8],
    ) -> std::io::Result<()>;
}

pub trait LayeredFieldCompressor<W: Write> {
    fn size_of_field(&self) -> usize;
    fn init_first_point(
        &mut self,
        dst: &mut W,
        first_point: &[u8],
        context: &mut usize,
    ) -> std::io::Result<()>;

    fn compress_field_with(
        &mut self,
        current_point: &[u8],
        context: &mut usize,
    ) -> std::io::Result<()>;

    fn write_layers_sizes(&mut self, dst: &mut W) -> std::io::Result<()>;
    fn write_layers(&mut self, dst: &mut W) -> std::io::Result<()>;
}

pub trait RecordCompressor<W> {
    fn set_fields_from(&mut self, laz_items: &Vec<LazItem>) -> Result<(), LasZipError>;
    fn record_size(&self) -> usize;

    fn compress_next(&mut self, input: &[u8]) -> std::io::Result<()>;
    fn done(&mut self) -> std::io::Result<()>;
    fn reset(&mut self);

    fn borrow_stream_mut(&mut self) -> &mut W;

    fn into_stream(self) -> W;
    fn box_into_stream(self: Box<Self>) -> W;
}

/***************************************************************************************************
                    Record Compressors implementations
***************************************************************************************************/
pub struct SequentialPointRecordCompressor<'a, W: Write> {
    is_first_compression: bool,
    field_compressors: Vec<Box<dyn FieldCompressor<W> + 'a>>,
    encoder: encoders::ArithmeticEncoder<W>,
    record_size: usize,
}

impl<'a, W: Write> SequentialPointRecordCompressor<'a, W> {
    pub fn new(output: W) -> Self {
        Self {
            is_first_compression: true,
            field_compressors: vec![],
            encoder: encoders::ArithmeticEncoder::new(output),
            record_size: 0,
        }
    }

    pub fn add_field_compressor<T: FieldCompressor<W> + 'a>(&mut self, field: T) {
        self.record_size += field.size_of_field();
        self.field_compressors.push(Box::new(field));
    }

    pub fn add_boxed_compressor(&mut self, c: Box<dyn FieldCompressor<W>>) {
        self.record_size += c.size_of_field();
        self.field_compressors.push(c);
    }
}

impl<'a, W: Write> RecordCompressor<W> for SequentialPointRecordCompressor<'a, W> {
    fn set_fields_from(&mut self, laz_items: &Vec<LazItem>) -> Result<(), LasZipError> {
        for record_item in laz_items {
            match record_item.version {
                1 => match record_item.item_type {
                    LazItemType::Point10 => {
                        self.add_field_compressor(las::v1::LasPoint0Compressor::default())
                    }
                    LazItemType::GpsTime => {
                        self.add_field_compressor(las::v1::LasGpsTimeCompressor::default())
                    }
                    LazItemType::RGB12 => {
                        self.add_field_compressor(las::v1::LasRGBCompressor::default())
                    }
                    LazItemType::Byte(_) => self.add_field_compressor(
                        las::v1::LasExtraByteCompressor::new(record_item.size as usize),
                    ),
                    _ => {
                        return Err(LasZipError::UnsupportedLazItemVersion(
                            record_item.item_type,
                            record_item.version,
                        ));
                    }
                },
                2 => match record_item.item_type {
                    LazItemType::Point10 => {
                        self.add_field_compressor(las::v2::LasPoint0Compressor::default())
                    }
                    LazItemType::GpsTime => {
                        self.add_field_compressor(las::v2::GpsTimeCompressor::default())
                    }
                    LazItemType::RGB12 => {
                        self.add_field_compressor(las::v2::LasRGBCompressor::default())
                    }
                    LazItemType::Byte(_) => self.add_field_compressor(
                        las::v2::LasExtraByteCompressor::new(record_item.size as usize),
                    ),
                    _ => {
                        return Err(LasZipError::UnsupportedLazItemVersion(
                            record_item.item_type,
                            record_item.version,
                        ));
                    }
                },
                _ => {
                    return Err(LasZipError::UnsupportedLazItemVersion(
                        record_item.item_type,
                        record_item.version,
                    ));
                }
            }
        }
        Ok(())
    }

    fn record_size(&self) -> usize {
        self.record_size
    }

    fn compress_next(&mut self, input: &[u8]) -> std::io::Result<()> {
        if self.is_first_compression {
            let mut field_start = 0;
            for field in &mut self.field_compressors {
                let field_end = field_start + field.size_of_field();
                field.compress_first(self.encoder.out_stream(), &input[field_start..field_end])?;
                field_start = field_end;
            }
            self.is_first_compression = false;
        } else {
            let mut field_start = 0;
            for field in &mut self.field_compressors {
                let field_end = field_start + field.size_of_field();
                field.compress_with(&mut self.encoder, &input[field_start..field_end])?;
                field_start = field_end;
            }
        }
        Ok(())
    }

    fn done(&mut self) -> std::io::Result<()> {
        self.encoder.done()
    }

    fn reset(&mut self) {
        self.is_first_compression = true;
        self.encoder.reset();
        self.field_compressors.clear();
        self.record_size = 0;
    }

    fn borrow_stream_mut(&mut self) -> &mut W {
        self.encoder.out_stream()
    }

    fn into_stream(self) -> W {
        self.encoder.into_stream()
    }

    fn box_into_stream(self: Box<Self>) -> W {
        self.encoder.into_stream()
    }
}

pub struct LayeredPointRecordCompressor<'a, W: Write> {
    field_compressors: Vec<Box<dyn LayeredFieldCompressor<W> + 'a>>,
    point_size: usize,
    point_count: u32,
    dst: W,
}

impl<'a, W: Write> LayeredPointRecordCompressor<'a, W> {
    pub fn new(dst: W) -> Self {
        Self {
            field_compressors: vec![],
            point_size: 0,
            point_count: 0,
            dst,
        }
    }

    pub fn add_field_compressor<T: LayeredFieldCompressor<W> + 'a>(&mut self, field: T) {
        self.point_size += field.size_of_field();
        self.field_compressors.push(Box::new(field));
    }
}

impl<'a, W: Write> RecordCompressor<W> for LayeredPointRecordCompressor<'a, W> {
    fn set_fields_from(&mut self, laz_items: &Vec<LazItem>) -> Result<(), LasZipError> {
        for item in laz_items {
            match item.version {
                3 => match item.item_type {
                    LazItemType::Point14 => {
                        self.add_field_compressor(las::v3::LasPoint6Compressor::default())
                    }
                    LazItemType::RGB14 => {
                        self.add_field_compressor(las::v3::LasRGBCompressor::default())
                    }
                    LazItemType::RGBNIR14 => {
                        self.add_field_compressor(las::v3::LasRGBCompressor::default());
                        self.add_field_compressor(las::v3::LasNIRCompressor::default());
                    }
                   LazItemType::Byte14(n) => {
                       self.add_field_compressor(las::v3::LasExtraByteCompressor::new(n as usize));
                   }
                    _ => {
                        return Err(LasZipError::UnsupportedLazItemVersion(
                            item.item_type,
                            item.version,
                        ));
                    }
                },
                _ => {
                    return Err(LasZipError::UnsupportedLazItemVersion(
                        item.item_type,
                        item.version,
                    ));
                }
            }
        }
        Ok(())
    }

    fn record_size(&self) -> usize {
        self.point_size
    }

    fn compress_next(&mut self, point: &[u8]) -> std::io::Result<()> {
        let mut context = 0usize;
        if self.point_count == 0 {
            let mut field_start = 0;
            for compressor in &mut self.field_compressors {
                let field_end = field_start + compressor.size_of_field();
                compressor.init_first_point(
                    &mut self.dst,
                    &point[field_start..field_end],
                    &mut context,
                )?;
                field_start = field_end;
            }
        } else {
            let mut field_start = 0;
            for compressor in &mut self.field_compressors {
                let field_end = field_start + compressor.size_of_field();
                compressor.compress_field_with(&point[field_start..field_end], &mut context)?;
                field_start = field_end;
            }
        }
        self.point_count += 1;
        Ok(())
    }

    fn done(&mut self) -> std::io::Result<()> {
        if self.point_count > 0 {
            self.dst.write_u32::<LittleEndian>(self.point_count)?;
            for compressor in &mut self.field_compressors {
                compressor.write_layers_sizes(&mut self.dst)?;
            }
            for compressor in &mut self.field_compressors {
                compressor.write_layers(&mut self.dst)?;
            }
        }
        Ok(())
    }

    fn reset(&mut self) {
        self.point_count = 0;
        self.point_size = 0;
        self.field_compressors.clear();
    }

    fn borrow_stream_mut(&mut self) -> &mut W {
        &mut self.dst
    }

    fn into_stream(self) -> W {
        self.dst
    }

    fn box_into_stream(self: Box<Self>) -> W {
        self.dst
    }
}
