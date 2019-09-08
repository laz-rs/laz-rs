use std::io::{Read, Seek, Write};

use num_traits::{zero, AsPrimitive, PrimInt, Zero};

use crate::compressors;
use crate::decoders;
use crate::decompressors;
use crate::encoders;
use crate::encoders::ArithmeticEncoder;
use crate::las;
use crate::las::laszip::{LasZipError, LazItem, LazItemType};
use crate::packers::Packable;

use byteorder::{LittleEndian, ReadBytesExt};

pub trait PointFieldDecompressor<R: Read, P> {
    fn init_first_point(&mut self, src: &mut R, first_point: &mut P) -> std::io::Result<()>;

    fn decompress_field_with(
        &mut self,
        decoder: &mut decoders::ArithmeticDecoder<R>,
        current_point: &mut P,
    ) -> std::io::Result<()>;
}

pub trait LayeredPointFieldDecompressor<R: Read, P> {
    fn init_first_point(
        &mut self,
        src: &mut R,
        first_point: &mut P,
        context: &mut usize,
    ) -> std::io::Result<()>;

    fn decompress_field_with(
        &mut self,
        current_point: &mut P,
        context: &mut usize,
    ) -> std::io::Result<()>;

    fn read_layers_sizes(&mut self, src: &mut R) -> std::io::Result<()>;
    fn read_layers(&mut self, src: &mut R) -> std::io::Result<()>;
}

pub trait BufferFieldDecompressor<R: Read> {
    fn size_of_field(&self) -> usize;

    fn decompress_first(&mut self, src: &mut R, first_point: &mut [u8]) -> std::io::Result<()>;

    fn decompress_with(
        &mut self,
        decoder: &mut decoders::ArithmeticDecoder<R>,
        buf: &mut [u8],
    ) -> std::io::Result<()>;
}

pub trait BufferLayeredFieldDecompressor<R: Read> {
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
pub struct PointRecordDecompressor<R: Read, P: Default> {
    point_fields_decompressor: Vec<Box<dyn PointFieldDecompressor<R, P>>>,
    decoder: decoders::ArithmeticDecoder<R>,
    first_decompression: bool,
}

impl<R: Read, P: Default> PointRecordDecompressor<R, P> {
    pub fn new(input: R) -> Self {
        Self {
            point_fields_decompressor: vec![],
            decoder: decoders::ArithmeticDecoder::new(input),
            first_decompression: true,
        }
    }

    pub fn add_decompressor<D: PointFieldDecompressor<R, P> + 'static>(
        &mut self,
        field_decompressor: D,
    ) {
        self.point_fields_decompressor
            .push(Box::new(field_decompressor));
    }

    pub fn add_boxed_decompressor(&mut self, d: Box<dyn PointFieldDecompressor<R, P>>) {
        self.point_fields_decompressor.push(d);
    }

    pub fn decompress_next(&mut self) -> std::io::Result<P> {
        let mut current_point = P::default();
        if self.first_decompression {
            for field in &mut self.point_fields_decompressor {
                field.init_first_point(self.decoder.in_stream(), &mut current_point)?;
            }
            self.decoder.read_init_bytes()?;
            self.first_decompression = false;
        } else {
            for field in &mut self.point_fields_decompressor {
                field.decompress_field_with(&mut self.decoder, &mut current_point)?;
            }
        }
        Ok(current_point)
    }
}

/// LayeredPointRecordDecompressor decompresses data using LayeredPointFieldDecompressor.
/// The Points data is organized in layer as follow:
///
/// 1) 1 Raw Point (as per ASPRS LAS definition)
/// 2) Number of remaining points in the chunk
/// 3) Number of bytes for each layer of the chunk
/// 4) Data of the layers
pub struct LayeredPointRecordDecompressor<R: Read, P> {
    point_fields_decompressor: Vec<Box<dyn LayeredPointFieldDecompressor<R, P>>>,
    input: R,
    first_decompression: bool,
    point_count: u32,
    point_read: u32,
}

impl<R: Read, P: Default> LayeredPointRecordDecompressor<R, P> {
    pub fn new(input: R) -> Self {
        Self {
            point_fields_decompressor: vec![],
            input,
            first_decompression: true,
            point_count: 0,
            point_read: 0,
        }
    }

    pub fn add_decompressor<D: LayeredPointFieldDecompressor<R, P> + 'static>(
        &mut self,
        field_decompressor: D,
    ) {
        self.point_fields_decompressor
            .push(Box::new(field_decompressor));
    }

    pub fn add_boxed_decompressor(&mut self, d: Box<dyn LayeredPointFieldDecompressor<R, P>>) {
        self.point_fields_decompressor.push(d);
    }

    pub fn decompress_next(&mut self) -> std::io::Result<P> {
        let mut current_point = P::default();
        let mut context = 0usize;

        if self.first_decompression {
            for field in &mut self.point_fields_decompressor {
                field.init_first_point(&mut self.input, &mut current_point, &mut context)?;
            }

            self.point_count = self.input.read_u32::<LittleEndian>()?;
            for field_reader in &mut self.point_fields_decompressor {
                field_reader.read_layers_sizes(&mut self.input)?;
            }
            for field_reader in &mut self.point_fields_decompressor {
                field_reader.read_layers(&mut self.input)?;
            }
            self.first_decompression = false;
        } else {
            //TODO if point_read >= point_count what do we do ?
            for field in &mut self.point_fields_decompressor {
                field.decompress_field_with(&mut current_point, &mut context)?;
            }
            self.point_read += 1;
        }
        Ok(current_point)
    }

    pub fn into_inner(self) -> R {
        self.input
    }
}

pub struct BufferRecordDecompressor<R: Read> {
    field_decompressors: Vec<Box<dyn BufferFieldDecompressor<R>>>,
    decoder: decoders::ArithmeticDecoder<R>,
    is_first_decompression: bool,
    record_size: usize,
}

impl<R: Read> BufferRecordDecompressor<R> {
    pub fn new(input: R) -> Self {
        Self::with_decoder(decoders::ArithmeticDecoder::new(input))
    }

    pub fn with_decoder(decoder: decoders::ArithmeticDecoder<R>) -> Self {
        Self {
            field_decompressors: vec![],
            decoder,
            is_first_decompression: true,
            record_size: 0,
        }
    }

    pub fn add_field_decompressor<T: 'static + BufferFieldDecompressor<R>>(&mut self, field: T) {
        self.record_size += field.size_of_field();
        self.field_decompressors.push(Box::new(field));
    }

    pub fn add_boxed_decompressor(&mut self, d: Box<dyn BufferFieldDecompressor<R>>) {
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

impl<R: Read> RecordDecompressor<R> for BufferRecordDecompressor<R> {
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
                        ))
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
                        ))
                    }
                },
                _ => {
                    return Err(LasZipError::UnsupportedLazItemVersion(
                        record_item.item_type,
                        record_item.version,
                    ))
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

pub struct BufferLayeredRecordDecompressor<R: Read + Seek> {
    field_decompressors: Vec<Box<dyn BufferLayeredFieldDecompressor<R>>>,
    input: R,
    is_first_decompression: bool,
    record_size: usize,
    context: usize,
}

impl<R: Read + Seek> BufferLayeredRecordDecompressor<R> {
    pub fn new(input: R) -> Self {
        Self {
            field_decompressors: vec![],
            input,
            is_first_decompression: true,
            record_size: 0,
            context: 0,
        }
    }
    pub fn add_field_decompressor<T: 'static + BufferLayeredFieldDecompressor<R>>(
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

impl<R: Read + Seek> RecordDecompressor<R> for BufferLayeredRecordDecompressor<R> {
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
                        self.add_field_decompressor(las::v3::LasRGBNIRDecompressor::new())
                    }
                    LazItemType::Byte14(count) => self.add_field_decompressor(
                        las::v3::LasExtraByteDecompressor::new(count as usize),
                    ),
                    _ => {
                        return Err(LasZipError::UnsupportedLazItemVersion(
                            record_item.item_type,
                            record_item.version,
                        ))
                    }
                },
                _ => {
                    return Err(LasZipError::UnsupportedLazItemVersion(
                        record_item.item_type,
                        record_item.version,
                    ))
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
                    Record Compressors implementations
***************************************************************************************************/

pub trait PointFieldCompressor<W: Write, P> {
    fn init_first_point(&mut self, dst: &mut W, first_point: &P) -> std::io::Result<()>;

    fn compress_field_with(
        &mut self,
        encoder: &mut encoders::ArithmeticEncoder<W>,
        current_point: &P,
    ) -> std::io::Result<()>;
}

pub trait BufferFieldCompressor<W: Write> {
    fn size_of_field(&self) -> usize;

    fn compress_first(&mut self, dst: &mut W, buf: &[u8]) -> std::io::Result<()>;

    fn compress_with(
        &mut self,
        encoder: &mut encoders::ArithmeticEncoder<W>,
        buf: &[u8],
    ) -> std::io::Result<()>;
}

pub struct PointRecordCompressor<W: Write, P> {
    compressors: Vec<Box<dyn PointFieldCompressor<W, P>>>,
    is_first_compression: bool,
    encoder: encoders::ArithmeticEncoder<W>,
}

impl<W: Write, P> PointRecordCompressor<W, P> {
    pub fn new(output: W) -> Self {
        Self {
            compressors: vec![],
            is_first_compression: true,
            encoder: ArithmeticEncoder::new(output),
        }
    }

    pub fn add_compressor<C: 'static + PointFieldCompressor<W, P>>(&mut self, c: C) {
        self.compressors.push(Box::new(c));
    }

    pub fn add_boxed_compressor(&mut self, c: Box<dyn PointFieldCompressor<W, P>>) {
        self.compressors.push(c);
    }

    pub fn compress_next(&mut self, point: &P) -> std::io::Result<()> {
        if self.is_first_compression {
            for compressor in &mut self.compressors {
                compressor.init_first_point(self.encoder.out_stream(), point)?;
            }
            self.is_first_compression = false;
        } else {
            for compressor in &mut self.compressors {
                compressor.compress_field_with(&mut self.encoder, point)?;
            }
        }
        Ok(())
    }

    pub fn done(&mut self) -> std::io::Result<()> {
        self.encoder.done()
    }

    pub fn into_stream(self) -> W {
        self.encoder.into_stream()
    }
}

//FIXME idea to reduce code copy pasta
/*
pub struct BufferDecompressor<R: Read, InternalPointType> {
    field_decompressors: Vec<Box<dyn LayeredPointFieldDecompressor<R, InternalPointType>>>,
}
*/
pub struct BufferRecordCompressor<W: Write> {
    is_first_compression: bool,
    field_compressors: Vec<Box<dyn BufferFieldCompressor<W>>>,
    encoder: encoders::ArithmeticEncoder<W>,
    record_size: usize,
}

impl<W: Write> BufferRecordCompressor<W> {
    pub fn new(output: W) -> Self {
        Self::with_encoder(encoders::ArithmeticEncoder::new(output))
    }

    pub fn with_encoder(encoder: encoders::ArithmeticEncoder<W>) -> Self {
        Self {
            is_first_compression: true,
            field_compressors: vec![],
            encoder,
            record_size: 0,
        }
    }

    pub fn add_field_compressor<T: 'static + BufferFieldCompressor<W>>(&mut self, field: T) {
        self.record_size += field.size_of_field();
        self.field_compressors.push(Box::new(field));
    }

    pub fn add_boxed_compressor(&mut self, c: Box<dyn BufferFieldCompressor<W>>) {
        self.record_size += c.size_of_field();
        self.field_compressors.push(c);
    }

    pub fn record_size(&self) -> usize {
        self.record_size
    }

    pub fn compress(&mut self, input: &[u8]) -> std::io::Result<()> {
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

    pub fn reset(&mut self) {
        self.is_first_compression = true;
        self.encoder.reset();
        self.field_compressors.clear();
        self.record_size = 0;
    }

    pub fn set_fields_from(&mut self, laz_items: &Vec<LazItem>) -> Result<(), LasZipError> {
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
                        ))
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
                        ))
                    }
                },
                _ => {
                    return Err(LasZipError::UnsupportedLazItemVersion(
                        record_item.item_type,
                        record_item.version,
                    ))
                }
            }
        }
        Ok(())
    }

    pub fn into_stream(self) -> W {
        self.encoder.into_stream()
    }

    pub(crate) fn borrow_mut_stream(&mut self) -> &mut W {
        self.encoder.out_stream()
    }

    pub fn done(&mut self) -> std::io::Result<()> {
        self.encoder.done()
    }
}

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

impl<IntType, R> BufferFieldDecompressor<R> for IntegerFieldDecompressor<IntType>
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

impl<IntType, W> BufferFieldCompressor<W> for IntegerFieldCompressor<IntType>
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

        let encoder = encoders::ArithmeticEncoder::new(stream);
        let mut compressor = BufferRecordCompressor::with_encoder(encoder);
        compressor.done().unwrap();

        let stream = compressor.into_stream();
        let data = stream.into_inner();

        assert_eq!(&data, &[1u8, 0u8, 0u8, 0u8]);
    }

    #[test]
    fn dyna2() {
        let stream = std::io::Cursor::new(Vec::<u8>::new());

        let encoder = encoders::ArithmeticEncoder::new(stream);
        let mut compressor = BufferRecordCompressor::with_encoder(encoder);
        compressor.add_field_compressor(IntegerFieldCompressor::<i32>::new());
        compressor.compress(&[0u8, 0u8, 0u8, 0u8]).unwrap();
        compressor.done().unwrap();

        let stream = compressor.into_stream();
        let data = stream.into_inner();

        assert_eq!(&data, &[0u8, 0u8, 0u8, 0u8, 1u8, 0u8, 0u8, 0u8]);
    }

    #[test]
    fn dyna3() {
        let stream = std::io::Cursor::new(Vec::<u8>::new());

        let encoder = encoders::ArithmeticEncoder::new(stream);
        let mut compressor = BufferRecordCompressor::with_encoder(encoder);
        compressor.add_field_compressor(IntegerFieldCompressor::<i32>::new());
        compressor.compress(&[17u8, 42u8, 35u8, 1u8]).unwrap();
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

        let encoder = encoders::ArithmeticEncoder::new(stream);
        let mut compressor = BufferRecordCompressor::with_encoder(encoder);
        compressor.add_field_compressor(IntegerFieldCompressor::<i32>::new());
        compressor.compress(&[]).unwrap();
        compressor.done().unwrap();
    }
}
