//! This module contains a quick implementation of a LAS file reader
//! with just enough code to be able to read points in any LAS file but not enough
//! to support all the features of LAS.

#![allow(dead_code)]
use crate::las::laszip::{LasZipDecompressor, LazVlr};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};

/// LAS header with only the minimum information
/// to be able to read points contained in a LAS file.
#[derive(Debug)]
pub struct QuickHeader {
    pub major: u8,
    pub minor: u8,
    pub offset_to_points: u32,
    pub num_vlrs: u32,
    pub point_format_id: u8,
    pub point_size: u16,
    pub num_points: u64,
    pub header_size: u16,
}

impl QuickHeader {
    pub fn read_from<R: Read + Seek>(src: &mut R) -> std::io::Result<Self> {
        src.seek(SeekFrom::Start(24))?;
        let major = src.read_u8()?;
        let minor = src.read_u8()?;

        src.seek(SeekFrom::Start(94))?;
        let header_size = src.read_u16::<LittleEndian>()?;
        let offset_to_points = src.read_u32::<LittleEndian>()?;
        let num_vlrs = src.read_u32::<LittleEndian>()?;
        let point_format_id = src.read_u8()?;
        let point_size = src.read_u16::<LittleEndian>()?;
        let num_points = if major == 1 && minor == 4 {
            src.seek(SeekFrom::Start(247))?;
            src.read_u64::<LittleEndian>()?
        } else {
            u64::from(src.read_u32::<LittleEndian>()?)
        };

        Ok(Self {
            major,
            minor,
            offset_to_points,
            num_vlrs,
            point_format_id,
            point_size,
            num_points,
            header_size,
        })
    }


    pub fn num_extra_bytes(&self) -> u16 {
        let point_size_wo_extra = match self.point_format_id {
            0 => 20,
            1 => 28,
            2 => 26,
            3 => 34,
            6 => 30,
            7 => 36,
            8 => 38,
            _ => panic!("Unknown fmt id")
        };

        self.point_size - point_size_wo_extra
    }
}

pub struct Vlr {
    user_id: [u8; 16],
    record_id: u16,
    #[allow(dead_code)]
    description: [u8; 32],
    data: Vec<u8>,
}

impl Vlr {
    pub fn read_from<R: Read>(src: &mut R) -> std::io::Result<Self> {
        src.read_u16::<LittleEndian>()?; // reserved
        let mut user_id = [0u8; 16];
        src.read_exact(&mut user_id)?;

        let record_id = src.read_u16::<LittleEndian>()?;
        let record_length = src.read_u16::<LittleEndian>()?;

        let mut description = [0u8; 32];
        src.read_exact(&mut description)?;

        let mut data = Vec::<u8>::new();
        data.resize(record_length as usize, 0);
        src.read_exact(&mut data)?;

        Ok(Self {
            user_id,
            record_id,
            description,
            data,
        })
    }
}

pub fn read_vlrs_and_get_laszip_vlr<R: Read>(src: &mut R, header: &QuickHeader) -> Option<LazVlr> {
    let mut laszip_vlr = None;
    for _i in 0..header.num_vlrs {
        let vlr = Vlr::read_from(src).unwrap();
        if vlr.record_id == 22204
            && String::from_utf8_lossy(&vlr.user_id).trim_end_matches(|c| c as u8 == 0)
            == "laszip encoded"
        {
            laszip_vlr = Some(LazVlr::from_buffer(&vlr.data).unwrap());
        }
    }
    laszip_vlr
}

const IS_COMPRESSED_MASK: u8 = 0x80;
fn is_point_format_compressed(point_format_id: u8) -> bool {
    point_format_id & IS_COMPRESSED_MASK == IS_COMPRESSED_MASK
}
pub fn point_format_id_compressed_to_uncompressd(point_format_id: u8) -> u8 {
    point_format_id & 0x3f
}

fn point_format_id_uncompressed_to_compressed(point_format_id: u8) -> u8 {
    point_format_id | 0x80
}


pub trait LasPointReader {
    fn read_next_into(&mut self, buffer: &mut [u8]) -> std::io::Result<()>;
}

struct RawPointReader<R: Read> {
    src: R,
}

impl<R: Read> LasPointReader for RawPointReader<R> {
    fn read_next_into(&mut self, buffer: &mut [u8]) -> std::io::Result<()> {
        self.src.read_exact(buffer)
    }
}

impl<'a, R: Read + Seek> LasPointReader for LasZipDecompressor<'a, R> {
    fn read_next_into(&mut self, buffer: &mut [u8]) -> std::io::Result<()> {
        self.decompress_one(buffer)
    }
}



/// Reader, that knows just enough things to be able to read LAS and LAZ data
pub struct SimpleReader<'a> {
    pub header: QuickHeader,
    point_reader: Box<dyn LasPointReader + 'a>,
    internal_buffer: Vec<u8>,
    current_index: u64,
}


impl<'a> SimpleReader<'a> {
    pub fn new<R: Read + Seek + 'a>(mut src: R) -> std::io::Result<Self> {
        let mut header = QuickHeader::read_from(&mut src)?;
        src.seek(SeekFrom::Start(header.header_size as u64))?;
        let laszip_vlr = read_vlrs_and_get_laszip_vlr(&mut src, &header);
        src.seek(SeekFrom::Start(header.offset_to_points as u64))?;
        let point_reader: Box<dyn LasPointReader> =
            if is_point_format_compressed(header.point_format_id) {
                Box::new(
                    LasZipDecompressor::new(
                        src,
                        laszip_vlr.expect("Compressed data, but no Laszip Vlr found"),
                    )
                    .unwrap(),
                )
            } else {
                Box::new(RawPointReader { src })
            };
        header.point_format_id = point_format_id_compressed_to_uncompressd(header.point_format_id);
        let internal_buffer = vec![0u8; header.point_size as usize];
        Ok(Self {
            header,
            point_reader,
            internal_buffer,
            current_index: 0,
        })
    }

    pub fn read_next(&mut self) -> Option<std::io::Result<&[u8]>> {
        if self.current_index < self.header.num_points {
            if let Err(e) = self.point_reader.read_next_into(&mut self.internal_buffer) {
                Some(Err(e))
            } else {
                self.current_index += 1;
                Some(Ok(self.internal_buffer.as_slice()))
            }
        } else {
            None
        }
    }
}
