use std::io::{Seek, Read};
use crate::las::file::SimpleReader;

pub fn check_decompression<R1: Read + Seek + 'static, R2: Read + Seek+ 'static>(laz_src: R1, las_src: R2) {
    let mut laz_reader = SimpleReader::new(laz_src).unwrap();
    let mut las_reader = SimpleReader::new(las_src).unwrap();

    assert_eq!(laz_reader.header.num_points, las_reader.header.num_points);
    assert_eq!(laz_reader.header.point_format_id, las_reader.header.point_format_id);

    while let (Some(laz_pts), Some(las_pts)) = (laz_reader.read_next(), las_reader.read_next()) {
        let laz_pts = laz_pts.unwrap();
        let las_pts = las_pts.unwrap();

        assert_eq!(laz_pts, las_pts);
    }
}