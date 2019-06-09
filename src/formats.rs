use std::io::{Cursor, Read, Write};

use num_traits::{AsPrimitive, PrimInt, zero, Zero};

use crate::compressors;
use crate::decoders;
use crate::decompressors;
use crate::encoders;
use crate::packers::Packable;

pub trait FieldDecompressor<R: Read> {
    fn size_of_field(&self) -> usize;

    fn decompress_with(
        &mut self, decoder: &mut decoders::ArithmeticDecoder<R>, buf: &mut [u8]);
}


pub struct RecordDecompressor<R: Read> {
    fields: Vec<Box<dyn FieldDecompressor<R>>>,
    decoder: decoders::ArithmeticDecoder<R>,
    first_decompression: bool,
}

impl<R: Read> RecordDecompressor<R> {
    pub fn new(decoder: decoders::ArithmeticDecoder<R>) -> Self {
        Self {
            fields: vec![],
            decoder,
            first_decompression: true,
        }
    }

    pub fn add_field<T: 'static + FieldDecompressor<R>>(&mut self, field: T) {
        self.fields.push(Box::new(field));
    }

    pub fn decompress(&mut self, out: &mut [u8]) {
        let record_size = self.fields.iter().map(|f| f.size_of_field()).sum();
        if out.len() < record_size {
            panic!("Input buffer to small")
        }
        let mut field_start = 0;
        for field in &mut self.fields {
            let field_end = field_start + field.size_of_field();
            field.decompress_with(&mut self.decoder, &mut out[field_start..field_end]);
            field_start = field_end;
        }

        // the decoder needs to be told that it should read the
        // init bytes after the first record has been read
        if self.first_decompression {
            self.first_decompression = false;
            self.decoder.read_init_bytes().unwrap();
        }
    }


    pub fn into_stream(self) -> R {
        self.decoder.into_stream()
    }

    pub fn reset(&mut self) {
        self.decoder.reset();
        self.first_decompression = true;
    }
}

pub trait FieldCompressor<W: Write> {
    fn size_of_field(&self) -> usize;

    fn compress_with(
        &mut self, encoder: &mut encoders::ArithmeticEncoder<W>, buf: &[u8],
    );
}


pub struct RecordCompressor<W: Write> {
    field_compressors: Vec<Box<dyn FieldCompressor<W>>>,
    encoder: encoders::ArithmeticEncoder<W>,
    first_compression: bool,
}

impl<W: Write> RecordCompressor<W> {
    pub fn new(encoder: encoders::ArithmeticEncoder<W>) -> Self {
        Self {
            field_compressors: vec![],
            encoder,
            first_compression: false,
        }
    }

    pub fn add_field_compressor<T: 'static + FieldCompressor<W>>(&mut self, field: T) {
        self.field_compressors.push(Box::new(field));
    }

    pub fn compress(&mut self, input: &[u8]) {
        let record_size = self.field_compressors.iter().map(|f| f.size_of_field()).sum();
        if input.len() < record_size {
            panic!("Input buffer to small")
        }
        let mut field_start = 0;
        for field in &mut self.field_compressors {
            let field_end = field_start + field.size_of_field();
            field.compress_with(&mut self.encoder, &input[field_start..field_end]);
            field_start = field_end;
        }
    }

    pub fn into_stream(self) -> W {
        self.encoder.into_stream()
    }

    pub fn done(&mut self) {
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
            decompressor: decompressors::IntegerDecompressorBuilder::new().bits(std::mem::size_of::<IntType>() as u32 * 8).build(),
            diff_method: StandardDiffMethod::<IntType>::new(),
        }
    }
}

impl<IntType, R> FieldDecompressor<R> for IntegerFieldDecompressor<IntType>
    where i32: num_traits::cast::AsPrimitive<IntType>,
          IntType: Zero + Copy + PrimInt + Packable + AsPrimitive<i32> + AsPrimitive<<IntType as Packable>::Type>,
          <IntType as Packable>::Type: AsPrimitive<IntType>,
          R: Read
{
    fn size_of_field(&self) -> usize {
        std::mem::size_of::<IntType>()
    }

    fn decompress_with(
        &mut self, mut decoder: &mut decoders::ArithmeticDecoder<R>, mut buf: &mut [u8]) where Self: Sized {
        if !self.decompressor_inited {
            self.decompressor.init();
        }

        let r: IntType = if self.diff_method.have_value() {
            let v = self.decompressor.decompress(&mut decoder, self.diff_method.value().as_(), 0).as_();
            IntType::pack(v.as_(), &mut buf);
            v
        } else {
            // this is probably the first time we're reading stuff, read the record as is
            decoder.in_stream().read_exact(&mut buf[0..std::mem::size_of::<IntType>()]).unwrap();
            IntType::unpack(&buf).as_()
        };
        self.diff_method.push(r);
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
            compressor: compressors::IntegerCompressor::new(std::mem::size_of::<IntType>() as u32 * 8, 1, 8, 0),
            diff_method: StandardDiffMethod::<IntType>::new(),
        }
    }
}


impl<IntType, W> FieldCompressor<W> for IntegerFieldCompressor<IntType>
    where IntType: Zero + Copy + PrimInt + Packable + 'static + AsPrimitive<i32>,
          <IntType as Packable>::Type: AsPrimitive<IntType>,
          W: Write
{
    fn size_of_field(&self) -> usize {
        std::mem::size_of::<IntType>()
    }

    fn compress_with(&mut self, mut encoder: &mut encoders::ArithmeticEncoder<W>, buf: &[u8]) {
        let this_val: IntType = IntType::unpack(&buf).as_();
        if !self.compressor_inited {
            self.compressor.init();
        }
        // Let the differ decide what values we're going to push
        if self.diff_method.have_value() {
            self.compressor.compress(&mut encoder, self.diff_method.value().as_(), this_val.as_(), 0);
        } else {
            // differ is not ready for us to start encoding values
            // for us, so we need to write raw into
            // the outputstream
            encoder.out_stream().write_all(&buf).unwrap();
        }
        self.diff_method.push(this_val);
    }
}

#[cfg(test)]
mod test {
    use std::io::{Cursor, Seek, SeekFrom};

    use super::*;

    #[test]
    fn t() {
        let decoder = decoders::ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new()));
        let mut mdr = RecordDecompressor::new(decoder);

        let i32_field_decompressor = IntegerFieldDecompressor::<i32>::new();
        mdr.add_field(i32_field_decompressor);
    }

    #[test]
    fn dyna() {
        let mut stream = std::io::Cursor::new(Vec::<u8>::new());

        let mut encoder = encoders::ArithmeticEncoder::new(stream);
        let mut compressor = RecordCompressor::new(encoder);
        compressor.done();

        let mut stream = compressor.into_stream();
        let data = stream.into_inner();

        assert_eq!(&data, &[1u8, 0u8, 0u8, 0u8]);
    }

    #[test]
    fn dyna2() {
        let mut stream = std::io::Cursor::new(Vec::<u8>::new());

        let mut encoder = encoders::ArithmeticEncoder::new(stream);
        let mut compressor = RecordCompressor::new(encoder);
        compressor.add_field_compressor(IntegerFieldCompressor::<i32>::new());
        compressor.compress(&[0u8, 0u8, 0u8, 0u8]);
        compressor.done();

        let mut stream = compressor.into_stream();
        let data = stream.into_inner();

        assert_eq!(&data, &[0u8, 0u8, 0u8, 0u8, 1u8, 0u8, 0u8, 0u8]);
    }

    #[test]
    fn dyna3() {
        let mut stream = std::io::Cursor::new(Vec::<u8>::new());

        let mut encoder = encoders::ArithmeticEncoder::new(stream);
        let mut compressor = RecordCompressor::new(encoder);
        compressor.add_field_compressor(IntegerFieldCompressor::<i32>::new());
        compressor.compress(&[17u8, 42u8, 35u8, 1u8]);
        compressor.done();

        let mut stream = compressor.into_stream();
        stream.seek(SeekFrom::Start(0)).unwrap();

        let mut read_from_stream = [0u8; 8];
        stream.read_exact(&mut read_from_stream).unwrap();
        assert_eq!(&read_from_stream, &[17u8, 42u8, 35u8, 1u8, 1u8, 0u8, 0u8, 0u8]);

        let data = stream.into_inner();
        assert_eq!(&data, &[17u8, 42u8, 35u8, 1u8, 1u8, 0u8, 0u8, 0u8]);
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
