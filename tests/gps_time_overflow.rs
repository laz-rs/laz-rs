use std::io::Cursor;

use laz::{compress_buffer, decompress_buffer, LazItemRecordBuilder, LazItemType, LazVlr};

const POINT_SIZE: usize = 28; // Point10 (20) + GpsTime (8)

fn make_point(i: usize, gps_time: f64) -> [u8; POINT_SIZE] {
    let mut p = [0u8; POINT_SIZE];
    p[0..4].copy_from_slice(&(i as i32).to_le_bytes());
    p[4..8].copy_from_slice(&(i as i32).to_le_bytes());
    p[8..12].copy_from_slice(&(i as i32).to_le_bytes());
    p[12..14].copy_from_slice(&(1u16).to_le_bytes());
    p[14] = 0b0000_1001;
    p[15] = 2;
    p[18..20].copy_from_slice(&(1u16).to_le_bytes());
    p[20..28].copy_from_slice(&gps_time.to_le_bytes());
    p
}

// GPS times with opposite sign put the two operands at opposite ends of the
// i64 range when reinterpreted, so a non-wrapping subtraction between them
// overflows. See https://github.com/laz-rs/laz-rs/issues/72
#[test]
fn compress_gps_time_diff_does_not_overflow() {
    let n = 500;
    let mut buffer = Vec::with_capacity(n * POINT_SIZE);
    for i in 0..n {
        let gps_time = if i % 2 == 0 {
            1.0e6 + i as f64
        } else {
            -1.0e6 - i as f64
        };
        buffer.extend_from_slice(&make_point(i, gps_time));
    }

    let items = LazItemRecordBuilder::new()
        .add_item(LazItemType::Point10)
        .add_item(LazItemType::GpsTime)
        .build();
    let vlr = LazVlr::from_laz_items(items);

    let mut compressed = Cursor::new(Vec::<u8>::new());
    compress_buffer(&mut compressed, &buffer, vlr.clone()).expect("compression failed");

    let compressed = compressed.into_inner();
    let mut decompressed = vec![0u8; buffer.len()];
    decompress_buffer(&compressed, &mut decompressed, vlr).expect("decompression failed");
    assert_eq!(decompressed, buffer);
}
