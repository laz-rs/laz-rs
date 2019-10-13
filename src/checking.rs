use crate::las::file::SimpleReader;
use crate::las::laszip::{LasZipCompressor, LasZipDecompressor, LazItem, LazVlr};
use std::io::{Cursor, Read, Seek};

pub fn check_decompression< R1: Read + Seek,  R2: Read + Seek>(
    laz_src: R1,
    las_src: R2,
) {
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

pub fn check_that_we_can_decompress_what_we_compressed<R: Read + Seek>(
    las_src: R,
    laz_items: Vec<LazItem>,
) {
    let mut las_reader = SimpleReader::new(las_src).unwrap();
    let laz_vlr = LazVlr::from_laz_items(laz_items);
    assert_eq!(laz_vlr.items_size() as usize, las_reader.header.point_size as usize);
    let mut compressor =
        LasZipCompressor::from_laz_vlr(Cursor::new(Vec::<u8>::new()), laz_vlr).unwrap();

    while let Some(point_buf) = las_reader.read_next() {
        let point_buf = point_buf.unwrap();
        compressor.compress_one(point_buf).unwrap()
    }
    compressor.done().unwrap();

    let vlr = compressor.vlr().clone();
    let mut compression_output = compressor.into_stream();
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
