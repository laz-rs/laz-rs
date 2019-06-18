use std::io::{Read, Write};

use num_traits::{zero, AsPrimitive, PrimInt, Zero};

use crate::compressors;
use crate::decoders;
use crate::decompressors;
use crate::encoders;
use crate::las;
use crate::las::laszip::{LazItem, LazItemType, LasZipError};
use crate::packers::Packable;

//TODO since all field compressor do not actually compress the 1st point
// but write it directly to the dst the FieldCompressor trait should maybe define
// "compress_first" & FieldDecompressor have "decompress_first"

pub trait FieldDecompressor<R: Read> {
    fn size_of_field(&self) -> usize;

    fn decompress_with(
        &mut self,
        decoder: &mut decoders::ArithmeticDecoder<R>,
        buf: &mut [u8],
    ) -> std::io::Result<()>;
}

pub struct RecordDecompressor<R: Read> {
    field_decompressors: Vec<Box<dyn FieldDecompressor<R>>>,
    decoder: decoders::ArithmeticDecoder<R>,
    first_decompression: bool,
    record_size: usize,
}

impl<R: Read> RecordDecompressor<R> {
    pub fn new(input: R) -> Self {
        Self::with_decoder(decoders::ArithmeticDecoder::new(input))
    }

    pub fn with_decoder(decoder: decoders::ArithmeticDecoder<R>) -> Self {
        Self {
            field_decompressors: vec![],
            decoder,
            first_decompression: true,
            record_size: 0,
        }
    }

    pub fn add_field_decompressor<T: 'static + FieldDecompressor<R>>(&mut self, field: T) {
        self.record_size += field.size_of_field();
        self.field_decompressors.push(Box::new(field));
    }

    pub fn record_size(&self) -> usize {
        self.record_size
    }

    pub fn decompress(&mut self, out: &mut [u8]) -> std::io::Result<()> {
        let mut field_start = 0;
        for field in &mut self.field_decompressors {
            let field_end = field_start + field.size_of_field();
            field.decompress_with(&mut self.decoder, &mut out[field_start..field_end])?;
            field_start = field_end;
        }

        // the decoder needs to be told that it should read the
        // init bytes after the first record has been read
        if self.first_decompression {
            self.first_decompression = false;
            self.decoder.read_init_bytes()?;
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
        self.first_decompression = true;
        self.field_decompressors.clear();
        self.record_size = 0;
    }

    pub fn set_fields_from(&mut self, laz_items: &Vec<LazItem>) -> Result<(),LasZipError> {
        for record_item in laz_items {
            match record_item.version {
                1 => match record_item.item_type {
                    LazItemType::Byte(_) => {
                        self.add_field_decompressor(las::v1::ExtraBytesDecompressor::new(
                            record_item.size as usize,
                        ))
                    }
                    LazItemType::Point10 => {
                        self.add_field_decompressor(las::v1::Point10Decompressor::new())
                    }
                    LazItemType::GpsTime => {
                        self.add_field_decompressor(las::v1::GpsTimeDecompressor::new())
                    }
                    LazItemType::RGB12 => self.add_field_decompressor(las::v1::RGBDecompressor::new()),
                },
                2 => match record_item.item_type {
                    LazItemType::Byte(_) => {
                        self.add_field_decompressor(las::v2::ExtraBytesDecompressor::new(
                            record_item.size as usize,
                        ))
                    }
                    LazItemType::Point10 => {
                        self.add_field_decompressor(las::v2::Point10Decompressor::new())
                    }
                    LazItemType::GpsTime => {
                        self.add_field_decompressor(las::v2::GpsTimeDecompressor::new())
                    }
                    LazItemType::RGB12 => self.add_field_decompressor(las::v2::RGBDecompressor::new()),
                },
                _ => return  Err(LasZipError::UnsupportedLazItemVersion(
                    record_item.item_type,
                    record_item.version)),
            }
        }
        Ok(())
    }
}

pub trait FieldCompressor<W: Write> {
    fn size_of_field(&self) -> usize;

    fn compress_with(
        &mut self,
        encoder: &mut encoders::ArithmeticEncoder<W>,
        buf: &[u8],
    ) -> std::io::Result<()>;
}

pub struct RecordCompressor<W: Write> {
    field_compressors: Vec<Box<dyn FieldCompressor<W>>>,
    encoder: encoders::ArithmeticEncoder<W>,
    record_size: usize,
}

impl<W: Write> RecordCompressor<W> {
    pub fn new(output: W) -> Self {
        Self::with_encoder(encoders::ArithmeticEncoder::new(output))
    }

    pub fn with_encoder(encoder: encoders::ArithmeticEncoder<W>) -> Self {
        Self {
            field_compressors: vec![],
            encoder,
            record_size: 0,
        }
    }

    pub fn add_field_compressor<T: 'static + FieldCompressor<W>>(&mut self, field: T) {
        self.record_size += field.size_of_field();
        self.field_compressors.push(Box::new(field));
    }

    pub fn record_size(&self) -> usize {
        self.record_size
    }

    pub fn compress(&mut self, input: &[u8]) -> std::io::Result<()> {

        let mut field_start = 0;
        for field in &mut self.field_compressors {
            let field_end = field_start + field.size_of_field();
            field.compress_with(&mut self.encoder, &input[field_start..field_end])?;
            field_start = field_end;
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        self.encoder.reset();
        self.field_compressors.clear();
        self.record_size = 0;
    }

    pub fn set_fields_from(&mut self, laz_items: &Vec<LazItem>) -> Result<(), LasZipError> {
        for record_item in laz_items {
            match record_item.version {
                1 => match record_item.item_type {
                    LazItemType::Point10 => {
                        self.add_field_compressor(las::v1::Point10Compressor::new())
                    }
                    LazItemType::GpsTime => {
                        self.add_field_compressor(las::v1::GpsTimeCompressor::new())
                    }
                    LazItemType::RGB12 => {
                        self.add_field_compressor(las::v1::RGBCompressor::new())
                    }
                    LazItemType::Byte(_) => self.add_field_compressor(
                        las::v1::ExtraBytesCompressor::new(record_item.size as usize),
                    ),
                },
                2 => match record_item.item_type {
                    LazItemType::Point10 => {
                        self.add_field_compressor(las::v2::Point10Compressor::new())
                    }
                    LazItemType::GpsTime => {
                        self.add_field_compressor(las::v2::GpsTimeCompressor::new())
                    }
                    LazItemType::RGB12 => {
                        self.add_field_compressor(las::v2::RGBCompressor::new())
                    }
                    LazItemType::Byte(_) => self.add_field_compressor(
                        las::v2::ExtraBytesCompressor::new(record_item.size as usize),
                    ),
                },
                _ => return  Err(LasZipError::UnsupportedLazItemVersion(
                    record_item.item_type,
                    record_item.version)),
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
    decompressor_inited: bool,
    decompressor: decompressors::IntegerDecompressor,
    diff_method: StandardDiffMethod<IntType>,
}

impl<IntType: Zero + Copy + PrimInt> IntegerFieldDecompressor<IntType> {
    pub fn new() -> Self {
        Self {
            decompressor_inited: false,
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

    fn decompress_with(
        &mut self,
        mut decoder: &mut decoders::ArithmeticDecoder<R>,
        mut buf: &mut [u8],
    ) -> std::io::Result<()>
    where
        Self: Sized,
    {
        if !self.decompressor_inited {
            self.decompressor.init();
        }

        let r: IntType = if self.diff_method.have_value() {
            let v: IntType = self
                .decompressor
                .decompress(&mut decoder, self.diff_method.value().as_(), 0)?
                .as_(); // i32 -> IntType
            v.pack_into(&mut buf);
            v
        } else {
            // this is probably the first time we're reading stuff, read the record as is
            decoder
                .in_stream()
                .read_exact(&mut buf[0..std::mem::size_of::<IntType>()])?;
            IntType::unpack_from(&buf).as_()
        };
        self.diff_method.push(r);
        Ok(())
    }
}

pub struct IntegerFieldCompressor<IntType: Zero + Copy + PrimInt> {
    compressor_inited: bool,
    compressor: compressors::IntegerCompressor,
    diff_method: StandardDiffMethod<IntType>,
}

impl<IntType: Zero + Copy + PrimInt> IntegerFieldCompressor<IntType> {
    pub fn new() -> Self {
        Self {
            compressor_inited: false,
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

    fn compress_with(
        &mut self,
        mut encoder: &mut encoders::ArithmeticEncoder<W>,
        buf: &[u8],
    ) -> std::io::Result<()> {
        let this_val: IntType = IntType::unpack_from(&buf).as_();
        if !self.compressor_inited {
            self.compressor.init();
        }
        // Let the differ decide what values we're going to push
        if self.diff_method.have_value() {
            self.compressor.compress(
                &mut encoder,
                self.diff_method.value().as_(),
                this_val.as_(),
                0,
            )?;
        } else {
            // differ is not ready for us to start encoding values
            // for us, so we need to write raw into
            // the outputstream
            encoder.out_stream().write_all(&buf)?;
        }
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
        let mut compressor = RecordCompressor::with_encoder(encoder);
        compressor.done().unwrap();

        let stream = compressor.into_stream();
        let data = stream.into_inner();

        assert_eq!(&data, &[1u8, 0u8, 0u8, 0u8]);
    }

    #[test]
    fn dyna2() {
        let stream = std::io::Cursor::new(Vec::<u8>::new());

        let encoder = encoders::ArithmeticEncoder::new(stream);
        let mut compressor = RecordCompressor::with_encoder(encoder);
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
        let mut compressor = RecordCompressor::with_encoder(encoder);
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
        let mut compressor = RecordCompressor::with_encoder(encoder);
        compressor.add_field_compressor(IntegerFieldCompressor::<i32>::new());
        compressor.compress(&[]).unwrap();
        compressor.done().unwrap();
    }

        /*
            fn test_packer_on<T: Packable>(val: T) {
                let mut buf = [0u8, std::mem::size_of::<T>()];
                T::pack(v, &mut buf);
                let v = T::unpack(&buf);
                assert_eq!(v, val);
            }
        */
}
