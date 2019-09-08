#![allow(dead_code)]
use crate::las::laszip::LazVlr;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug)]
pub struct QuickHeader {
    pub major: u8,
    pub minor: u8,
    pub offset_to_points: u32,
    pub num_vlrs: u32,
    pub point_format_id: u8,
    pub point_size: u16,
    pub num_points: u64,
    header_size: u16,
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
        println!("{:?}", user_id);

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
pub trait LasPointIO {
    fn read_from<R: Read>(&mut self, src: &mut R) -> std::io::Result<()>;
}

pub struct UncompressedTypedPointReader<R: Read, P: LasPointIO + Default> {
    src: R,
    point_type: std::marker::PhantomData<P>,
}

impl<R: Read, P: LasPointIO + Default> UncompressedTypedPointReader<R, P> {
    fn new(src: R) -> Self {
        Self {
            src,
            point_type: std::marker::PhantomData,
        }
    }

    fn read_next(&mut self) -> std::io::Result<P> {
        let mut p = P::default();
        p.read_from(&mut self.src)?;
        Ok(p)
    }
}

pub struct PointReader<R: Read> {
    src: R,
}

impl<R: Read> PointReader<R> {
    fn read_next_point_buffer(&mut self, _out: &mut [u8]) -> std::io::Result<()> {
        unimplemented!()
    }

    fn read_next_as<P: LasPointIO>(&mut self) -> std::io::Result<P> {
        unimplemented!()
    }
}

pub struct TypedSimpleReader<R: Read + Seek, P: LasPointIO + Default> {
    pub header: QuickHeader,
    pub laszip_vlr: Option<LazVlr>,
    point_reader: UncompressedTypedPointReader<R, P>,
}

impl<R: Read + Seek, P: LasPointIO + Default> TypedSimpleReader<R, P> {
    pub fn new(mut src: R) -> std::io::Result<Self> {
        let header = QuickHeader::read_from(&mut src)?;
        src.seek(SeekFrom::Start(header.header_size as u64))?;
        let mut laszip_vlr = None;
        for _i in 0..header.num_vlrs {
            let vlr = Vlr::read_from(&mut src)?;
            if vlr.record_id == 22204
                && String::from_utf8_lossy(&vlr.user_id).trim_end_matches(|c| c as u8 == 0)
                    == "laszip encoded"
            {
                laszip_vlr = Some(LazVlr::from_buffer(&vlr.data).unwrap());
            }
        }
        let point_reader = UncompressedTypedPointReader::new(src);
        Ok(Self {
            header,
            laszip_vlr,
            point_reader,
        })
    }
}

pub struct SimpleReader<R: Read + Seek> {
    pub src: R,
    pub header: QuickHeader,
    pub laszip_vlr: Option<LazVlr>,
}

impl<R: Read + Seek> SimpleReader<R> {
    pub fn new(mut src: R) -> std::io::Result<Self> {
        let header = QuickHeader::read_from(&mut src)?;
        src.seek(SeekFrom::Start(header.header_size as u64))?;
        let mut laszip_vlr = None;
        for _i in 0..header.num_vlrs {
            let vlr = Vlr::read_from(&mut src)?;
            if vlr.record_id == 22204
                && String::from_utf8_lossy(&vlr.user_id).trim_end_matches(|c| c as u8 == 0)
                    == "laszip encoded"
            {
                laszip_vlr = Some(LazVlr::from_buffer(&vlr.data).unwrap());
            }
        }
        Ok(Self {
            src,
            header,
            laszip_vlr,
        })
    }
}
