use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

/// Test that on a file with only one chunk,
/// if we use parallel decompressor with a small number of points
/// everything works.
#[cfg(feature = "parallel")]
#[test]
fn test_par_decompress_less_than_chunk_size() {
    let laz_path = "tests/data/extra-bytes.laz";
    let las_path = "tests/data/extra-bytes.las";

    // Prepare LAZ file decompression
    let mut laz_file = File::open(laz_path).unwrap();
    let laz_header = laz::las::file::QuickHeader::read_from(&mut laz_file).unwrap();
    let laz_vlr = laz::las::file::read_vlrs_and_get_laszip_vlr(&mut laz_file, &laz_header).unwrap();
    laz_file
        .seek(SeekFrom::Start(laz_header.offset_to_points as u64))
        .unwrap();
    let mut decompressor = laz::ParLasZipDecompressor::new(laz_file, laz_vlr).unwrap();

    // Prepare LAS file that is our ground truth
    let mut las_file = File::open(las_path).unwrap();
    let las_header = laz::las::file::QuickHeader::read_from(&mut las_file).unwrap();
    las_file
        .seek(SeekFrom::Start(las_header.offset_to_points as u64))
        .unwrap();

    assert_eq!(las_header.point_size, laz_header.point_size);
    assert_eq!(las_header.num_points, laz_header.num_points);

    let num_points_per_iter = 50;
    let mut num_point_left = las_header.num_points;
    let mut points = vec![0u8; las_header.point_size as usize * num_points_per_iter];
    let mut expected_points = vec![0u8; las_header.point_size as usize * num_points_per_iter];

    while num_point_left > 0 {
        let points_to_read = num_points_per_iter.min(num_point_left as usize);
        let end = points_to_read * las_header.point_size as usize;
        decompressor.decompress_many(&mut points[..end]).unwrap();

        las_file.read_exact(&mut expected_points[..end]).unwrap();

        assert_eq!(&expected_points[..end], &points[..end]);
        num_point_left -= points_to_read as u64;
    }
}
