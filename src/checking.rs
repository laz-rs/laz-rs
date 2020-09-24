use crate::las::file::SimpleReader;
use crate::las::laszip::{LasZipCompressor, LasZipDecompressor, LazItem, LazVlr};
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};

pub fn check_decompression<R1: Read + Seek + Send, R2: Read + Seek + Send>(laz_src: R1, las_src: R2) {
    let mut laz_reader = SimpleReader::new(laz_src).unwrap();
    let mut las_reader = SimpleReader::new(las_src).unwrap();

    assert_eq!(laz_reader.header.num_points, las_reader.header.num_points);
    assert_eq!(
        laz_reader.header.point_format_id,
        las_reader.header.point_format_id
    );

    while let (Some(laz_pts), Some(las_pts)) = (laz_reader.read_next(), las_reader.read_next()) {
        let laz_pts = laz_pts.unwrap();
        let las_pts = las_pts.unwrap();

        assert_eq!(laz_pts, las_pts);
    }
}

pub struct LasChecker<'a> {
    reader: crate::las::file::SimpleReader<'a>,
}

impl<'a> LasChecker<'a> {
    pub fn new<R: Read + Seek + Send + 'a>(src: R) -> crate::Result<Self> {
        Ok(Self {
            reader: crate::las::file::SimpleReader::new(src)?,
        })
    }

    pub fn from_path(las_path: &str) -> crate::Result<Self> {
        Ok(Self {
            reader: crate::las::file::SimpleReader::new(BufReader::new(File::open(las_path)?))?,
        })
    }

    pub fn check(&mut self, points: &[u8]) {
        assert_eq!(points.len() % self.reader.header.point_size as usize, 0);

        for point in points.chunks_exact(self.reader.header.point_size as usize) {
            assert_eq!(point, self.reader.read_next().unwrap().unwrap());
        }
    }
}

pub fn check_that_we_can_decompress_what_we_compressed<R: Read + Seek + Send>(
    las_src: R,
    laz_items: Vec<LazItem>,
) {
    let mut las_reader = SimpleReader::new(las_src).unwrap();
    let laz_vlr = LazVlr::from_laz_items(laz_items);
    assert_eq!(
        laz_vlr.items_size() as usize,
        las_reader.header.point_size as usize
    );
    let mut compressor = LasZipCompressor::new(Cursor::new(Vec::<u8>::new()), laz_vlr).unwrap();

    while let Some(point_buf) = las_reader.read_next() {
        let point_buf = point_buf.unwrap();
        compressor.compress_one(point_buf).unwrap()
    }
    compressor.done().unwrap();

    let vlr = compressor.vlr().clone();
    let mut compression_output = compressor.into_inner();
    compression_output.set_position(0);

    let mut decompressor = LasZipDecompressor::new(compression_output, vlr).unwrap();

    let mut decompressed_point = vec![0u8; las_reader.header.point_size as usize];
    while let Some(point_buf) = las_reader.read_next() {
        let point_buf = point_buf.unwrap();
        decompressor
            .decompress_one(&mut decompressed_point)
            .unwrap();
        assert_eq!(decompressed_point.as_slice(), point_buf);
    }
}
