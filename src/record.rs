use std::io::{Read, Seek, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use num_traits::{AsPrimitive, PrimInt, zero, Zero};

use crate::compressors;
use crate::decoders;
use crate::decompressors;
use crate::encoders;
use crate::las;
use crate::las::laszip::{LasZipError, LazItem, LazItemType};
use crate::packers::Packable;

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
pub struct SequentialPointRecordDecompressor<R: Read> {
    field_decompressors: Vec<Box<dyn FieldDecompressor<R>>>,
    decoder: decoders::ArithmeticDecoder<R>,
    is_first_decompression: bool,
    record_size: usize,
}

impl<R: Read> SequentialPointRecordDecompressor<R> {
    pub fn new(input: R) -> Self {
        Self {
            field_decompressors: vec![],
            decoder: decoders::ArithmeticDecoder::new(input),
            is_first_decompression: true,
            record_size: 0,
        }
    }

    pub fn add_field_decompressor<T: 'static + FieldDecompressor<R>>(&mut self, field: T) {
        self.record_size += field.size_of_field();
        self.field_decompressors.push(Box::new(field));
    }

    pub fn add_boxed_decompressor(&mut self, d: Box<dyn FieldDecompressor<R>>) {
        self.record_size += d.size_of_field();
        self.field_decompressors.push(d);
    }

    pub fn record_size(&self) -> usize {
        self.record_size
    }

    pub fn decompress(&mut self, out: &mut [u8]) -> std::io::Result<()> {
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

    pub fn borrow_mut_stream(&mut self) -> &mut R {
        self.decoder.in_stream()
    }

    pub fn into_stream(self) -> R {
        self.decoder.into_stream()
    }

    pub fn reset(&mut self) {
        self.decoder.reset();
        self.is_first_decompression = true;
        self.field_decompressors.clear();
        self.record_size = 0;
    }
}

impl<R: Read> RecordDecompressor<R> for SequentialPointRecordDecompressor<R> {
    fn set_fields_from(&mut self, laz_items: &Vec<LazItem>) -> Result<(), LasZipError> {
        for record_item in laz_items {
            match record_item.version {
                1 => match record_item.item_type {
                    LazItemType::Byte(_) => self.add_field_decompressor(
                        las::v1::LasExtraByteDecompressor::new(record_item.size as usize),
                    ),
                    LazItemType::Point10 => {
                        self.add_field_decompressor(las::v1::LasPoint0Decompressor::new())
                    }
                    LazItemType::GpsTime => {
                        self.add_field_decompressor(las::v1::LasGpsTimeDecompressor::new())
                    }
                    LazItemType::RGB12 => {
                        self.add_field_decompressor(las::v1::LasRGBDecompressor::new())
                    }
                    _ => {
                        return Err(LasZipError::UnsupportedLazItemVersion(
                            record_item.item_type,
                            record_item.version,
                        ));
                    }
                },
                2 => match record_item.item_type {
                    LazItemType::Byte(_) => self.add_field_decompressor(
                        las::v2::LasExtraByteDecompressor::new(record_item.size as usize),
                    ),
                    LazItemType::Point10 => {
                        self.add_field_decompressor(las::v2::LasPoint0Decompressor::new())
                    }
                    LazItemType::GpsTime => {
                        self.add_field_decompressor(las::v2::GpsTimeDecompressor::new())
                    }
                    LazItemType::RGB12 => {
                        self.add_field_decompressor(las::v2::LasRGBDecompressor::new())
                    }
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
        self.record_size()
    }

    fn decompress_next(&mut self, out: &mut [u8]) -> std::io::Result<()> {
        self.decompress(out)
    }

    fn reset(&mut self) {
        self.reset();
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
pub struct LayeredPointRecordDecompressor<R: Read + Seek> {
    field_decompressors: Vec<Box<dyn LayeredFieldDecompressor<R>>>,
    input: R,
    is_first_decompression: bool,
    record_size: usize,
    context: usize,
}

impl<R: Read + Seek> LayeredPointRecordDecompressor<R> {
    pub fn new(input: R) -> Self {
        Self {
            field_decompressors: vec![],
            input,
            is_first_decompression: true,
            record_size: 0,
            context: 0,
        }
    }
    pub fn add_field_decompressor<T: 'static + LayeredFieldDecompressor<R>>(
        &mut self,
        field: T,
    ) {
        self.record_size += field.size_of_field();
        self.field_decompressors.push(Box::new(field));
    }

    pub fn record_size(&self) -> usize {
        self.record_size
    }

    pub fn decompress(&mut self, out: &mut [u8]) -> std::io::Result<()> {
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
            for field in &mut self.field_decompressors {
                let field_end = field_start + field.size_of_field();
                field.decompress_field_with(&mut out[field_start..field_end], &mut self.context)?;
                field_start = field_end;
            }
        }
        Ok(())
    }
}

impl<R: Read + Seek> RecordDecompressor<R> for LayeredPointRecordDecompressor<R> {
    fn set_fields_from(&mut self, laz_items: &Vec<LazItem>) -> Result<(), LasZipError> {
        for record_item in laz_items {
            match record_item.version {
                3 => match record_item.item_type {
                    LazItemType::Point14 => {
                        self.add_field_decompressor(las::v3::LasPoint6Decompressor::new())
                    }
                    LazItemType::RGB14 => {
                        self.add_field_decompressor(las::v3::LasRGBDecompressor::new())
                    }
                    LazItemType::RGBNIR14 => {
                        self.add_field_decompressor(las::v3::LasRGBDecompressor::new());
                        self.add_field_decompressor(las::v3::LasNIRDecompressor::new());
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
        self.decompress(out)
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
    fn init_first_point(&mut self, dst: &mut W, first_point: &[u8], context: &mut usize) -> std::io::Result<()>;

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
pub struct SequentialPointRecordCompressor<W: Write> {
    is_first_compression: bool,
    field_compressors: Vec<Box<dyn FieldCompressor<W>>>,
    encoder: encoders::ArithmeticEncoder<W>,
    record_size: usize,
}

impl<W: Write> SequentialPointRecordCompressor<W> {
    pub fn new(output: W) -> Self {
        Self {
            is_first_compression: true,
            field_compressors: vec![],
            encoder: encoders::ArithmeticEncoder::new(output),
            record_size: 0,
        }
    }

    pub fn add_field_compressor<T: 'static + FieldCompressor<W>>(&mut self, field: T) {
        self.record_size += field.size_of_field();
        self.field_compressors.push(Box::new(field));
    }

    pub fn add_boxed_compressor(&mut self, c: Box<dyn FieldCompressor<W>>) {
        self.record_size += c.size_of_field();
        self.field_compressors.push(c);
    }
}

impl<W: Write> RecordCompressor<W> for SequentialPointRecordCompressor<W> {
    fn set_fields_from(&mut self, laz_items: &Vec<LazItem>) -> Result<(), LasZipError> {
        for record_item in laz_items {
            match record_item.version {
                1 => match record_item.item_type {
                    LazItemType::Point10 => {
                        self.add_field_compressor(las::v1::LasPoint0Compressor::new())
                    }
                    LazItemType::GpsTime => {
                        self.add_field_compressor(las::v1::LasGpsTimeCompressor::new())
                    }
                    LazItemType::RGB12 => {
                        self.add_field_compressor(las::v1::LasRGBCompressor::new())
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
                        self.add_field_compressor(las::v2::LasPoint0Compressor::new())
                    }
                    LazItemType::GpsTime => {
                        self.add_field_compressor(las::v2::GpsTimeCompressor::new())
                    }
                    LazItemType::RGB12 => {
                        self.add_field_compressor(las::v2::LasRGBCompressor::new())
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


pub struct LayeredPointRecordCompressor<W: Write> {
    field_compressors: Vec<Box<dyn LayeredFieldCompressor<W>>>,
    point_size: usize,
    point_count: u32,
    dst: W,
}

impl<W: Write> LayeredPointRecordCompressor<W> {
    pub fn new(dst: W) -> Self {
        Self {
            field_compressors: vec![],
            point_size: 0, //TODO
            point_count: 0,
            dst,
        }
    }

    pub fn add_field_compressor<T: 'static + LayeredFieldCompressor<W>>(&mut self, field: T) {
        self.field_compressors.push(Box::new(field));
    }
}

impl<W: Write> RecordCompressor<W> for LayeredPointRecordCompressor<W> {
    fn set_fields_from(&mut self, laz_items: &Vec<LazItem>) -> Result<(), LasZipError> {
        for item in laz_items {
            match item.version {
                3 =>
                    match item.item_type {
                        LazItemType::Point14 => {
                            self.add_field_compressor(las::v3::LasPoint6Compressor::default())
                        }
                        LazItemType::RGB14 => {
                            self.add_field_compressor(las::v3::LasRGBCompressor::new())
                        }
                        LazItemType::RGBNIR14 => {
                            self.add_field_compressor(las::v3::LasRGBCompressor::new());
                            self.add_field_compressor(las::v3::LasNIRCompressor::new());
                        }
                        //TODO Extrabyte Compressor
                        _ => {
                            return Err(LasZipError::UnsupportedLazItemVersion(
                                item.item_type,
                                item.version,
                            ));
                        }
                    }
                ,
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
                compressor.init_first_point(&mut self.dst, &point[field_start..field_end], &mut context)?;
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
        //TODO call reset our done on all compressors
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


/***************************************************************************************************
                    Something else
***************************************************************************************************/

struct StandardDiffMethod<T: Zero + Copy> {
    have_value: bool,
    value: T,
}

impl<T: Zero + Copy> StandardDiffMethod<T> {
    pub fn new() -> Self {
        Self {
            have_value: false,
            value: zero(),
        }
    }
    pub fn value(&self) -> T {
        self.value
    }

    #[allow(dead_code)]
    pub fn have_value(&self) -> bool {
        self.have_value
    }

    pub fn push(&mut self, value: T) {
        if !self.have_value {
            self.have_value = true;
        }
        self.value = value;
    }
}

pub struct IntegerFieldDecompressor<IntType: Zero + Copy + PrimInt> {
    decompressor: decompressors::IntegerDecompressor,
    diff_method: StandardDiffMethod<IntType>,
}

impl<IntType: Zero + Copy + PrimInt> IntegerFieldDecompressor<IntType> {
    pub fn new() -> Self {
        Self {
            decompressor: decompressors::IntegerDecompressorBuilder::new()
                .bits(std::mem::size_of::<IntType>() as u32 * 8)
                .build(),
            diff_method: StandardDiffMethod::<IntType>::new(),
        }
    }
}

impl<IntType, R> FieldDecompressor<R> for IntegerFieldDecompressor<IntType>
    where
        i32: num_traits::cast::AsPrimitive<IntType>,
        IntType: Zero
        + Copy
        + PrimInt
        + Packable
        + AsPrimitive<i32>
        + AsPrimitive<<IntType as Packable>::Type>,
        <IntType as Packable>::Type: AsPrimitive<IntType>,
        R: Read,
{
    fn size_of_field(&self) -> usize {
        std::mem::size_of::<IntType>()
    }

    fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()> {
        self.decompressor.init();
        src.read_exact(first_point)?;
        let r = IntType::unpack_from(&first_point).as_();
        self.diff_method.push(r);
        Ok(())
    }

    fn decompress_with(
        &mut self,
        mut decoder: &mut decoders::ArithmeticDecoder<R>,
        mut buf: &mut [u8],
    ) -> std::io::Result<()>
        where
            Self: Sized,
    {
        self.decompressor.init();
        let v: IntType = self
            .decompressor
            .decompress(&mut decoder, self.diff_method.value().as_(), 0)?
            .as_(); // i32 -> IntType
        v.pack_into(&mut buf);

        self.diff_method.push(v);
        Ok(())
    }
}

pub struct IntegerFieldCompressor<IntType: Zero + Copy + PrimInt> {
    compressor: compressors::IntegerCompressor,
    diff_method: StandardDiffMethod<IntType>,
}

impl<IntType: Zero + Copy + PrimInt> IntegerFieldCompressor<IntType> {
    pub fn new() -> Self {
        Self {
            compressor: compressors::IntegerCompressor::new(
                std::mem::size_of::<IntType>() as u32 * 8,
                1,
                8,
                0,
            ),
            diff_method: StandardDiffMethod::<IntType>::new(),
        }
    }
}

impl<IntType, W> FieldCompressor<W> for IntegerFieldCompressor<IntType>
    where
        IntType: Zero + Copy + PrimInt + Packable + 'static + AsPrimitive<i32>,
        <IntType as Packable>::Type: AsPrimitive<IntType>,
        W: Write,
{
    fn size_of_field(&self) -> usize {
        std::mem::size_of::<IntType>()
    }

    fn compress_first(&mut self, dst: &mut W, buf: &[u8]) -> std::io::Result<()> {
        self.compressor.init();
        let this_val: IntType = IntType::unpack_from(&buf).as_();
        self.diff_method.push(this_val);
        dst.write_all(&buf)
    }

    fn compress_with(
        &mut self,
        mut encoder: &mut encoders::ArithmeticEncoder<W>,
        buf: &[u8],
    ) -> std::io::Result<()> {
        let this_val: IntType = IntType::unpack_from(&buf).as_();
        // Strange that wi init each time but this is as in laz-perf code
        self.compressor.init();

        // Let the differ decide what values we're going to push
        self.compressor.compress(
            &mut encoder,
            self.diff_method.value().as_(),
            this_val.as_(),
            0,
        )?;
        self.diff_method.push(this_val);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::io::{Seek, SeekFrom};

    use super::*;

    #[test]
    fn dyna() {
        let stream = std::io::Cursor::new(Vec::<u8>::new());

        let mut compressor = SequentialPointRecordCompressor::new(stream);
        compressor.done().unwrap();

        let stream = compressor.into_stream();
        let data = stream.into_inner();

        assert_eq!(&data, &[1u8, 0u8, 0u8, 0u8]);
    }

    #[test]
    fn dyna2() {
        let stream = std::io::Cursor::new(Vec::<u8>::new());

        let mut compressor = SequentialPointRecordCompressor::new(stream);
        compressor.add_field_compressor(IntegerFieldCompressor::<i32>::new());
        compressor.compress_next(&[0u8, 0u8, 0u8, 0u8]).unwrap();
        compressor.done().unwrap();

        let stream = compressor.into_stream();
        let data = stream.into_inner();

        assert_eq!(&data, &[0u8, 0u8, 0u8, 0u8, 1u8, 0u8, 0u8, 0u8]);
    }

    #[test]
    fn dyna3() {
        let stream = std::io::Cursor::new(Vec::<u8>::new());

        let mut compressor = SequentialPointRecordCompressor::new(stream);
        compressor.add_field_compressor(IntegerFieldCompressor::<i32>::new());
        compressor.compress_next(&[17u8, 42u8, 35u8, 1u8]).unwrap();
        compressor.done().unwrap();

        let mut stream = compressor.into_stream();
        stream.seek(SeekFrom::Start(0)).unwrap();

        let mut read_from_stream = [0u8; 8];
        stream.read_exact(&mut read_from_stream).unwrap();
        assert_eq!(
            &read_from_stream,
            &[17u8, 42u8, 35u8, 1u8, 1u8, 0u8, 0u8, 0u8]
        );

        let data = stream.into_inner();
        assert_eq!(&data, &[17u8, 42u8, 35u8, 1u8, 1u8, 0u8, 0u8, 0u8]);
    }

    #[test]
    #[should_panic]
    fn test_small_input_buffer() {
        let stream = std::io::Cursor::new(Vec::<u8>::new());

        let mut compressor = SequentialPointRecordCompressor::new(stream);
        compressor.add_field_compressor(IntegerFieldCompressor::<i32>::new());
        compressor.compress_next(&[]).unwrap();
        compressor.done().unwrap();
    }
}
