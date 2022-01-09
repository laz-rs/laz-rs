use laz::las::file::SimpleReader;
use std::fs::File;
use std::io::BufReader;

#[test]
fn test_version_1_point_wise() {
    let mut las_file = SimpleReader::new(BufReader::new(
        File::open("./tests/data/point-version-1-point-wise.las").unwrap(),
    ))
    .unwrap();

    let mut laz_file = SimpleReader::new(BufReader::new(
        File::open("./tests/data/point-version-1-point-wise.laz").unwrap(),
    ))
    .unwrap();

    assert_eq!(las_file.header.num_points, laz_file.header.num_points);

    for i in 0..las_file.header.num_points {
        let las_point = las_file.read_next().unwrap().unwrap();
        let laz_point = laz_file.read_next().unwrap().unwrap();

        assert_eq!(las_point, laz_point, "Point {} are not equal", i);
    }
}
