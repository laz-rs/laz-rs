//! Defines the Point Format 6 ands different version of compressors and decompressors

use crate::las::gps::GpsTime;
use crate::packers::Packable;

fn u32_zero_bit_0(n: u32) -> u32 {
    n & 0xFFFFFFFE
}

pub struct DecompressionSelector(u32);

impl DecompressionSelector {
    pub fn decompress_all() -> Self {
        Self { 0: 0xFFFFFFFF }
    }

    pub fn channel_returns_xy_requested(&self) -> bool {
        self.is_set(0x00000000)
    }

    pub fn z_requested(&self) -> bool {
        self.is_set(0x00000001)
    }

    pub fn classification_requested(&self) -> bool {
        self.is_set(0x00000002)
    }

    pub fn flags_requested(&self) -> bool {
        self.is_set(0x00000004)
    }

    pub fn intensity_requested(&self) -> bool {
        self.is_set(0x00000008)
    }

    pub fn scan_angle_requested(&self) -> bool {
        self.is_set(0x00000010)
    }

    pub fn user_data_requested(&self) -> bool {
        self.is_set(0x00000020)
    }

    pub fn point_source_requested(&self) -> bool {
        self.is_set(0x00000040)
    }

    pub fn gps_time_requested(&self) -> bool {
        self.is_set(0x00000080)
    }

    fn is_set(&self, mask: u32) -> bool {
        self.0 & mask != 0
    }
}

//TODO cleanup
pub trait LasPoint6 {
    // Non mutable accessors
    fn x(&self) -> i32;
    fn y(&self) -> i32;
    fn z(&self) -> i32;
    fn intensity(&self) -> u16;

    // return_number & number_of_returns_of_given_pulse are packed into
    // bit_fields
    fn bit_fields(&self) -> u8;
    fn return_number(&self) -> u8;
    // 4 bits
    fn number_of_returns_of_given_pulse(&self) -> u8; // 4bits

    fn flags(&self) -> u8;
    // all theses values are packed into the same byte
    fn classification_flags(&self) -> u8;
    // 4 bits
    fn scanner_channel(&self) -> u8;
    // 2 bits
    fn scan_direction_flag(&self) -> bool;
    fn edge_of_flight_line(&self) -> bool;

    fn classification(&self) -> u8;
    fn scan_angle_rank(&self) -> u16;
    fn user_data(&self) -> u8;
    fn point_source_id(&self) -> u16;
    fn gps_time(&self) -> f64;

    // Mutable accessors

    fn set_x(&mut self, new_val: i32);
    fn set_y(&mut self, new_val: i32);
    fn set_z(&mut self, new_val: i32);
    fn set_intensity(&mut self, new_val: u16);

    fn set_bit_fields(&mut self, new_val: u8);
    fn set_flags(&mut self, new_val: u8);

    fn set_number_of_returns(&mut self, new_val: u8);
    fn set_return_number(&mut self, new_val: u8);

    fn set_scanner_channel(&mut self, new_val: u8);

    fn set_classification(&mut self, new_val: u8);
    fn set_scan_angle_rank(&mut self, new_val: u16);
    fn set_user_data(&mut self, new_val: u8);
    fn set_point_source_id(&mut self, new_val: u16);
    fn set_gps_time(&mut self, new_val: f64);
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Point6 {
    x: i32,
    y: i32,
    z: i32,

    bit_fields: u8,
    flags: u8,

    intensity: u16,
    classification: u8,
    scan_angle_rank: u16,
    user_data: u8,
    point_source_id: u16,
    gps_time: f64,

    // compressed LASzip 1.4 points only
    // Not part of data written
    gps_time_change: bool,
}

impl LasPoint6 for Point6 {
    fn x(&self) -> i32 {
        self.x
    }

    fn y(&self) -> i32 {
        self.y
    }

    fn z(&self) -> i32 {
        self.z
    }

    fn intensity(&self) -> u16 {
        self.intensity
    }

    fn bit_fields(&self) -> u8 {
        self.bit_fields
    }

    fn return_number(&self) -> u8 {
        self.bit_fields & 0b00001111
    }

    fn number_of_returns_of_given_pulse(&self) -> u8 {
        (self.bit_fields & 0b11110000) >> 4
    }

    fn flags(&self) -> u8 {
        self.flags
    }

    fn classification_flags(&self) -> u8 {
        self.flags & 0b00001111
    }

    fn scanner_channel(&self) -> u8 {
        (self.flags & 0b00110000) >> 4
    }

    fn scan_direction_flag(&self) -> bool {
        self.flags & 0b01000000 != 0
    }

    fn edge_of_flight_line(&self) -> bool {
        self.flags & 0b10000000 != 0
    }

    fn classification(&self) -> u8 {
        self.classification
    }

    fn scan_angle_rank(&self) -> u16 {
        self.scan_angle_rank
    }

    fn user_data(&self) -> u8 {
        self.user_data
    }

    fn point_source_id(&self) -> u16 {
        self.point_source_id
    }

    fn gps_time(&self) -> f64 {
        self.gps_time
    }

    fn set_x(&mut self, new_val: i32) {
        self.x = new_val;
    }

    fn set_y(&mut self, new_val: i32) {
        self.y = new_val;
    }

    fn set_z(&mut self, new_val: i32) {
        self.z = new_val;
    }

    fn set_intensity(&mut self, new_val: u16) {
        self.intensity = new_val;
    }

    fn set_bit_fields(&mut self, new_val: u8) {
        self.bit_fields = new_val;
    }

    fn set_flags(&mut self, new_val: u8) {
        self.flags = new_val;
    }

    fn set_number_of_returns(&mut self, new_val: u8) {
        self.bit_fields ^= self.bit_fields & 0b11110000;
        self.bit_fields |= (new_val << 4) & 0b11110000;
    }

    fn set_return_number(&mut self, new_val: u8) {
        self.bit_fields ^= self.bit_fields & 0b00001111;
        self.bit_fields |= new_val & 0b00001111;
    }

    fn set_scanner_channel(&mut self, new_val: u8) {
        self.flags ^= self.flags & 0b00110000;
        self.flags |= (new_val << 4) & 0b00110000;
    }

    fn set_classification(&mut self, new_val: u8) {
        self.classification = new_val;
    }

    fn set_scan_angle_rank(&mut self, new_val: u16) {
        self.scan_angle_rank = new_val;
    }

    fn set_user_data(&mut self, new_val: u8) {
        self.user_data = new_val;
    }

    fn set_point_source_id(&mut self, new_val: u16) {
        self.point_source_id = new_val;
    }

    fn set_gps_time(&mut self, new_val: f64) {
        self.gps_time = new_val;
    }
}

impl Default for Point6 {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            z: 0,
            bit_fields: 0,
            flags: 0,
            classification: 0,
            scan_angle_rank: 0,
            user_data: 0,
            point_source_id: 0,
            gps_time: 0.0,
            intensity: 0,
            gps_time_change: false,
        }
    }
}

impl Point6 {
    pub const SIZE: usize = 30;
}

impl Packable for Point6 {

    fn unpack_from(input: &[u8]) -> Self {
        if input.len() < Self::SIZE {
            panic!("Point6::unpack_from expected buffer of 30 bytes");
        } else {
            Point6 {
                x: i32::unpack_from(&input[..4]),
                y: i32::unpack_from(&input[4..8]),
                z: i32::unpack_from(&input[8..12]),
                bit_fields: u8::unpack_from(&input[14..15]),
                flags: u8::unpack_from(&input[15..16]),
                intensity: u16::unpack_from(&input[12..14]),
                classification: u8::unpack_from(&input[16..17]),
                scan_angle_rank: u16::unpack_from(&input[18..20]),
                user_data: u8::unpack_from(&input[17..18]),
                point_source_id: u16::unpack_from(&input[20..22]),
                gps_time: f64::from(GpsTime::unpack_from(&input[22..30])),
                gps_time_change: false,
            }
        }
    }

    fn pack_into(&self, output: &mut [u8]) {
        if output.len() < Self::SIZE {
            panic!("Point6::pack_into expected buffer of 30 bytes");
        } else {
            self.x.pack_into(&mut output[..4]);
            self.y.pack_into(&mut output[4..8]);
            self.z.pack_into(&mut output[8..12]);
            self.intensity.pack_into(&mut output[12..14]);
            self.bit_fields.pack_into(&mut output[14..15]);
            self.flags.pack_into(&mut output[15..16]);
            self.classification.pack_into(&mut output[16..17]);
            self.user_data.pack_into(&mut output[17..18]);
            self.scan_angle_rank.pack_into(&mut output[18..20]);
            self.point_source_id.pack_into(&mut output[20..22]);
            GpsTime::from(self.gps_time).pack_into(&mut output[22..30]);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::las::point6::{LasPoint6, Point6};

    #[test]
    fn test_bit_fields_get_set() {
        let mut p = Point6::default();
        assert_eq!(p.bit_fields, 0);
        p.set_number_of_returns(1);
        p.set_return_number(1);
        assert_eq!(p.bit_fields, 17);

        p.set_number_of_returns(2);
        assert_eq!(p.number_of_returns_of_given_pulse(), 2);
        assert_eq!(p.bit_fields, 33);
    }
}

pub mod v3 {
    use std::io::{Cursor, Read, Seek, Write};

    use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

    use crate::compressors::{
        IntegerCompressor, IntegerCompressorBuilder, DEFAULT_COMPRESS_CONTEXTS,
    };
    use crate::decoders::ArithmeticDecoder;
    use crate::decompressors::{
        IntegerDecompressor, IntegerDecompressorBuilder, DEFAULT_DECOMPRESS_CONTEXTS,
    };
    use crate::encoders::ArithmeticEncoder;
    use crate::las::gps::{GpsTime, LasGpsTime};
    use crate::las::point6::{u32_zero_bit_0, DecompressionSelector, LasPoint6, Point6};
    use crate::las::utils::{copy_bytes_into_decoder, copy_encoder_content_to, read_and_unpack, StreamingMedian, NUMBER_RETURN_LEVEL_8CT, NUMBER_RETURN_MAP_6CTX, i32_quantize};
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::packers::Packable;
    use crate::record::{LayeredFieldCompressor, LayeredFieldDecompressor};

    fn compute_last_point_return(last_point: &Point6) -> usize {
        // Create single (3) / first (1) / last (2) / intermediate (0) context from last point return
        let mut lpr = if last_point.return_number() == 1 {
            1
        } else {
            0
        };
        lpr += if last_point.return_number() >= last_point.number_of_returns_of_given_pulse() {
            2
        } else {
            0
        };
        // Add info whether the GPS time changed in the last return to the context
        lpr += if last_point.gps_time_change { 4 } else { 0 };
        lpr
    }

    struct Point6Models {
        changed_values: Vec<ArithmeticModel>,
        //8
        scanner_channel: ArithmeticModel,
        number_of_returns: Vec<Option<ArithmeticModel>>,
        // 16
        return_number: Vec<Option<ArithmeticModel>>,
        // 16
        return_number_gps_same: ArithmeticModel,
        classification: Vec<Option<ArithmeticModel>>,
        // 64
        classification_flags: Vec<Option<ArithmeticModel>>,
        // 64
        user_data: Vec<Option<ArithmeticModel>>,
        // 64
        gps_time_multi: ArithmeticModel,
        gps_time_no_diff: ArithmeticModel,
    }

    impl Default for Point6Models {
        fn default() -> Self {
            Self {
                changed_values: (0..8)
                    .into_iter()
                    .map(|_| ArithmeticModelBuilder::new(128).build())
                    .collect(),
                scanner_channel: ArithmeticModelBuilder::new(3).build(),
                number_of_returns: (0..16).into_iter().map(|_| None).collect(),
                return_number: (0..16).into_iter().map(|_| None).collect(),
                return_number_gps_same: ArithmeticModelBuilder::new(13).build(),
                classification: (0..64).into_iter().map(|_| None).collect(),
                classification_flags: (0..64).into_iter().map(|_| None).collect(),
                user_data: (0..64).into_iter().map(|_| None).collect(),
                gps_time_multi: ArithmeticModelBuilder::new(LASZIP_GPS_TIME_MULTI_TOTAL as u32)
                    .build(),
                gps_time_no_diff: ArithmeticModelBuilder::new(5).build(),
            }
        }
    }

    /// Holds a bunch of boolean flags
    ///
    /// In a decompression context the flags are set to true
    /// if the user asked for decompression, and that the corresponding layer is not empty
    /// (either because the user didn't wanted it to be compressed or all the data were
    /// the same thus 'compressed' to 0 bytes)
    ///
    /// In a compression context, the flags are set to true if the corresponding field
    /// has changed at least once.
    #[derive(Debug)]
    struct Point6FieldFlags {
        z: bool,
        classification: bool,
        flags: bool,
        intensity: bool,
        scan_angle: bool,
        user_data: bool,
        point_source: bool,
        gps_time: bool,
    }

    impl Default for Point6FieldFlags {
        fn default() -> Self {
            Self {
                z: false,
                classification: false,
                flags: false,
                intensity: false,
                scan_angle: false,
                user_data: false,
                point_source: false,
                gps_time: false,
            }
        }
    }

    struct Point6Decompressors {
        dx: IntegerDecompressor,
        dy: IntegerDecompressor,
        z: IntegerDecompressor,
        intensity: IntegerDecompressor,
        scan_angle: IntegerDecompressor,
        source_id: IntegerDecompressor,
        gps_time: IntegerDecompressor,
    }

    impl Default for Point6Decompressors {
        fn default() -> Self {
            Self {
                dx: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(2)
                    .build_initialized(),
                dy: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(22)
                    .build_initialized(),
                z: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(20)
                    .build_initialized(),
                intensity: IntegerDecompressorBuilder::new()
                    .bits(16)
                    .contexts(4)
                    .build_initialized(),
                scan_angle: IntegerDecompressorBuilder::new()
                    .bits(16)
                    .contexts(2)
                    .build_initialized(),
                source_id: IntegerDecompressorBuilder::new()
                    .bits(16)
                    .build_initialized(),
                gps_time: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(9)
                    .build_initialized(),
            }
        }
    }

    struct Point6DecompressionContext {
        unused: bool,

        last_point: Point6,
        last_intensities: [u16; 8],
        last_x_diff_median5: [StreamingMedian<i32>; 12],
        last_y_diff_median5: [StreamingMedian<i32>; 12],
        last_z: [i32; 8],

        models: Point6Models,
        decompressors: Point6Decompressors,

        gps_sequences: GpsTimeSequences,
    }

    impl Point6DecompressionContext {
        fn from_last_point(point: &Point6) -> Self {
            let mut me = Self {
                unused: false,
                last_point: *point,
                last_intensities: [point.intensity; 8],
                last_x_diff_median5: [StreamingMedian::<i32>::new(); 12],
                last_y_diff_median5: [StreamingMedian::<i32>::new(); 12],
                last_z: [point.z; 8],
                models: Point6Models::default(),
                decompressors: Point6Decompressors::default(),
                gps_sequences: GpsTimeSequences::from_point(point),
            };
            me.last_point.gps_time_change = false;
            me
        }
    }

    const LASZIP_GPS_TIME_MULTI: i32 = 500;
    const LASZIP_GPS_TIME_MULTI_MINUS: i32 = -10;
    const LASZIP_GPS_TIME_MULTI_CODE_FULL: i32 =
        (LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 1);

    const LASZIP_GPS_TIME_MULTI_TOTAL: i32 =
        (LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 5);

    #[derive(Clone)]
    struct GpsTimeSequences {
        last: usize,
        next: usize,
        last_gps_times: [GpsTime; 4],
        last_gps_diffs: [i32; 4],
        multi_extreme_counter: [i32; 4],
    }

    impl Default for GpsTimeSequences {
        fn default() -> Self {
            Self {
                last: 0,
                next: 0,
                last_gps_times: [GpsTime::default(); 4],
                last_gps_diffs: [0; 4],
                multi_extreme_counter: [0; 4],
            }
        }
    }

    impl GpsTimeSequences {
        fn from_point(point: &Point6) -> Self {
            let mut me = Self::default();
            me.last_gps_times[0] = GpsTime::from(point.gps_time);
            me
        }
    }

    // Each layer has its own decoder that holds the compressed data
    // to be decoded
    struct Point6Decoders {
        channel_returns_xy: ArithmeticDecoder<Cursor<Vec<u8>>>,
        z: ArithmeticDecoder<Cursor<Vec<u8>>>,
        classification: ArithmeticDecoder<Cursor<Vec<u8>>>,
        flags: ArithmeticDecoder<Cursor<Vec<u8>>>,
        intensity: ArithmeticDecoder<Cursor<Vec<u8>>>,
        scan_angle: ArithmeticDecoder<Cursor<Vec<u8>>>,
        user_data: ArithmeticDecoder<Cursor<Vec<u8>>>,
        point_source: ArithmeticDecoder<Cursor<Vec<u8>>>,
        gps_time: ArithmeticDecoder<Cursor<Vec<u8>>>,
    }

    impl Default for Point6Decoders {
        fn default() -> Self {
            Self {
                channel_returns_xy: ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new())),
                z: ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new())),
                classification: ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new())),
                flags: ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new())),
                intensity: ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new())),
                scan_angle: ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new())),
                user_data: ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new())),
                point_source: ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new())),
                gps_time: ArithmeticDecoder::new(Cursor::new(Vec::<u8>::new())),
            }
        }
    }

    /// Simple struct to store the size  of each layers
    #[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
    struct LayerSizes {
        channel_returns_xy: usize,
        z: usize,
        classification: usize,
        flags: usize,
        intensity: usize,
        scan_angle: usize,
        user_data: usize,
        point_source: usize,
        gps_time: usize,
    }

    impl LayerSizes {
        fn read_from<R: Read>(src: &mut R) -> std::io::Result<Self> {
            let channel_returns_xy = src.read_u32::<LittleEndian>()? as usize;
            let z = src.read_u32::<LittleEndian>()? as usize;
            let classification = src.read_u32::<LittleEndian>()? as usize;
            let flags = src.read_u32::<LittleEndian>()? as usize;
            let intensity = src.read_u32::<LittleEndian>()? as usize;
            let scan_angle = src.read_u32::<LittleEndian>()? as usize;
            let user_data = src.read_u32::<LittleEndian>()? as usize;
            let point_source = src.read_u32::<LittleEndian>()? as usize;
            let gps_time = src.read_u32::<LittleEndian>()? as usize;

            Ok(Self {
                channel_returns_xy,
                z,
                classification,
                flags,
                intensity,
                scan_angle,
                user_data,
                point_source,
                gps_time,
            })
        }

        fn write_to<W: Write>(&self, dst: &mut W) -> std::io::Result<()> {
            dst.write_u32::<LittleEndian>(self.channel_returns_xy as u32)?;
            dst.write_u32::<LittleEndian>(self.z as u32)?;
            dst.write_u32::<LittleEndian>(self.classification as u32)?;
            dst.write_u32::<LittleEndian>(self.flags as u32)?;
            dst.write_u32::<LittleEndian>(self.intensity as u32)?;
            dst.write_u32::<LittleEndian>(self.scan_angle as u32)?;
            dst.write_u32::<LittleEndian>(self.user_data as u32)?;
            dst.write_u32::<LittleEndian>(self.point_source as u32)?;
            dst.write_u32::<LittleEndian>(self.gps_time as u32)?;
            Ok(())
        }
    }

    pub struct LasPoint6Decompressor {
        decoders: Point6Decoders,

        layers_sizes: LayerSizes,
        should_decompress: Point6FieldFlags,
        decompression_selector: DecompressionSelector,

        current_context: usize,
        contexts: [Point6DecompressionContext; 4],
    }

    impl LasPoint6Decompressor {
        pub fn new() -> Self {
            Self::selective(DecompressionSelector::decompress_all())
        }

        pub fn selective(selector: DecompressionSelector) -> Self {
            let p = Point6::default();
            Self {
                decoders: Point6Decoders::default(),
                layers_sizes: Default::default(),
                should_decompress: Point6FieldFlags::default(),
                decompression_selector: selector,
                current_context: 0,
                contexts: [
                    Point6DecompressionContext::from_last_point(&p),
                    Point6DecompressionContext::from_last_point(&p),
                    Point6DecompressionContext::from_last_point(&p),
                    Point6DecompressionContext::from_last_point(&p),
                ],
            }
        }

        fn read_gps_time(&mut self) -> std::io::Result<()> {
            let the_context = &mut self.contexts[self.current_context];

            let mut multi: i32;
            if the_context.gps_sequences.last_gps_diffs[the_context.gps_sequences.last] == 0 {
                multi = self
                    .decoders
                    .gps_time
                    .decode_symbol(&mut the_context.models.gps_time_no_diff)?
                    as i32;
                if multi == 0 {
                    // The difference can be represented with 32 bits
                    the_context.gps_sequences.last_gps_diffs[the_context.gps_sequences.last] =
                        the_context.decompressors.gps_time.decompress(
                            &mut self.decoders.gps_time,
                            0,
                            0,
                        )?;
                    the_context.gps_sequences.last_gps_times[the_context.gps_sequences.last] +=
                        i64::from(
                            the_context.gps_sequences.last_gps_diffs
                                [the_context.gps_sequences.last],
                        );
                    the_context.gps_sequences.multi_extreme_counter
                        [the_context.gps_sequences.last] = 0;
                } else if multi == 1 {
                    // Difference is huge
                    the_context.gps_sequences.next = (the_context.gps_sequences.next + 1) & 3;
                    let last_gps_time = the_context.gps_sequences.last_gps_times
                        [the_context.gps_sequences.last]
                        .value;
                    let next_gps_time = &mut the_context.gps_sequences.last_gps_times
                        [the_context.gps_sequences.next];

                    next_gps_time.value = the_context.decompressors.gps_time.decompress(
                        &mut self.decoders.gps_time,
                        (last_gps_time >> 32) as i32,
                        8,
                    )? as i64;
                    next_gps_time.value <<= 32;
                    next_gps_time.value |= self.decoders.gps_time.read_int()? as i64;
                    the_context.gps_sequences.last = the_context.gps_sequences.next;
                    the_context.gps_sequences.last_gps_diffs[the_context.gps_sequences.last] = 0;
                    the_context.gps_sequences.multi_extreme_counter
                        [the_context.gps_sequences.last] = 0;
                } else {
                    // We switch to another sequence
                    the_context.gps_sequences.last =
                        (the_context.gps_sequences.last + multi as usize - 1) & 3;
                    self.read_gps_time()?;
                }
            } else {
                multi = self
                    .decoders
                    .gps_time
                    .decode_symbol(&mut the_context.models.gps_time_multi)?
                    as i32;
                if multi == 1 {
                    the_context.gps_sequences.last_gps_times[the_context.gps_sequences.last] +=
                        the_context.decompressors.gps_time.decompress(
                            &mut self.decoders.gps_time,
                            the_context.gps_sequences.last_gps_diffs
                                [the_context.gps_sequences.last],
                            1,
                        )? as i64;
                    the_context.gps_sequences.multi_extreme_counter
                        [the_context.gps_sequences.last] = 0;
                } else if multi < LASZIP_GPS_TIME_MULTI_CODE_FULL {
                    let gps_time_diff: i32;
                    if multi == 0 {
                        gps_time_diff = the_context.decompressors.gps_time.decompress(
                            &mut self.decoders.gps_time,
                            0,
                            7,
                        )?;
                        the_context.gps_sequences.multi_extreme_counter
                            [the_context.gps_sequences.last] += 1;
                        if the_context.gps_sequences.multi_extreme_counter
                            [the_context.gps_sequences.last]
                            > 3
                        {
                            the_context.gps_sequences.last_gps_diffs
                                [the_context.gps_sequences.last] = gps_time_diff;
                            the_context.gps_sequences.multi_extreme_counter
                                [the_context.gps_sequences.last] = 0;
                        }
                    } else if multi < LASZIP_GPS_TIME_MULTI {
                        if multi < 10 {
                            gps_time_diff = the_context.decompressors.gps_time.decompress(
                                &mut self.decoders.gps_time,
                                multi.wrapping_mul(
                                    the_context.gps_sequences.last_gps_diffs
                                        [the_context.gps_sequences.last],
                                ),
                                2,
                            )?;
                        } else {
                            gps_time_diff = the_context.decompressors.gps_time.decompress(
                                &mut self.decoders.gps_time,
                                multi.wrapping_mul(
                                    the_context.gps_sequences.last_gps_diffs
                                        [the_context.gps_sequences.last],
                                ),
                                3,
                            )?;
                        }
                    } else if multi == LASZIP_GPS_TIME_MULTI {
                        gps_time_diff = the_context.decompressors.gps_time.decompress(
                            &mut self.decoders.gps_time,
                            LASZIP_GPS_TIME_MULTI
                                * the_context.gps_sequences.last_gps_diffs
                                    [the_context.gps_sequences.last],
                            4,
                        )?;
                        the_context.gps_sequences.multi_extreme_counter
                            [the_context.gps_sequences.last] += 1;
                        if the_context.gps_sequences.multi_extreme_counter
                            [the_context.gps_sequences.last]
                            > 3
                        {
                            the_context.gps_sequences.last_gps_diffs
                                [the_context.gps_sequences.last] = gps_time_diff;
                            the_context.gps_sequences.multi_extreme_counter
                                [the_context.gps_sequences.last] = 0;
                        }
                    } else {
                        multi = LASZIP_GPS_TIME_MULTI - multi;
                        if multi > LASZIP_GPS_TIME_MULTI_MINUS {
                            gps_time_diff = the_context.decompressors.gps_time.decompress(
                                &mut self.decoders.gps_time,
                                multi.wrapping_mul(
                                    the_context.gps_sequences.last_gps_diffs
                                        [the_context.gps_sequences.last],
                                ),
                                5,
                            )?;
                        } else {
                            gps_time_diff = the_context.decompressors.gps_time.decompress(
                                &mut self.decoders.gps_time,
                                LASZIP_GPS_TIME_MULTI_MINUS
                                    * the_context.gps_sequences.last_gps_diffs
                                        [the_context.gps_sequences.last],
                                6,
                            )?;
                            the_context.gps_sequences.multi_extreme_counter
                                [the_context.gps_sequences.last] += 1;
                            if the_context.gps_sequences.multi_extreme_counter
                                [the_context.gps_sequences.last]
                                > 3
                            {
                                the_context.gps_sequences.last_gps_diffs
                                    [the_context.gps_sequences.last] = gps_time_diff;
                                the_context.gps_sequences.multi_extreme_counter
                                    [the_context.gps_sequences.last] = 0;
                            }
                        }
                    }
                    the_context.gps_sequences.last_gps_times[the_context.gps_sequences.last] +=
                        i64::from(gps_time_diff);
                } else if multi == LASZIP_GPS_TIME_MULTI_CODE_FULL {
                    the_context.gps_sequences.next = (the_context.gps_sequences.next + 1) & 3;
                    the_context.gps_sequences.last_gps_times[the_context.gps_sequences.next] =
                        GpsTime::from(
                            the_context.decompressors.gps_time.decompress(
                                &mut self.decoders.gps_time,
                                (the_context.gps_sequences.last_gps_times
                                    [the_context.gps_sequences.last]
                                    .value
                                    >> 32) as i32,
                                8,
                            )? as i64,
                        );
                    the_context.gps_sequences.last_gps_times[the_context.gps_sequences.next]
                        .value <<= 32;
                    the_context.gps_sequences.last_gps_times[the_context.gps_sequences.next]
                        .value |= self.decoders.gps_time.read_int()? as i64;
                    the_context.gps_sequences.last = the_context.gps_sequences.next;
                    the_context.gps_sequences.last_gps_diffs[the_context.gps_sequences.last] = 0;
                    the_context.gps_sequences.multi_extreme_counter
                        [the_context.gps_sequences.last] = 0;
                } else if multi >= LASZIP_GPS_TIME_MULTI_CODE_FULL {
                    the_context.gps_sequences.last = (the_context.gps_sequences.last
                        + multi as usize
                        - LASZIP_GPS_TIME_MULTI_CODE_FULL as usize)
                        & 3;
                    self.read_gps_time()?;
                }
            }
            Ok(())
        }
    }

    impl<R: Read + Seek> LayeredFieldDecompressor<R> for LasPoint6Decompressor {
        fn size_of_field(&self) -> usize {
            Point6::SIZE
        }

        fn init_first_point(
            &mut self,
            src: &mut R,
            first_point: &mut [u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            for context in &mut self.contexts {
                context.unused = true;
            }
            let point = read_and_unpack::<_, Point6>(src, first_point)?;
            self.current_context = point.scanner_channel() as usize;
            *context = self.current_context;

            debug_assert!(self.contexts[*context].unused);
            self.contexts[*context] = Point6DecompressionContext::from_last_point(&point);
            Ok(())
        }

        fn decompress_field_with(
            &mut self,
            current_point: &mut [u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            let changed_values = {
                let the_context = &mut self.contexts[self.current_context];
                let last_point = &mut the_context.last_point;

                let lpr = compute_last_point_return(last_point);

                self.decoders
                    .channel_returns_xy
                    .decode_symbol(&mut the_context.models.changed_values[lpr])?
            };

            // Scanner channel changed
            if is_nth_bit_set!(changed_values, 6) {
                let diff = self.decoders.channel_returns_xy.decode_symbol(
                    &mut self.contexts[self.current_context].models.scanner_channel,
                )?;
                let scanner_channel = (self.current_context + diff as usize + 1) % 4; // TODO: num_context const ?

                if self.contexts[scanner_channel as usize].unused {
                    self.contexts[scanner_channel as usize] =
                        Point6DecompressionContext::from_last_point(
                            &self.contexts[self.current_context].last_point,
                        );
                }

                // Switch context to current channel
                self.current_context = scanner_channel;
            }
            *context = self.current_context;

            let point_source_changed = is_nth_bit_set!(changed_values, 5);
            let gps_time_changed = is_nth_bit_set!(changed_values, 4);
            let scan_angle_changed = is_nth_bit_set!(changed_values, 3);

            // Introduce a scope because we borrow &mut self
            // and later self.read_gps(also needs to borrow mut self
            {
                let mut the_context = &mut self.contexts[self.current_context];
                let mut last_point = &mut the_context.last_point;
                last_point.set_scanner_channel(self.current_context as u8);

                // Get last return counts
                let last_n = last_point.number_of_returns_of_given_pulse();
                let last_r = last_point.return_number();

                // If number of returns if different we decompress it
                let n;
                if is_nth_bit_set!(changed_values, 2) {
                    n = self.decoders.channel_returns_xy.decode_symbol(
                        the_context.models.number_of_returns[last_n as usize]
                            .get_or_insert_with(|| ArithmeticModelBuilder::new(16).build()),
                    )?;
                } else {
                    n = last_n as u32;
                }
                last_point.set_number_of_returns(n as u8);

                // how is the return number different
                let r: u32;
                if changed_values & 3 == 0 {
                    r = last_r as u32;
                } else if changed_values & 3 == 1 {
                    r = ((last_r + 1) % 16) as u32;
                } else if changed_values & 3 == 2 {
                    r = ((last_r + 15) % 16) as u32;
                } else {
                    // The return number is bigger than +1 / -1 so we decompress how it is different
                    if gps_time_changed {
                        r = self.decoders.channel_returns_xy.decode_symbol(
                            &mut the_context.models.return_number[last_r as usize]
                                .get_or_insert_with(|| ArithmeticModelBuilder::new(16).build()),
                        )?;
                    } else {
                        let sym = self
                            .decoders
                            .channel_returns_xy
                            .decode_symbol(&mut the_context.models.return_number_gps_same)?;
                        r = (last_r as u32 + (sym + 2)) % 16;
                    }
                }
                last_point.set_return_number(r as u8);

                let m = usize::from(NUMBER_RETURN_MAP_6CTX[n as usize][r as usize]);
                let l = usize::from(NUMBER_RETURN_LEVEL_8CT[n as usize][r as usize]);

                // Create single (3) / first (1) / last (2) / intermediate (0) return context for current point
                let mut cpr = if r == 1 { 2 } else { 0 }; // First ?
                cpr += if r >= n { 1 } else { 0 }; // last

                let mut k_bits: u32;
                let mut median: i32;
                let mut diff: i32;

                // Decompress X
                let idx = usize::from((m << 1) | (gps_time_changed as usize));
                median = the_context.last_x_diff_median5[idx].get();
                diff = the_context.decompressors.dx.decompress(
                    &mut self.decoders.channel_returns_xy,
                    median,
                    if n == 1 { 1 } else { 0 },
                )?;
                last_point.x = last_point.x.wrapping_add(diff);
                the_context.last_x_diff_median5[idx].add(diff);

                // Decompress Y
                let idx = usize::from((m << 1) | (gps_time_changed as usize));
                median = the_context.last_y_diff_median5[idx].get();
                k_bits = the_context.decompressors.dx.k();
                let mut context = if n == 1 { 1 } else { 0 };
                context += if k_bits < 20 {
                    u32_zero_bit_0(k_bits)
                } else {
                    20
                };
                diff = the_context.decompressors.dy.decompress(
                    &mut self.decoders.channel_returns_xy,
                    median,
                    context,
                )?;
                last_point.y = last_point.y.wrapping_add(diff);
                the_context.last_y_diff_median5[idx].add(diff);

                // Decompress Z
                if self.should_decompress.z {
                    k_bits =
                        (the_context.decompressors.dx.k() + the_context.decompressors.dy.k()) / 2;
                    let mut context = if n == 1 { 1 } else { 0 };
                    context += if k_bits < 18 {
                        u32_zero_bit_0(k_bits)
                    } else {
                        18
                    };
                    last_point.z = the_context.decompressors.z.decompress(
                        &mut self.decoders.z,
                        the_context.last_z[l],
                        context,
                    )?;
                    the_context.last_z[l] = last_point.z;
                }

                // Decompress classification
                if self.should_decompress.classification {
                    let last_classification = last_point.classification;
                    let ccc = (((last_classification & 0x1F) << 1) + (if cpr == 3 { 1 } else { 0 }))
                        as usize;
                    last_point.classification = self.decoders.classification.decode_symbol(
                        &mut the_context.models.classification[ccc]
                            .get_or_insert_with(|| ArithmeticModelBuilder::new(256).build()),
                    )? as u8;
                }

                // Decompress flags
                if self.should_decompress.flags {
                    let last_flags = (last_point.edge_of_flight_line() as u8) << 5
                        | (last_point.scan_direction_flag() as u8) << 4
                        | last_point.classification_flags();
                    let last_flags = last_flags as usize;
                    let flags = self.decoders.flags.decode_symbol(
                        &mut the_context.models.classification_flags[last_flags]
                            .get_or_insert_with(|| ArithmeticModelBuilder::new(64).build()),
                    )?;

                    // FIXME
                    last_point.flags = ((flags >> 5 & 1) << 7
                        | (flags >> 4 & 1) << 6
                        | ((last_point.scanner_channel() << 4) & 0b00110000) as u32
                        | (flags & 0b00001111)) as u8;
                }

                if self.should_decompress.intensity {
                    let idx = (cpr << 1 | (gps_time_changed as u32)) as usize;
                    last_point.intensity = the_context.decompressors.intensity.decompress(
                        &mut self.decoders.intensity,
                        the_context.last_intensities[idx] as i32,
                        cpr,
                    )? as u16;
                    the_context.last_intensities[idx] = last_point.intensity;
                }

                if self.should_decompress.scan_angle && scan_angle_changed {
                    last_point.scan_angle_rank = the_context.decompressors.scan_angle.decompress(
                        &mut self.decoders.scan_angle,
                        last_point.scan_angle_rank as i32,
                        gps_time_changed as u32,
                    )? as u16;
                }

                if self.should_decompress.user_data {
                    let user_data = self.decoders.user_data.decode_symbol(
                        the_context.models.user_data[(last_point.user_data / 4) as usize]
                            .get_or_insert_with(|| ArithmeticModelBuilder::new(256).build()),
                    )?;
                    last_point.set_user_data(user_data as u8);
                }

                if self.should_decompress.point_source && point_source_changed {
                    last_point.point_source_id = the_context.decompressors.source_id.decompress(
                        &mut self.decoders.point_source,
                        last_point.point_source_id as i32,
                        DEFAULT_DECOMPRESS_CONTEXTS,
                    )? as u16;
                }
                last_point.gps_time_change = gps_time_changed;
            }

            if self.should_decompress.gps_time && gps_time_changed {
                self.read_gps_time()?;
                let gps_context = &self.contexts[self.current_context].gps_sequences;
                self.contexts[self.current_context].last_point.gps_time =
                    gps_context.last_gps_times[gps_context.last].gps_time();
            }

            let last_point = &mut self.contexts[self.current_context].last_point;
            last_point.gps_time_change = gps_time_changed;
            last_point.pack_into(current_point);
            Ok(())
        }

        fn read_layers_sizes(&mut self, src: &mut R) -> std::io::Result<()> {
            self.layers_sizes = LayerSizes::read_from(src)?;
            //println!("{:?}", self.layers_sizes);
            Ok(())
        }

        fn read_layers(&mut self, src: &mut R) -> std::io::Result<()> {
            let num_bytes = &self.layers_sizes;

            copy_bytes_into_decoder(
                true, // Always decode x,y,channel & returns,
                num_bytes.channel_returns_xy,
                &mut self.decoders.channel_returns_xy,
                src,
            )?;

            self.should_decompress.z = copy_bytes_into_decoder(
                self.decompression_selector.z_requested(),
                num_bytes.z,
                &mut self.decoders.z,
                src,
            )?;

            self.should_decompress.classification = copy_bytes_into_decoder(
                self.decompression_selector.classification_requested(),
                num_bytes.classification,
                &mut self.decoders.classification,
                src,
            )?;

            self.should_decompress.flags = copy_bytes_into_decoder(
                self.decompression_selector.flags_requested(),
                num_bytes.flags,
                &mut self.decoders.flags,
                src,
            )?;

            self.should_decompress.intensity = copy_bytes_into_decoder(
                self.decompression_selector.intensity_requested(),
                num_bytes.intensity,
                &mut self.decoders.intensity,
                src,
            )?;

            self.should_decompress.scan_angle = copy_bytes_into_decoder(
                self.decompression_selector.scan_angle_requested(),
                num_bytes.scan_angle,
                &mut self.decoders.scan_angle,
                src,
            )?;

            self.should_decompress.user_data = copy_bytes_into_decoder(
                self.decompression_selector.user_data_requested(),
                num_bytes.user_data,
                &mut self.decoders.user_data,
                src,
            )?;

            self.should_decompress.point_source = copy_bytes_into_decoder(
                self.decompression_selector.point_source_requested(),
                num_bytes.point_source,
                &mut self.decoders.point_source,
                src,
            )?;

            self.should_decompress.gps_time = copy_bytes_into_decoder(
                self.decompression_selector.gps_time_requested(),
                num_bytes.gps_time,
                &mut self.decoders.gps_time,
                src,
            )?;

            Ok(())
        }
    }

    struct Point6Encoders {
        channel_returns_xy: ArithmeticEncoder<Cursor<Vec<u8>>>,
        z: ArithmeticEncoder<Cursor<Vec<u8>>>,
        classification: ArithmeticEncoder<Cursor<Vec<u8>>>,
        flags: ArithmeticEncoder<Cursor<Vec<u8>>>,
        intensity: ArithmeticEncoder<Cursor<Vec<u8>>>,
        scan_angle: ArithmeticEncoder<Cursor<Vec<u8>>>,
        user_data: ArithmeticEncoder<Cursor<Vec<u8>>>,
        point_source: ArithmeticEncoder<Cursor<Vec<u8>>>,
        gps_time: ArithmeticEncoder<Cursor<Vec<u8>>>,
    }

    impl Default for Point6Encoders {
        fn default() -> Self {
            Self {
                channel_returns_xy: ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new())),
                z: ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new())),
                classification: ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new())),
                flags: ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new())),
                intensity: ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new())),
                scan_angle: ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new())),
                user_data: ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new())),
                point_source: ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new())),
                gps_time: ArithmeticEncoder::new(Cursor::new(Vec::<u8>::new())),
            }
        }
    }

    struct Point6Compressors {
        dx: IntegerCompressor,
        dy: IntegerCompressor,
        z: IntegerCompressor,
        intensity: IntegerCompressor,
        scan_angle: IntegerCompressor,
        source_id: IntegerCompressor,
        gps_time: IntegerCompressor,
    }

    impl Default for Point6Compressors {
        fn default() -> Self {
            Self {
                dx: IntegerCompressorBuilder::new()
                    .bits(32)
                    .contexts(2)
                    .build_initialized(),
                dy: IntegerCompressorBuilder::new()
                    .bits(32)
                    .contexts(22)
                    .build_initialized(),
                z: IntegerCompressorBuilder::new()
                    .bits(32)
                    .contexts(20)
                    .build_initialized(),
                intensity: IntegerCompressorBuilder::new()
                    .bits(16)
                    .contexts(4)
                    .build_initialized(),
                scan_angle: IntegerCompressorBuilder::new()
                    .bits(16)
                    .contexts(2)
                    .build_initialized(),
                source_id: IntegerCompressorBuilder::new().bits(16).build_initialized(),
                gps_time: IntegerCompressorBuilder::new()
                    .bits(32)
                    .contexts(9)
                    .build_initialized(),
            }
        }
    }

    struct Point6CompressionContext {
        unused: bool,

        last_point: Point6,
        last_intensities: [u16; 8],
        last_x_diff_median5: [StreamingMedian<i32>; 12],
        last_y_diff_median5: [StreamingMedian<i32>; 12],
        last_z: [i32; 8],

        models: Point6Models,
        compressors: Point6Compressors,
        gps_sequences: GpsTimeSequences,
    }

    impl Default for Point6CompressionContext {
        fn default() -> Self {
            Self {
                unused: true,
                last_point: Default::default(),
                last_intensities: [0; 8],
                last_x_diff_median5: [StreamingMedian::<i32>::new(); 12],
                last_y_diff_median5: [StreamingMedian::<i32>::new(); 12],
                last_z: [0; 8],
                models: Point6Models::default(),
                compressors: Point6Compressors::default(),
                gps_sequences: GpsTimeSequences::default(),
            }
        }
    }

    impl Point6CompressionContext {
        fn init_from_last(&mut self, last: Point6) {
            self.last_point = last;
            for intensity in &mut self.last_intensities {
                *intensity = self.last_point.intensity;
            }
            for z in &mut self.last_z {
                *z = self.last_point.z;
            }
            self.gps_sequences = GpsTimeSequences::from_point(&self.last_point);
            self.unused = false;
        }
    }

    pub struct LasPoint6Compressor {
        encoders: Point6Encoders,
        has_changed: Point6FieldFlags,

        current_context: usize,
        contexts: [Point6CompressionContext; 4],
    }

    impl Default for LasPoint6Compressor {
        fn default() -> Self {
            Self {
                encoders: Point6Encoders::default(),
                has_changed: Point6FieldFlags::default(),
                current_context: 0,
                contexts: [
                    Point6CompressionContext::default(),
                    Point6CompressionContext::default(),
                    Point6CompressionContext::default(),
                    Point6CompressionContext::default(),
                ],
            }
        }
    }

    impl LasPoint6Compressor {
        fn compress_gps_time(&mut self, gps_time: GpsTime) -> std::io::Result<()> {
            let the_context = &mut self.contexts[self.current_context];
            if the_context.gps_sequences.last_gps_diffs[the_context.gps_sequences.last] == 0 {
                ////println!("Last diff was 0");
                // if the last integer difference was zero
                // calculate the difference between the two doubles as an integer
                let curr_gps_time_diff_64 = i64::from(gps_time)
                    - i64::from(
                        the_context.gps_sequences.last_gps_times[the_context.gps_sequences.last],
                    );
                let curr_gps_time_diff = curr_gps_time_diff_64 as i32;
                if i64::from(curr_gps_time_diff) == curr_gps_time_diff_64 {
                    // the difference can be represented with 32 bits
                    self.encoders
                        .gps_time
                        .encode_symbol(&mut the_context.models.gps_time_no_diff, 0)?;
                    the_context.compressors.gps_time.compress(
                        &mut self.encoders.gps_time,
                        0,
                        curr_gps_time_diff,
                        0,
                    )?;
                    the_context.gps_sequences.last_gps_diffs[the_context.gps_sequences.last] =
                        curr_gps_time_diff;
                    the_context.gps_sequences.multi_extreme_counter
                        [the_context.gps_sequences.last] = 0;
                } else {
                    // The difference is huge
                    // Maybe the double belongs to another sequence
                    let mut other_sequence = None;
                    for i in 1..4 {
                        let idx = (the_context.gps_sequences.last + i) & 3;
                        let other_gps_time_diff_64 = i64::from(gps_time)
                            - i64::from(the_context.gps_sequences.last_gps_times[idx]);
                        let other_gps_time_diff = other_gps_time_diff_64 as i32;
                        if i64::from(other_gps_time_diff) == other_gps_time_diff_64 {
                            other_sequence = Some(i);
                            break;
                        }
                    }
                    if let Some(i) = other_sequence {
                        self.encoders.gps_time.encode_symbol(
                            &mut the_context.models.gps_time_no_diff,
                            (i + 1) as u32,
                        )?;
                        the_context.gps_sequences.last = (the_context.gps_sequences.last + i) & 3;
                        return self.compress_gps_time(gps_time);
                    } else {
                        // Lets start a new sequence
                        self.encoders
                            .gps_time
                            .encode_symbol(&mut the_context.models.gps_time_no_diff, 1)?;
                        the_context.compressors.gps_time.compress(
                            &mut self.encoders.gps_time,
                            (i64::from(
                                the_context.gps_sequences.last_gps_times
                                    [the_context.gps_sequences.last],
                            ) >> 32) as i32,
                            (i64::from(gps_time) >> 32) as i32,
                            8,
                        )?;
                        self.encoders
                            .gps_time
                            .write_int(i64::from(gps_time) as u32)?;
                        the_context.gps_sequences.next += 1;
                        the_context.gps_sequences.next &= 3;
                        the_context.gps_sequences.last = the_context.gps_sequences.next;
                        the_context.gps_sequences.last_gps_diffs[the_context.gps_sequences.last] =
                            0;
                        the_context.gps_sequences.multi_extreme_counter
                            [the_context.gps_sequences.last] = 0;
                    }
                    the_context.gps_sequences.last_gps_times[the_context.gps_sequences.last] =
                        gps_time;
                }
            } else {
                ////println!("Last diff was not 0");
                // the last integer difference was *not* zero
                let curr_gps_time_diff_64 = i64::from(gps_time)
                    - i64::from(
                        the_context.gps_sequences.last_gps_times[the_context.gps_sequences.last],
                    );
                let curr_gps_time_diff = curr_gps_time_diff_64 as i32;

                if curr_gps_time_diff_64 == i64::from(curr_gps_time_diff) {
                    ////println!("diff encodable with 32 bits");
                    // if the current gps_time difference can be represented with 32 bits
                    let multi_f = (curr_gps_time_diff as f32)
                        / (the_context.gps_sequences.last_gps_diffs[the_context.gps_sequences.last]
                            as f32);
                    let multi = i32_quantize(multi_f);

                    // compress the residual curr_gps_time_diff in dependence on the multiplier
                    if multi == 1 {
                        // this is the case we assume we get most often for regular spaced pulses
                        self.encoders
                            .gps_time
                            .encode_symbol(&mut the_context.models.gps_time_multi, 1)?;
                        the_context.compressors.gps_time.compress(
                            &mut self.encoders.gps_time,
                            the_context.gps_sequences.last_gps_diffs
                                [the_context.gps_sequences.last],
                            curr_gps_time_diff,
                            1,
                        )?;
                        the_context.gps_sequences.multi_extreme_counter
                            [the_context.gps_sequences.last] = 0;
                    } else if multi > 0 {
                        if multi < LASZIP_GPS_TIME_MULTI {
                            // positive multipliers up to LASZIP_GPS_TIME_MULTI are compressed directly
                            self.encoders.gps_time.encode_symbol(
                                &mut the_context.models.gps_time_multi,
                                multi as u32,
                            )?;
                            let context = if multi < 10 { 2 } else { 3 };
                            the_context.compressors.gps_time.compress(
                                &mut self.encoders.gps_time,
                                multi.wrapping_mul(
                                    the_context.gps_sequences.last_gps_diffs
                                        [the_context.gps_sequences.last],
                                ),
                                curr_gps_time_diff,
                                context,
                            )?;
                        } else {
                            self.encoders.gps_time.encode_symbol(
                                &mut the_context.models.gps_time_multi,
                                LASZIP_GPS_TIME_MULTI as u32,
                            )?;
                            the_context.compressors.gps_time.compress(
                                &mut self.encoders.gps_time,
                                LASZIP_GPS_TIME_MULTI
                                    * the_context.gps_sequences.last_gps_diffs
                                        [the_context.gps_sequences.last],
                                curr_gps_time_diff,
                                4,
                            )?;
                            let counter = &mut the_context.gps_sequences.multi_extreme_counter
                                [the_context.gps_sequences.last];
                            *counter += 1;
                            if *counter > 3 {
                                the_context.gps_sequences.last_gps_diffs
                                    [the_context.gps_sequences.last] = curr_gps_time_diff;
                                *counter = 0;
                            }
                        }
                    } else if multi < 0 {
                        if multi > LASZIP_GPS_TIME_MULTI_MINUS {
                            // negative multipliers larger than LASZIP_GPS_TIME_MULTI_MINUS are compressed directly
                            self.encoders.gps_time.encode_symbol(
                                &mut the_context.models.gps_time_multi,
                                (LASZIP_GPS_TIME_MULTI - multi) as u32,
                            )?;
                            the_context.compressors.gps_time.compress(
                                &mut self.encoders.gps_time,
                                multi.wrapping_mul(
                                    the_context.gps_sequences.last_gps_diffs
                                        [the_context.gps_sequences.last],
                                ),
                                curr_gps_time_diff,
                                5,
                            )?;
                        } else {
                            //TODO this codes is just copy pasta + changed values compared to above
                            self.encoders.gps_time.encode_symbol(
                                &mut the_context.models.gps_time_multi,
                                (LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS) as u32,
                            )?;
                            the_context.compressors.gps_time.compress(
                                &mut self.encoders.gps_time,
                                LASZIP_GPS_TIME_MULTI_MINUS
                                    * the_context.gps_sequences.last_gps_diffs
                                        [the_context.gps_sequences.last],
                                curr_gps_time_diff,
                                6,
                            )?;
                            let counter = &mut the_context.gps_sequences.multi_extreme_counter
                                [the_context.gps_sequences.last];
                            *counter += 1;
                            if *counter > 3 {
                                the_context.gps_sequences.last_gps_diffs
                                    [the_context.gps_sequences.last] = curr_gps_time_diff;
                                *counter = 0;
                            }
                        }
                    } else {
                        //TODO this codes is just copy pasta + changed values compared to above
                        self.encoders
                            .gps_time
                            .encode_symbol(&mut the_context.models.gps_time_multi, 0)?;
                        the_context.compressors.gps_time.compress(
                            &mut self.encoders.gps_time,
                            0,
                            curr_gps_time_diff,
                            7,
                        )?;
                        let counter = &mut the_context.gps_sequences.multi_extreme_counter
                            [the_context.gps_sequences.last];
                        *counter += 1;
                        if *counter > 3 {
                            the_context.gps_sequences.last_gps_diffs
                                [the_context.gps_sequences.last] = curr_gps_time_diff;
                            *counter = 0;
                        }
                    }
                } else {
                    //TODO this fucking looks so much fucking like the code a few lines above
                    // the difference is huge
                    for i in 1..4 {
                        let other_gps_time_diff_64 = i64::from(gps_time)
                            - i64::from(
                                the_context.gps_sequences.last_gps_times
                                    [(the_context.gps_sequences.last + i) & 3],
                            );
                        let other_gps_time_diff = other_gps_time_diff_64 as i32;
                        if other_gps_time_diff_64 == i64::from(other_gps_time_diff) {
                            // it belongs to this sequence
                            self.encoders.gps_time.encode_symbol(
                                &mut the_context.models.gps_time_multi,
                                (LASZIP_GPS_TIME_MULTI_CODE_FULL + i as i32) as u32,
                            )?;
                            the_context.gps_sequences.last += i;
                            the_context.gps_sequences.last &= 3;
                            return self.compress_gps_time(gps_time);
                        }
                    }
                    // no other sequence found. start new sequence.
                    self.encoders.gps_time.encode_symbol(
                        &mut the_context.models.gps_time_multi,
                        LASZIP_GPS_TIME_MULTI_CODE_FULL as u32,
                    )?;
                    the_context.compressors.gps_time.compress(
                        &mut self.encoders.gps_time,
                        (i64::from(
                            the_context.gps_sequences.last_gps_times
                                [the_context.gps_sequences.last],
                        ) >> 32) as i32,
                        (i64::from(gps_time) >> 32) as i32,
                        8,
                    )?;
                    self.encoders
                        .gps_time
                        .write_int(i64::from(gps_time) as u32)?;
                    the_context.gps_sequences.next += 1;
                    the_context.gps_sequences.next &= 3;
                    the_context.gps_sequences.last = the_context.gps_sequences.next;
                    the_context.gps_sequences.last_gps_diffs[the_context.gps_sequences.last] = 0;
                    the_context.gps_sequences.multi_extreme_counter
                        [the_context.gps_sequences.last] = 0;
                }
                the_context.gps_sequences.last_gps_times[the_context.gps_sequences.last] = gps_time;
            }
            the_context.gps_sequences.last_gps_times[the_context.gps_sequences.last] = gps_time;
            Ok(())
        }
    }

    impl<W: Write> LayeredFieldCompressor<W> for LasPoint6Compressor {
        fn size_of_field(&self) -> usize {
            Point6::SIZE
        }

        fn init_first_point(
            &mut self,
            dst: &mut W,
            first_point: &[u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            for context in &mut self.contexts {
                context.unused = true;
            }
            dst.write_all(first_point)?;

            let first_point = Point6::unpack_from(first_point);
            self.current_context = first_point.scanner_channel() as usize;
            *context = self.current_context;

            self.contexts[self.current_context].init_from_last(first_point);
            Ok(())
        }

        fn compress_field_with(
            &mut self,
            current_point: &[u8],
            context: &mut usize,
        ) -> std::io::Result<()> {
            let current_point = Point6::unpack_from(current_point);
            let lpr = compute_last_point_return(&self.contexts[self.current_context].last_point);
            let scanner_channel = current_point.scanner_channel();
            {
                if scanner_channel != self.current_context as u8 {
                    if self.contexts[scanner_channel as usize].unused == false {
                        //FIXME its not 100% the same as Laszip, will it be problematic ?
                        self.contexts[self.current_context].last_point =
                            self.contexts[scanner_channel as usize].last_point;
                    }
                }
            }

            let last_n: usize;
            let last_r: usize;
            let n: u8;
            let r: u8;
            let gps_time_changed: bool;
            let point_source_changed: bool;
            let scan_angle_changed: bool;
            let mut changed_values;
            {
                let the_context = &mut self.contexts[self.current_context];
                let last_point = &mut the_context.last_point;
                // determine changed attributes
                point_source_changed =
                    last_point.point_source_id != current_point.point_source_id();
                gps_time_changed = last_point.gps_time != current_point.gps_time();
                scan_angle_changed = last_point.scan_angle_rank != current_point.scan_angle_rank();

                // get last and current return counts
                last_n = last_point.number_of_returns_of_given_pulse() as usize;
                last_r = last_point.return_number() as usize;
                n = current_point.number_of_returns_of_given_pulse();
                r = current_point.return_number();

                // create the 7 bit mask that encodes various changes (its value ranges from 0 to 127)
                changed_values = (
                    ((scanner_channel != self.current_context as u8) as i32) << 6) | // scanner channel compared to last point (same = 0 / different = 1)
                    ((point_source_changed as i32) << 5) |                  // point source ID compared to last point from *same* scanner channel (same = 0 / different = 1)
                    ((gps_time_changed as i32) << 4) |                      // GPS time stamp compared to last point from *same* scanner channel (same = 0 / different = 1)
                    ((scan_angle_changed as i32) << 3) |                    // scan angle compared to last point from *same* scanner channel (same = 0 / different = 1)
                    (((n != last_n as u8) as i32) << 2); // number of returns compared to last point from *same* scanner channel (same = 0 / different = 1)

                // return number compared to last point of *same* scanner channel
                // (same = 0 / plus one mod 16 = 1 / minus one mod 16 = 2 / other difference = 3)
                if r != last_r as u8 {
                    if r == ((last_r + 1) % 16) as u8 {
                        changed_values |= 1;
                    } else if r == ((last_r + 15) % 16) as u8 {
                        changed_values |= 2;
                    } else {
                        changed_values |= 3;
                    }
                }
                self.encoders.channel_returns_xy.encode_symbol(
                    &mut the_context.models.changed_values[lpr],
                    changed_values as u32,
                )?;
            }
            //println!("Changed values: {:b}, lpr used: {}", changed_values, lpr);

            if changed_values & (1 << 6) != 0 {
                let diff = scanner_channel - self.current_context as u8;
                let symbol = if diff > 0 {
                    i32::from(diff - 1)
                } else {
                    i32::from(diff + 4)
                };
                self.encoders.channel_returns_xy.encode_symbol(
                    &mut self.contexts[self.current_context].models.scanner_channel,
                    symbol as u32,
                )?;

                if self.contexts[scanner_channel as usize].unused {
                    self.contexts[scanner_channel as usize]
                        .init_from_last(self.contexts[self.current_context].last_point);
                }
                self.current_context = scanner_channel as usize;
                *context = self.current_context;
            }

            let the_context = &mut self.contexts[self.current_context];

            // if number of returns is different we compress it
            if (changed_values & (1 << 2)) != 0 {
                let model = the_context.models.number_of_returns[last_n]
                    .get_or_insert_with(|| ArithmeticModelBuilder::new(16).build());
                self.encoders
                    .channel_returns_xy
                    .encode_symbol(model, u32::from(n))?;
            }

            // if return number is different and difference is bigger than +1 / -1
            // we compress how it is different
            if (changed_values & 3) == 3 {
                if gps_time_changed {
                    let model = the_context.models.return_number[last_r]
                        .get_or_insert_with(|| ArithmeticModelBuilder::new(16).build());
                    self.encoders
                        .channel_returns_xy
                        .encode_symbol(model, u32::from(r))?;
                } else {
                    let diff = i32::from(r) - last_r as i32;
                    let sym = if diff > 1 { diff - 2 } else { diff + 16 - 2 };
                    self.encoders.channel_returns_xy.encode_symbol(
                        &mut the_context.models.return_number_gps_same,
                        sym as u32,
                    )?;
                }
            }

            // get return map m and return level l context for current point
            let m = NUMBER_RETURN_MAP_6CTX[n as usize][r as usize];
            let l = NUMBER_RETURN_LEVEL_8CT[n as usize][r as usize];

            // create single return context for current point
            //(3) / first (1) / last (2) / intermediate (0)

            let mut cpr = if r == 1 { 2 } else { 0 }; //first ?
            cpr += if r >= n { 1 } else { 0 }; // last ?

            let idx = (m << 1) as usize | gps_time_changed as usize;
            // Compress X
            let median = the_context.last_x_diff_median5[idx].get();
            let diff = current_point.x().wrapping_sub(the_context.last_point.x);
            the_context.compressors.dx.compress(
                &mut self.encoders.channel_returns_xy,
                median,
                diff,
                (n == 1) as u32,
            )?;
            the_context.last_x_diff_median5[idx].add(diff);

            // Compress Y
            let k_bits = the_context.compressors.dx.k();
            let median = the_context.last_y_diff_median5[idx].get();
            //println!("Median {}, diff {}", median, diff);
            let diff = current_point.y().wrapping_sub(the_context.last_point.y);
            let context = (n == 1) as u32
                + if k_bits < 20 {
                    u32_zero_bit_0(k_bits)
                } else {
                    20
                };
            the_context.compressors.dy.compress(
                &mut self.encoders.channel_returns_xy,
                median,
                diff,
                context,
            )?;
            the_context.last_y_diff_median5[idx].add(diff);

            // Compress Z
            let k_bits = (the_context.compressors.dx.k() + the_context.compressors.dy.k()) / 2;
            let context = (n == 1) as u32
                + if k_bits < 18 {
                    u32_zero_bit_0(k_bits)
                } else {
                    18
                };
            the_context.compressors.z.compress(
                &mut self.encoders.z,
                the_context.last_z[l as usize],
                current_point.z(),
                context,
            )?;
            the_context.last_z[l as usize] = current_point.z();

            // Compress classification
            let last_classification = the_context.last_point.classification;
            let classification = current_point.classification();

            if classification != last_classification {
                self.has_changed.classification = true;
            }
            let ccc = ((last_classification & 0x1F) << 1) as usize + (cpr == 3) as usize;
            let model = the_context.models.classification[ccc]
                .get_or_insert_with(|| ArithmeticModelBuilder::new(256).build());
            self.encoders
                .classification
                .encode_symbol(model, u32::from(classification))?;

            // Compress flags
            let last_flags = the_context.last_point.classification_flags();
            let flags = current_point.classification_flags();
            if last_flags != flags {
                self.has_changed.flags = true;
            }

            let model = the_context.models.classification_flags[last_flags as usize]
                .get_or_insert_with(|| ArithmeticModelBuilder::new(64).build());
            self.encoders.flags.encode_symbol(model, u32::from(flags))?;

            // Compress intensity
            if the_context.last_point.intensity != current_point.intensity() {
                self.has_changed.intensity = true;
            }
            the_context.compressors.intensity.compress(
                &mut self.encoders.intensity,
                the_context.last_intensities[(cpr << 1) as usize | gps_time_changed as usize]
                    as i32,
                current_point.intensity() as i32,
                cpr,
            )?;
            the_context.last_intensities[(cpr << 1) as usize | gps_time_changed as usize] =
                current_point.intensity();

            // Compress scan angle
            if scan_angle_changed {
                self.has_changed.scan_angle = true;
                the_context.compressors.scan_angle.compress(
                    &mut self.encoders.scan_angle,
                    the_context.last_point.scan_angle_rank as i32,
                    current_point.scan_angle_rank() as i32,
                    gps_time_changed as u32,
                )?;
            }

            // Compress user data
            if the_context.last_point.user_data != current_point.user_data() {
                self.has_changed.user_data = true;
            }

            let model = the_context.models.user_data[the_context.last_point.user_data as usize / 4]
                .get_or_insert_with(|| ArithmeticModelBuilder::new(256).build());
            self.encoders
                .user_data
                .encode_symbol(model, u32::from(current_point.user_data()))?;

            // Compress point source id
            if point_source_changed {
                self.has_changed.point_source = true;
                the_context.compressors.source_id.compress(
                    &mut self.encoders.point_source,
                    the_context.last_point.point_source_id as i32,
                    current_point.point_source_id() as i32,
                    DEFAULT_COMPRESS_CONTEXTS,
                )?;
            }

            the_context.last_point = current_point;
            the_context.last_point.gps_time_change = gps_time_changed;
            if gps_time_changed {
                self.has_changed.gps_time = true;
                let gps_time = GpsTime::from(current_point.gps_time);
                self.compress_gps_time(gps_time)?;
            }
            Ok(())
        }

        fn write_layers_sizes(&mut self, dst: &mut W) -> std::io::Result<()> {
            macro_rules! call_done_if_has_changed {
                ($name:ident) => {
                    if self.has_changed.$name {
                        self.encoders.$name.done()?;
                    }
                };
            }
            self.encoders.channel_returns_xy.done()?;
            self.encoders.z.done()?;
            call_done_if_has_changed!(classification);
            call_done_if_has_changed!(flags);
            call_done_if_has_changed!(intensity);
            call_done_if_has_changed!(scan_angle);
            call_done_if_has_changed!(user_data);
            call_done_if_has_changed!(point_source);
            call_done_if_has_changed!(gps_time);

            let sizes = LayerSizes {
                channel_returns_xy: self
                    .encoders
                    .channel_returns_xy
                    .out_stream()
                    .get_ref()
                    .len(),
                z: self.encoders.z.out_stream().get_ref().len(),
                classification: self.encoders.classification.out_stream().get_ref().len(),
                flags: self.encoders.flags.out_stream().get_ref().len(),
                intensity: self.encoders.intensity.out_stream().get_ref().len(),
                scan_angle: self.encoders.scan_angle.out_stream().get_ref().len(),
                user_data: self.encoders.user_data.out_stream().get_ref().len(),
                point_source: self.encoders.point_source.out_stream().get_ref().len(),
                gps_time: self.encoders.gps_time.out_stream().get_ref().len(),
            };
            //println!("write layer size: {:?}", sizes);
            sizes.write_to(dst)?;
            Ok(())
        }

        fn write_layers(&mut self, dst: &mut W) -> std::io::Result<()> {
            //println!("Has changed: {:?}", self.has_changed);
            macro_rules! copy_encoder_content_if_has_changed {
                ($name:ident) => {
                    if self.has_changed.$name {
                        copy_encoder_content_to(&mut self.encoders.$name, dst)?;
                    }
                };
            }
            copy_encoder_content_to(&mut self.encoders.channel_returns_xy, dst)?;
            copy_encoder_content_to(&mut self.encoders.z, dst)?;
            copy_encoder_content_if_has_changed!(classification);
            copy_encoder_content_if_has_changed!(flags);
            copy_encoder_content_if_has_changed!(intensity);
            copy_encoder_content_if_has_changed!(scan_angle);
            copy_encoder_content_if_has_changed!(user_data);
            copy_encoder_content_if_has_changed!(point_source);
            copy_encoder_content_if_has_changed!(gps_time);

            Ok(())
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use std::io::SeekFrom;

        #[test]
        fn test_write_read_layer_sizes() {
            let sizes = LayerSizes {
                channel_returns_xy: 1,
                z: 2,
                classification: 3,
                flags: 4,
                intensity: 5,
                scan_angle: 6,
                user_data: 7,
                point_source: 8,
                gps_time: 9,
            };

            let mut dst = Cursor::new(Vec::<u8>::new());
            sizes.write_to(&mut dst).unwrap();
            dst.seek(SeekFrom::Start(0)).unwrap();

            let r_sizes = LayerSizes::read_from(&mut dst).unwrap();
            assert_eq!(r_sizes, sizes);
        }
    }
}
