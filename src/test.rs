use std::alloc::{GlobalAlloc, Layout, System};
use std::fs::File;
use std::io::Cursor;
use std::io::{Read, Seek, SeekFrom};

use crate::las::gps::{v2::GpsTimeCompressor, v2::GpsTimeDecompressor, GpsTime};
use crate::las::point0::{v2::LasPoint0Compressor, v2::LasPoint0Decompressor, Point0};
use crate::las::rgb::{v2::LasRGBCompressor, v2::LasRGBDecompressor, RGB};
use crate::las::v1;
use crate::packers::Packable;
use crate::record::{FieldCompressor, FieldDecompressor};
use crate::record::{
    LayeredFieldCompressor, LayeredFieldDecompressor, LayeredPointRecordCompressor,
    LayeredPointRecordDecompressor, RecordCompressor, RecordDecompressor,
    SequentialPointRecordCompressor, SequentialPointRecordDecompressor,
};

/// Allocator that fails any allocation bigger than [`MAX_ALLOCATION`].
///
/// Corrupted data can claim sizes (chunk sizes, layer sizes, number of chunks...)
/// much bigger than what can be allocated. With this allocator, trying to allocate
/// such a size aborts the test binary, independently of how much memory
/// the machine running the tests has, while fallible allocations get an error.
struct LimitedAllocator;

const MAX_ALLOCATION: usize = 1 << 30;

unsafe impl GlobalAlloc for LimitedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() > MAX_ALLOCATION {
            return std::ptr::null_mut();
        }
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: LimitedAllocator = LimitedAllocator;

#[test]
fn test_compression_decompression_of_point_10() {
    let mut compressor =
        SequentialPointRecordCompressor::new(std::io::Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(LasPoint0Compressor::default());

    let n: i32 = 10000;
    let mut buf = [0u8; 20];
    for i in 0..n {
        let point = Point0 {
            x: i,
            y: i + 1000,
            z: i + 10000,
            intensity: (i + (1 << 15)) as u16,
            return_number: ((i >> 3) & 0x7) as u8,
            number_of_returns_of_given_pulse: (i & 0x7) as u8,
            scan_direction_flag: (i & 1) != 0,
            edge_of_flight_line: ((i + 1) & 1) != 0,
            classification: (i % 256) as u8,
            scan_angle_rank: (i % 128) as i8,
            user_data: ((i >> 4) % 256) as u8,
            point_source_id: (i * 30 % (1 << 16)) as u16,
        };

        point.pack_into(&mut buf);
        compressor.compress_next(&buf).unwrap();
    }
    compressor.done().unwrap();

    let compressed_data = compressor.into_inner().into_inner();

    let mut decompressor =
        SequentialPointRecordDecompressor::new(std::io::Cursor::new(compressed_data));
    decompressor.add_field_decompressor(LasPoint0Decompressor::default());

    for i in 0..n {
        decompressor.decompress_next(&mut buf).unwrap();
        let point = Point0::unpack_from(&buf);

        let expected_point = Point0 {
            x: i,
            y: i + 1000,
            z: i + 10000,
            intensity: (i + (1 << 15)) as u16,
            return_number: ((i >> 3) & 0x7) as u8,
            number_of_returns_of_given_pulse: (i & 0x7) as u8,
            scan_direction_flag: (i & 1) != 0,
            edge_of_flight_line: ((i + 1) & 1) != 0,
            classification: (i % 256) as u8,
            scan_angle_rank: (i % 128) as i8,
            user_data: ((i >> 4) % 256) as u8,
            point_source_id: (i * 30 % (1 << 16)) as u16,
        };

        assert_eq!(point, expected_point);
    }
}

#[test]
fn test_rgb() {
    let mut compressor = SequentialPointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));

    compressor.add_field_compressor(LasRGBCompressor::default());

    let n = 10000;

    let mut buf = [0u8; 6];
    for i in 0..n {
        let rgb = RGB {
            red: (i + 1000) % 65535,
            green: (i + 5000) % 65535,
            blue: (i + 10000) % 65535,
        };

        rgb.pack_into(&mut buf);
        compressor.compress_next(&buf).unwrap();
    }
    compressor.done().unwrap();
    let compressed_data = compressor.into_inner().into_inner();

    let mut decompressor =
        SequentialPointRecordDecompressor::new(std::io::Cursor::new(compressed_data));
    decompressor.add_field_decompressor(LasRGBDecompressor::default());

    for i in 0..n {
        let expected_rgb = RGB {
            red: (i + 1000) % 65535,
            green: (i + 5000) % 65535,
            blue: (i + 10000) % 65535,
        };

        decompressor.decompress_next(&mut buf).unwrap();
        let rgb = RGB::unpack_from(&buf);

        assert_eq!(rgb, expected_rgb);
    }
}

#[test]
fn test_gps_time() {
    let mut compressor = SequentialPointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(GpsTimeCompressor::default());

    let n = 10000;

    let mut buf = [0u8; std::mem::size_of::<i64>()];
    for i in 0..n {
        let gps_time = GpsTime {
            value: (i + 48741) % std::i64::MAX,
        };
        gps_time.pack_into(&mut buf);

        compressor.compress_next(&buf).unwrap();
    }
    compressor.done().unwrap();

    let compressed_data = compressor.into_inner().into_inner();

    let mut decompressor = SequentialPointRecordDecompressor::new(Cursor::new(compressed_data));
    decompressor.add_field_decompressor(GpsTimeDecompressor::default());

    for i in 0..n {
        let expected_gps_time = GpsTime {
            value: (i + 48741) % std::i64::MAX,
        };
        decompressor.decompress_next(&mut buf).unwrap();
        let gps_time = GpsTime::unpack_from(&buf);
        assert_eq!(expected_gps_time, gps_time);
    }
}

const LAS_HEADER_SIZE: u64 = 227;
const NUM_POINTS: usize = 1065;
const VLR_HEADER_SIZE: u64 = 54;

macro_rules! loop_test_on_buffer {
    ($test_name:ident, $source_las:expr, $point_size:expr, $point_start:expr, $field_compressors:expr, $field_decompressors:expr) => {
        #[test]
        fn $test_name() {
            use crate::record::RecordCompressor;
            use crate::record::RecordDecompressor;
            let mut las_file = File::open($source_las).unwrap();
            las_file.seek(SeekFrom::Start($point_start)).unwrap();

            let mut expected_buf = [0u8; $point_size];

            let mut compressor =
                SequentialPointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));
            for c in $field_compressors {
                compressor.add_boxed_compressor(c);
            }

            for _ in 0..NUM_POINTS {
                las_file.read_exact(&mut expected_buf).unwrap();
                compressor.compress_next(&expected_buf).unwrap();
            }
            compressor.done().unwrap();

            let mut compression_output = compressor.into_inner();
            compression_output.set_position(0);

            let mut decompressor = SequentialPointRecordDecompressor::new(compression_output);
            for d in $field_decompressors {
                decompressor.add_boxed_decompressor(d);
            }

            let mut buf = [0u8; $point_size];
            las_file.seek(SeekFrom::Start($point_start)).unwrap();
            for i in 0..NUM_POINTS {
                las_file.read_exact(&mut expected_buf).unwrap();
                decompressor
                    .decompress_next(&mut buf)
                    .expect(&format!("Failed to decompress point {}", i));

                let mut s;
                let mut e = 0;
                for j in 0..$point_size / 32 {
                    s = j * 32;
                    e = (j + 1) * 32;
                    assert_eq!(
                        buf[s..e],
                        expected_buf[s..e],
                        "Buffers[{}..{}] for point {} are not eq!",
                        s,
                        e,
                        i
                    );
                }
                assert_eq!(buf[e..], expected_buf[e..]);
            }
        }
    };
}

macro_rules! vec_of_boxed_buffer_compressors {
    ($($x: expr),*) => {{
        let mut vector = Vec::new();
        $(vector.push(Box::new($x) as Box<dyn FieldCompressor<_> + Send + Sync>);)*
        vector
    }}
}

macro_rules! vec_of_boxed_buffer_decompressors {
    ($($x: expr),*) => {{
        let mut vector = Vec::new();
        $(vector.push(Box::new($x) as Box<dyn FieldDecompressor<_> + Send + Sync>);)*
        vector
    }}
}

loop_test_on_buffer!(
    test_point_0_buffer,
    "tests/data/point10.las",
    20,
    LAS_HEADER_SIZE,
    vec_of_boxed_buffer_compressors![v1::LasPoint0Compressor::default()],
    vec_of_boxed_buffer_decompressors![v1::LasPoint0Decompressor::default()]
);

loop_test_on_buffer!(
    test_point_1_buffer,
    "tests/data/point-time.las",
    28,
    LAS_HEADER_SIZE,
    vec_of_boxed_buffer_compressors![
        v1::LasPoint0Compressor::default(),
        v1::LasGpsTimeCompressor::default()
    ],
    vec_of_boxed_buffer_decompressors![
        v1::LasPoint0Decompressor::default(),
        v1::LasGpsTimeDecompressor::default()
    ]
);

loop_test_on_buffer!(
    test_point_2_buffer,
    "tests/data/point-color.las",
    26,
    LAS_HEADER_SIZE,
    vec_of_boxed_buffer_compressors![
        v1::LasPoint0Compressor::default(),
        v1::LasRGBCompressor::default()
    ],
    vec_of_boxed_buffer_decompressors![
        v1::LasPoint0Decompressor::default(),
        v1::LasRGBDecompressor::default()
    ]
);

loop_test_on_buffer!(
    test_point_3_buffer,
    "tests/data/point-time-color.las",
    34,
    LAS_HEADER_SIZE,
    vec_of_boxed_buffer_compressors![
        v1::LasPoint0Compressor::default(),
        v1::LasGpsTimeCompressor::default(),
        v1::LasRGBCompressor::default()
    ],
    vec_of_boxed_buffer_decompressors![
        v1::LasPoint0Decompressor::default(),
        v1::LasGpsTimeDecompressor::default(),
        v1::LasRGBDecompressor::default()
    ]
);

loop_test_on_buffer!(
    test_point_3_extra_bytes_buffer,
    "tests/data/extra-bytes.las",
    61,
    LAS_HEADER_SIZE + VLR_HEADER_SIZE + (5 * 192),
    vec_of_boxed_buffer_compressors![
        v1::LasPoint0Compressor::default(),
        v1::LasGpsTimeCompressor::default(),
        v1::LasRGBCompressor::default(),
        v1::LasExtraByteCompressor::new(27)
    ],
    vec_of_boxed_buffer_decompressors![
        v1::LasPoint0Decompressor::default(),
        v1::LasGpsTimeDecompressor::default(),
        v1::LasRGBDecompressor::default(),
        v1::LasExtraByteDecompressor::new(27)
    ]
);

/// GPS times are compressed as the difference of their f64 bit patterns
/// reinterpreted as i64, going from a negative to a positive time
/// overflows that difference.
fn gps_times_crossing_zero() -> Vec<GpsTime> {
    (-20..=20)
        .map(|i| i as f64 * 0.25)
        .chain((1..=20).flat_map(|i| [-(i as f64), i as f64]))
        .map(GpsTime::from)
        .collect()
}

/// GPS times cycling through 4 sequences far apart from each other,
/// once the 4 sequences exist, every time is encoded as a sequence switch
/// followed by the time.
fn gps_times_interleaved_sequences() -> Vec<GpsTime> {
    (0..200i64)
        .map(|i| GpsTime::from(((i % 4) << 40) + i * 1000))
        .collect()
}

fn check_gps_time_roundtrip<C, D>(compressor: C, decompressor: D, gps_times: &[GpsTime])
where
    C: FieldCompressor<Cursor<Vec<u8>>> + Send + Sync + 'static,
    D: FieldDecompressor<Cursor<Vec<u8>>> + Send + Sync + 'static,
{
    let mut record_compressor = SequentialPointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    record_compressor.add_field_compressor(compressor);

    let mut buf = [0u8; std::mem::size_of::<i64>()];
    for gps_time in gps_times {
        gps_time.pack_into(&mut buf);
        record_compressor.compress_next(&buf).unwrap();
    }
    record_compressor.done().unwrap();

    let compressed_data = record_compressor.into_inner().into_inner();
    let mut record_decompressor =
        SequentialPointRecordDecompressor::new(Cursor::new(compressed_data));
    record_decompressor.add_field_decompressor(decompressor);

    for expected in gps_times {
        record_decompressor.decompress_next(&mut buf).unwrap();
        assert_eq!(*expected, GpsTime::unpack_from(&buf));
    }
}

#[test]
fn test_gps_time_v1_crossing_zero() {
    check_gps_time_roundtrip(
        v1::LasGpsTimeCompressor::default(),
        v1::LasGpsTimeDecompressor::default(),
        &gps_times_crossing_zero(),
    );
}

#[test]
fn test_gps_time_v2_crossing_zero() {
    check_gps_time_roundtrip(
        GpsTimeCompressor::default(),
        GpsTimeDecompressor::default(),
        &gps_times_crossing_zero(),
    );
}

#[test]
fn test_gps_time_v2_sequence_switches() {
    check_gps_time_roundtrip(
        GpsTimeCompressor::default(),
        GpsTimeDecompressor::default(),
        &gps_times_interleaved_sequences(),
    );
}

fn check_point6_gps_time_roundtrip<C, D>(compressor: C, decompressor: D, gps_times: &[GpsTime])
where
    C: LayeredFieldCompressor<Cursor<Vec<u8>>> + Send + Sync + 'static,
    D: LayeredFieldDecompressor<Cursor<Vec<u8>>> + Send + Sync + 'static,
{
    const POINT_SIZE: usize = 30;

    let points: Vec<[u8; POINT_SIZE]> = gps_times
        .iter()
        .map(|gps_time| {
            let mut point = [0u8; POINT_SIZE];
            // return 1 of 1
            point[14] = 0x11;
            gps_time.pack_into(&mut point[22..]);
            point
        })
        .collect();

    let mut record_compressor = LayeredPointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    record_compressor.add_field_compressor(compressor);
    for point in &points {
        record_compressor.compress_next(point).unwrap();
    }
    record_compressor.done().unwrap();

    let compressed_data = record_compressor.into_inner().into_inner();
    let mut record_decompressor = LayeredPointRecordDecompressor::new(Cursor::new(compressed_data));
    record_decompressor.add_field_decompressor(decompressor);

    let mut point = [0u8; POINT_SIZE];
    for expected in &points {
        record_decompressor.decompress_next(&mut point).unwrap();
        assert_eq!(*expected, point);
    }
}

#[test]
fn test_point6_v3_gps_time_sequence_switches() {
    check_point6_gps_time_roundtrip(
        crate::las::v3::LasPoint6Compressor::default(),
        crate::las::v3::LasPoint6Decompressor::default(),
        &gps_times_interleaved_sequences(),
    );
}

#[test]
fn test_point6_v4_gps_time_sequence_switches() {
    check_point6_gps_time_roundtrip(
        crate::las::v4::LasPoint6Compressor::default(),
        crate::las::v4::LasPoint6Decompressor::default(),
        &gps_times_interleaved_sequences(),
    );
}

/// A valid arithmetic stream always starts with a value below the
/// decoder's initial length (`0xFFFF_FFFF`). A malformed chunk whose
/// first 4 bytes are all `0xFF` must be reported as an error,
/// not lead to an out of bounds panic later in `decode_symbol`.
#[test]
fn test_layered_init_bytes_not_below_length() {
    use crate::las::v3::{LasPoint6Compressor, LasPoint6Decompressor};

    const POINT_SIZE: usize = 30;
    const NUM_POINTS: usize = 50;

    let mut compressor = LayeredPointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(LasPoint6Compressor::default());
    let mut point = [0u8; POINT_SIZE];
    for i in 0..NUM_POINTS {
        for (j, byte) in point.iter_mut().enumerate() {
            *byte = (i * 31 + j * 7) as u8;
        }
        compressor.compress_next(&point).unwrap();
    }
    compressor.done().unwrap();
    let mut chunk = compressor.into_inner().into_inner();

    // first point, point count, then the 9 layer sizes of point6
    let first_layer_start = POINT_SIZE + 4 + 9 * 4;
    chunk[first_layer_start..first_layer_start + 4].fill(0xFF);

    let mut decompressor = LayeredPointRecordDecompressor::new(Cursor::new(chunk));
    decompressor.add_field_decompressor(LasPoint6Decompressor::default());
    let result = (0..NUM_POINTS).try_for_each(|_| decompressor.decompress_next(&mut point));
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
}

/// The number of chunks of the chunk table comes from the data,
/// memory must not be reserved for chunks that are not there.
#[test]
fn test_chunk_table_huge_number_of_chunks() {
    use crate::laszip::{ChunkTable, ChunkTableEntry};

    let mut chunk_table = ChunkTable::default();
    for _ in 0..3 {
        chunk_table.push(ChunkTableEntry {
            point_count: 50_000,
            byte_count: 1234,
        });
    }
    let mut data = Vec::<u8>::new();
    chunk_table.write(&mut data, false).unwrap();
    // version, then number of chunks
    data[4..8].copy_from_slice(&u32::MAX.to_le_bytes());

    let err = ChunkTable::read(&mut Cursor::new(data), false).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
}

/// The size of each layer comes from the data,
/// memory must not be allocated for bytes that are not there.
#[test]
fn test_layered_huge_layer_size() {
    use crate::las::v3::{LasPoint6Compressor, LasPoint6Decompressor};

    const POINT_SIZE: usize = 30;
    const NUM_POINTS: usize = 50;

    let mut compressor = LayeredPointRecordCompressor::new(Cursor::new(Vec::<u8>::new()));
    compressor.add_field_compressor(LasPoint6Compressor::default());
    let mut point = [0u8; POINT_SIZE];
    for i in 0..NUM_POINTS {
        for (j, byte) in point.iter_mut().enumerate() {
            *byte = (i * 31 + j * 7) as u8;
        }
        compressor.compress_next(&point).unwrap();
    }
    compressor.done().unwrap();
    let mut chunk = compressor.into_inner().into_inner();

    // first point, point count, then the first layer size
    let first_layer_size_start = POINT_SIZE + 4;
    chunk[first_layer_size_start..first_layer_size_start + 4]
        .copy_from_slice(&u32::MAX.to_le_bytes());

    let mut decompressor = LayeredPointRecordDecompressor::new(Cursor::new(chunk));
    decompressor.add_field_decompressor(LasPoint6Decompressor::default());
    let err = decompressor.decompress_next(&mut point).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
}

/// The items describe the point layout (de)compressors use, they must be consistent:
/// a point is between 1 and `u16::MAX` bytes (decompressors allocate memory according
/// to the size of the items), and each item has the size of its type.
#[test]
fn test_laz_vlr_invalid_items() {
    use crate::laszip::{LazItemRecordBuilder, LazItemType, LazVlr};
    use crate::LasZipError;

    let read_back = |items| {
        let mut data = Vec::<u8>::new();
        LazVlr::from_laz_items(items).write_to(&mut data).unwrap();
        LazVlr::read_from(data.as_slice())
    };

    let items = LazItemRecordBuilder::new()
        .add_item(LazItemType::Byte(40_000))
        .add_item(LazItemType::Byte(40_000))
        .build();
    let result = read_back(items);
    assert!(
        matches!(result, Err(LasZipError::InvalidPointSize(80_000))),
        "{result:?}"
    );

    // The number of items (u16) is at offset 32, patch it to 0
    let mut data = Vec::<u8>::new();
    let items = LazItemRecordBuilder::default_for_point_format_id(0, 0).unwrap();
    LazVlr::from_laz_items(items).write_to(&mut data).unwrap();
    data.truncate(34);
    data[32..34].copy_from_slice(&0u16.to_le_bytes());
    let result = LazVlr::read_from(data.as_slice());
    assert!(
        matches!(result, Err(LasZipError::InvalidPointSize(0))),
        "{result:?}"
    );

    // Point10 is 20 bytes
    let mut items = LazItemRecordBuilder::new()
        .add_item(LazItemType::Point10)
        .build();
    items[0].size = 21;
    let result = read_back(items);
    assert!(
        matches!(
            result,
            Err(LasZipError::InvalidLazItemSize(LazItemType::Point10, 21))
        ),
        "{result:?}"
    );
}

/// The chunk size comes from the vlr, which comes from the data.
#[cfg(feature = "parallel")]
#[test]
fn test_par_huge_chunk_size() {
    use crate::laszip::LazVlrBuilder;
    use crate::{LasZipCompressor, LasZipError, ParLasZipCompressor, ParLasZipDecompressor};

    fn is_out_of_memory<T>(result: crate::Result<T>) -> bool {
        matches!(result, Err(LasZipError::IoError(e)) if e.kind() == std::io::ErrorKind::OutOfMemory)
    }

    let vlr = LazVlrBuilder::default()
        .with_point_format(0, 0)
        .unwrap()
        .build();
    let point_size = vlr.items_size() as usize;
    let mut compressor = LasZipCompressor::new(Cursor::new(Vec::<u8>::new()), vlr).unwrap();
    compressor
        .compress_many(&vec![0u8; 10 * point_size])
        .unwrap();
    compressor.done().unwrap();
    let data = compressor.into_inner().into_inner();

    let hostile_vlr = LazVlrBuilder::default()
        .with_point_format(0, 0)
        .unwrap()
        .with_fixed_chunk_size(u32::MAX - 1)
        .build();

    let result = ParLasZipCompressor::new(Cursor::new(Vec::<u8>::new()), hostile_vlr.clone());
    assert!(is_out_of_memory(result));

    let mut point = vec![0u8; point_size];
    let mut decompressor =
        ParLasZipDecompressor::new(Cursor::new(data.clone()), hostile_vlr.clone()).unwrap();
    assert!(is_out_of_memory(decompressor.decompress_many(&mut point)));

    let mut decompressor = ParLasZipDecompressor::new(Cursor::new(data), hostile_vlr).unwrap();
    assert!(is_out_of_memory(decompressor.seek(0)));
}

/// Returns the vlr (fixed-size chunks of 100 points), the points,
/// and the compressed data (offset to the chunk table, chunks, chunk table)
#[cfg(feature = "parallel")]
fn par_test_data(num_points: usize) -> (crate::LazVlr, Vec<u8>, Vec<u8>) {
    use crate::laszip::LazVlrBuilder;

    let vlr = LazVlrBuilder::default()
        .with_point_format(0, 0)
        .unwrap()
        .with_fixed_chunk_size(100)
        .build();
    let points = (0..num_points * vlr.items_size() as usize)
        .map(|i| (i * 7 % 251) as u8)
        .collect::<Vec<u8>>();
    let mut compressed = Cursor::new(Vec::<u8>::new());
    crate::par_compress_buffer(&mut compressed, &points, &vlr).unwrap();
    (vlr, points, compressed.into_inner())
}

#[cfg(feature = "parallel")]
#[test]
fn test_par_decompress_buffer() {
    // 250 points: the last chunk is not full
    for num_points in [200, 250] {
        let (vlr, points, compressed) = par_test_data(num_points);
        let mut decompressed = vec![0u8; points.len()];
        crate::par_decompress_buffer(&compressed, &mut decompressed, &vlr).unwrap();
        assert!(decompressed == points);

        // As many points as the output can hold are decompressed
        let mut decompressed = vec![0u8; 150 * vlr.items_size() as usize];
        crate::par_decompress_buffer(&compressed, &mut decompressed, &vlr).unwrap();
        assert!(decompressed == points[..decompressed.len()]);
    }
}

/// The chunk table comes from the data, its counts must be checked against the buffers.
#[cfg(feature = "parallel")]
#[test]
fn test_par_decompress_chunk_table_bigger_than_data() {
    use crate::laszip::{ChunkTable, ChunkTableEntry};
    use crate::LasZipError;

    fn is_unexpected_eof(result: crate::Result<()>) -> bool {
        matches!(result, Err(LasZipError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof)
    }

    let (vlr, points, compressed) = par_test_data(250);
    let mut decompressed = vec![0u8; points.len()];

    // Rewrite the chunk table, with a chunk bigger than the data
    let chunk_table = ChunkTable::read_from(Cursor::new(&compressed), &vlr).unwrap();
    let offset = i64::from_le_bytes(compressed[..8].try_into().unwrap()) as usize;
    let mut hostile_table = ChunkTable::default();
    for (i, entry) in chunk_table.as_ref().iter().enumerate() {
        let byte_count = if i == 1 { 1 << 30 } else { entry.byte_count };
        hostile_table.push(ChunkTableEntry {
            byte_count,
            ..*entry
        });
    }
    let mut hostile = compressed[..offset].to_vec();
    hostile_table.write_to(&mut hostile, &vlr).unwrap();
    let result = crate::par_decompress_buffer(&hostile, &mut decompressed, &vlr);
    assert!(is_unexpected_eof(result));

    // Chunk table given directly, with counts that overflow
    let hostile_table = [ChunkTableEntry {
        point_count: u64::MAX,
        byte_count: u64::MAX,
    }];
    let result = crate::par_decompress(&compressed[8..], &mut decompressed, &vlr, &hostile_table);
    assert!(is_unexpected_eof(result));
}
