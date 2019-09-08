use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

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
    fn return_number(&self) -> u8; // 4 bits
    fn number_of_returns_of_given_pulse(&self) -> u8; // 4bits

    fn flags(&self) -> u8;
    // all theses values are packed into the same byte
    fn classification_flags(&self) -> u8; // 4 bits
    fn scanner_channel(&self) -> u8; // 2 bits
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

    fn read_from<R: Read>(&mut self, src: &mut R) -> std::io::Result<()> {
        self.set_x(src.read_i32::<LittleEndian>()?);
        self.set_y(src.read_i32::<LittleEndian>()?);
        self.set_z(src.read_i32::<LittleEndian>()?);
        self.set_intensity(src.read_u16::<LittleEndian>()?);
        self.set_bit_fields(src.read_u8()?);
        self.set_flags(src.read_u8()?);
        self.set_classification(src.read_u8()?);
        self.set_user_data(src.read_u8()?);
        self.set_scan_angle_rank(src.read_u16::<LittleEndian>()?);
        self.set_point_source_id(src.read_u16::<LittleEndian>()?);
        self.set_gps_time(src.read_f64::<LittleEndian>()?);
        Ok(())
    }

    fn write_to<W: Write>(&mut self, dst: &mut W) -> std::io::Result<()> {
        dst.write_i32::<LittleEndian>(self.x())?;
        dst.write_i32::<LittleEndian>(self.y())?;
        dst.write_i32::<LittleEndian>(self.z())?;
        dst.write_u16::<LittleEndian>(self.intensity())?;
        dst.write_u8(self.bit_fields())?;
        dst.write_u8(self.flags())?;
        dst.write_u8(self.classification())?;
        dst.write_u8(self.user_data())?;
        dst.write_u16::<LittleEndian>(self.scan_angle_rank())?;
        dst.write_u16::<LittleEndian>(self.point_source_id())?;
        dst.write_f64::<LittleEndian>(self.gps_time())?;
        Ok(())
    }

    fn set_fields_from<P: LasPoint6>(&mut self, other: &P) {
        self.set_x(other.x());
        self.set_y(other.y());
        self.set_z(other.z());
        self.set_intensity(other.intensity());
        self.set_bit_fields(other.bit_fields());
        self.set_flags(other.flags());
        self.set_classification(other.classification());
        self.set_user_data(other.user_data());
        self.set_scan_angle_rank(other.scan_angle_rank());
        self.set_point_source_id(other.point_source_id());
        self.set_gps_time(other.gps_time());
    }
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

        let mut p2 = Point6::default();
        p2.set_fields_from(&p);
        assert_eq!(p2.bit_fields, 17);

        p.set_number_of_returns(2);
        assert_eq!(p.number_of_returns_of_given_pulse(), 2);
        assert_eq!(p.bit_fields, 33);
    }
}

pub mod v3 {
    use crate::decoders::ArithmeticDecoder;
    use crate::decompressors::{
        IntegerDecompressor, IntegerDecompressorBuilder, DEFAULT_DECOMPRESS_CONTEXTS,
    };
    use crate::las::gps::{GpsTime, LasGpsTime};
    use crate::las::point6::{u32_zero_bit_0, DecompressionSelector, LasPoint6, Point6};
    use crate::las::utils::{
        copy_bytes_into_decoder, StreamingMedian, NUMBER_RETURN_LEVEL_8CT, NUMBER_RETURN_MAP_6CTX,
    };
    use crate::models::{ArithmeticModel, ArithmeticModelBuilder};
    use crate::record::{BufferLayeredFieldDecompressor, LayeredPointFieldDecompressor};
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::{Cursor, Read, Seek};

    #[derive(Clone)]
    struct LasContextPoint6 {
        unused: bool,

        last_point: Point6,
        last_intensities: [u16; 8],
        last_x_diff_median5: [StreamingMedian<i32>; 12],
        last_y_diff_median5: [StreamingMedian<i32>; 12],
        last_z: [i32; 8],

        changed_values_models: Vec<ArithmeticModel>, //8
        scanner_channel_model: ArithmeticModel,
        number_of_returns_models: Vec<Option<ArithmeticModel>>, // 16
        return_number: Vec<Option<ArithmeticModel>>,            // 16
        return_number_gps_same_model: ArithmeticModel,
        classification_models: Vec<Option<ArithmeticModel>>, // 64
        classification_flags: Vec<Option<ArithmeticModel>>,  // 64
        user_data: Vec<Option<ArithmeticModel>>,             // 64

        id_dx: IntegerDecompressor,
        id_dy: IntegerDecompressor,
        id_z: IntegerDecompressor,
        id_intensity: IntegerDecompressor,
        id_scan_angle: IntegerDecompressor,
        id_source_id: IntegerDecompressor,

        gps_time_context: GpsTimeDecompressionContext,
    }

    impl LasContextPoint6 {
        fn from_last_point(point: &Point6) -> Self {
            let mut me = Self {
                unused: false,
                last_point: *point,
                last_intensities: [point.intensity; 8],
                last_x_diff_median5: [StreamingMedian::<i32>::new(); 12],
                last_y_diff_median5: [StreamingMedian::<i32>::new(); 12],
                last_z: [point.z; 8],
                changed_values_models: (0..8)
                    .into_iter()
                    .map(|_| ArithmeticModelBuilder::new(128).build())
                    .collect(),
                scanner_channel_model: ArithmeticModelBuilder::new(3).build(),
                number_of_returns_models: (0..16).into_iter().map(|_| None).collect(),
                return_number: (0..16).into_iter().map(|_| None).collect(),
                return_number_gps_same_model: ArithmeticModelBuilder::new(13).build(),
                classification_models: (0..64).into_iter().map(|_| None).collect(),
                classification_flags: (0..64).into_iter().map(|_| None).collect(),
                user_data: (0..64).into_iter().map(|_| None).collect(),
                id_dx: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(2)
                    .build_initialized(),
                id_dy: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(22)
                    .build_initialized(),
                id_z: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(20)
                    .build_initialized(),
                id_intensity: IntegerDecompressorBuilder::new()
                    .bits(16)
                    .contexts(4)
                    .build_initialized(),
                id_scan_angle: IntegerDecompressorBuilder::new()
                    .bits(16)
                    .contexts(2)
                    .build_initialized(),
                id_source_id: IntegerDecompressorBuilder::new()
                    .bits(16)
                    .build_initialized(),
                gps_time_context: GpsTimeDecompressionContext::from_point(point),
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
    struct GpsTimeDecompressionContext {
        last: usize,
        next: usize,
        last_gps_times: [GpsTime; 4],
        last_gps_diffs: [i32; 4],
        multi_extreme_counter: [i32; 4],

        multi_model: ArithmeticModel,
        no_diff_model: ArithmeticModel,
        dc: IntegerDecompressor,
    }

    impl Default for GpsTimeDecompressionContext {
        fn default() -> Self {
            Self {
                last: 0,
                next: 0,
                last_gps_times: [GpsTime::default(); 4],
                last_gps_diffs: [0; 4],
                multi_extreme_counter: [0; 4],
                multi_model: ArithmeticModelBuilder::new(LASZIP_GPS_TIME_MULTI_TOTAL as u32)
                    .build(),
                no_diff_model: ArithmeticModelBuilder::new(5).build(),
                dc: IntegerDecompressorBuilder::new()
                    .bits(32)
                    .contexts(9)
                    .build_initialized(),
            }
        }
    }

    impl GpsTimeDecompressionContext {
        fn from_point(point: &Point6) -> Self {
            let mut me = Self::default();
            me.last_gps_times[0] = GpsTime::from(point.gps_time);
            me
        }
    }

    // Each layer has its own decoder that holds the compressed data
    // to be decoded
    struct _Point6Decoders {
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

    impl Default for _Point6Decoders {
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
    #[derive(Copy, Clone, Default, Debug)]
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
        fn read_from<R: Read>(src: &mut R) -> std::io::Result<(Self)> {
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
    }

    pub struct LasPoint6Decompressor {
        decoders: _Point6Decoders,

        changed_z: bool,
        changed_classification: bool,
        changed_flags: bool,
        changed_intensity: bool,
        changed_scan_angle: bool,
        changed_user_data: bool,
        changed_point_source: bool,
        changed_gps_time: bool,
        layers_sizes: LayerSizes,

        decompression_selector: DecompressionSelector,

        current_context: usize,
        contexts: [LasContextPoint6; 4],
    }

    impl LasPoint6Decompressor {
        pub fn new() -> Self {
            Self::selective(DecompressionSelector::decompress_all())
        }

        pub fn selective(selector: DecompressionSelector) -> Self {
            let p = Point6::default();
            Self {
                decoders: _Point6Decoders::default(),
                changed_z: false,
                changed_classification: false,
                changed_flags: false,
                changed_intensity: false,
                changed_scan_angle: false,
                changed_user_data: false,
                changed_point_source: false,
                changed_gps_time: false,
                layers_sizes: Default::default(),
                decompression_selector: selector,
                current_context: 0,
                contexts: [
                    LasContextPoint6::from_last_point(&p),
                    LasContextPoint6::from_last_point(&p),
                    LasContextPoint6::from_last_point(&p),
                    LasContextPoint6::from_last_point(&p),
                ],
            }
        }

        fn read_gps_time(&mut self) -> std::io::Result<()> {
            let the_context = &mut self.contexts[self.current_context].gps_time_context;

            let mut multi: i32;
            if the_context.last_gps_diffs[the_context.last] == 0 {
                multi = self
                    .decoders
                    .gps_time
                    .decode_symbol(&mut the_context.no_diff_model)? as i32;
                if multi == 0 {
                    // The difference can be represented with 32 bits
                    the_context.last_gps_diffs[the_context.last] =
                        the_context
                            .dc
                            .decompress(&mut self.decoders.gps_time, 0, 0)?;
                    the_context.last_gps_times[the_context.last] +=
                        i64::from(the_context.last_gps_diffs[the_context.last]);
                    the_context.multi_extreme_counter[the_context.last] = 0;
                } else if multi == 1 {
                    // Difference is huge
                    the_context.next = (the_context.next + 1) & 3;
                    let last_gps_time = the_context.last_gps_times[the_context.last].value;
                    let next_gps_time = &mut the_context.last_gps_times[the_context.next];

                    next_gps_time.value = the_context.dc.decompress(
                        &mut self.decoders.gps_time,
                        (last_gps_time >> 32) as i32,
                        8,
                    )? as i64;
                    next_gps_time.value <<= 32;
                    next_gps_time.value |= self.decoders.gps_time.read_int()? as i64;
                    the_context.last = the_context.next;
                    the_context.last_gps_diffs[the_context.last] = 0;
                    the_context.multi_extreme_counter[the_context.last] = 0;
                } else {
                    // We switch to another sequence
                    the_context.last = (the_context.last + multi as usize - 1) & 3;
                    self.read_gps_time()?;
                }
            } else {
                multi = self
                    .decoders
                    .gps_time
                    .decode_symbol(&mut the_context.multi_model)? as i32;
                if multi == 1 {
                    the_context.last_gps_times[the_context.last] += the_context.dc.decompress(
                        &mut self.decoders.gps_time,
                        the_context.last_gps_diffs[the_context.last],
                        1,
                    )? as i64;
                    the_context.multi_extreme_counter[the_context.last] = 0;
                } else if multi < LASZIP_GPS_TIME_MULTI_CODE_FULL {
                    let gps_time_diff: i32;
                    if multi == 0 {
                        gps_time_diff =
                            the_context
                                .dc
                                .decompress(&mut self.decoders.gps_time, 0, 7)?;
                        the_context.multi_extreme_counter[the_context.last] += 1;
                        if the_context.multi_extreme_counter[the_context.last] > 3 {
                            the_context.last_gps_diffs[the_context.last] = gps_time_diff;
                            the_context.multi_extreme_counter[the_context.last] = 0;
                        }
                    } else if multi < LASZIP_GPS_TIME_MULTI {
                        if multi < 10 {
                            gps_time_diff = the_context.dc.decompress(
                                &mut self.decoders.gps_time,
                                multi.wrapping_mul(the_context.last_gps_diffs[the_context.last]),
                                2,
                            )?;
                        } else {
                            gps_time_diff = the_context.dc.decompress(
                                &mut self.decoders.gps_time,
                                multi.wrapping_mul(the_context.last_gps_diffs[the_context.last]),
                                3,
                            )?;
                        }
                    } else if multi == LASZIP_GPS_TIME_MULTI {
                        gps_time_diff = the_context.dc.decompress(
                            &mut self.decoders.gps_time,
                            LASZIP_GPS_TIME_MULTI * the_context.last_gps_diffs[the_context.last],
                            4,
                        )?;
                        the_context.multi_extreme_counter[the_context.last] += 1;
                        if the_context.multi_extreme_counter[the_context.last] > 3 {
                            the_context.last_gps_diffs[the_context.last] = gps_time_diff;
                            the_context.multi_extreme_counter[the_context.last] = 0;
                        }
                    } else {
                        multi = LASZIP_GPS_TIME_MULTI - multi;
                        if multi > LASZIP_GPS_TIME_MULTI_MINUS {
                            gps_time_diff = the_context.dc.decompress(
                                &mut self.decoders.gps_time,
                                multi.wrapping_mul(the_context.last_gps_diffs[the_context.last]),
                                5,
                            )?;
                        } else {
                            gps_time_diff = the_context.dc.decompress(
                                &mut self.decoders.gps_time,
                                LASZIP_GPS_TIME_MULTI_MINUS
                                    * the_context.last_gps_diffs[the_context.last],
                                6,
                            )?;
                            the_context.multi_extreme_counter[the_context.last] += 1;
                            if the_context.multi_extreme_counter[the_context.last] > 3 {
                                the_context.last_gps_diffs[the_context.last] = gps_time_diff;
                                the_context.multi_extreme_counter[the_context.last] = 0;
                            }
                        }
                    }
                    the_context.last_gps_times[the_context.last] += i64::from(gps_time_diff);
                } else if multi == LASZIP_GPS_TIME_MULTI_CODE_FULL {
                    the_context.next = (the_context.next + 1) & 3;
                    the_context.last_gps_times[the_context.next] =
                        GpsTime::from(the_context.dc.decompress(
                            &mut self.decoders.gps_time,
                            (the_context.last_gps_times[the_context.last].value >> 32) as i32,
                            8,
                        )? as i64);
                    the_context.last_gps_times[the_context.next].value <<= 32;
                    the_context.last_gps_times[the_context.next].value |=
                        self.decoders.gps_time.read_int()? as i64;
                    the_context.last = the_context.next;
                    the_context.last_gps_diffs[the_context.last] = 0;
                    the_context.multi_extreme_counter[the_context.last] = 0;
                } else if multi >= LASZIP_GPS_TIME_MULTI_CODE_FULL {
                    the_context.last = (the_context.last + multi as usize
                        - LASZIP_GPS_TIME_MULTI_CODE_FULL as usize)
                        & 3;
                    self.read_gps_time()?;
                }
            }
            Ok(())
        }
    }

    impl<R: Read + Seek, P: LasPoint6> LayeredPointFieldDecompressor<R, P> for LasPoint6Decompressor {
        fn init_first_point(
            &mut self,
            mut src: &mut R,
            first_point: &mut P,
            context: &mut usize,
        ) -> std::io::Result<()> {
            let mut point = Point6::default();
            point.read_from(&mut src)?;

            for context in &mut self.contexts {
                context.unused = true;
            }

            self.current_context = point.scanner_channel() as usize;
            *context = self.current_context;

            assert!(self.contexts[*context].unused);
            self.contexts[*context] = LasContextPoint6::from_last_point(&point);

            first_point.set_fields_from(&point);
            Ok(())
        }

        fn decompress_field_with(
            &mut self,
            current_point: &mut P,
            context: &mut usize,
        ) -> std::io::Result<()> {
            let changed_values = {
                let the_context = &mut self.contexts[self.current_context];
                let last_point = &mut the_context.last_point;
                // Create single (3) / first (1) / last (2) / intermediate (0) context from last point return
                let mut lpr = if last_point.return_number() == 1 {
                    1
                } else {
                    0
                };
                lpr += if last_point.return_number()
                    >= last_point.number_of_returns_of_given_pulse()
                {
                    2
                } else {
                    0
                };

                // Add info whether the GPS time changed in the last return to the context
                lpr += if last_point.gps_time_change { 4 } else { 0 };

                self.decoders
                    .channel_returns_xy
                    .decode_symbol(&mut the_context.changed_values_models[lpr])?
            };

            // Scanner channel changed
            if changed_values & (1 << 6) != 0 {
                let diff = self.decoders.channel_returns_xy.decode_symbol(
                    &mut self.contexts[self.current_context].scanner_channel_model,
                )?;
                let scanner_channel = (self.current_context + diff as usize + 1) % 4; // TODO: num_context const ?

                if self.contexts[scanner_channel as usize].unused {
                    self.contexts[scanner_channel as usize] = LasContextPoint6::from_last_point(
                        &self.contexts[self.current_context].last_point,
                    );
                }

                // Switch context to current channel
                self.current_context = scanner_channel;
            }
            *context = self.current_context;

            let point_source_changed = changed_values & (1 << 5) != 0;
            let gps_time_changed = changed_values & (1 << 4) != 0;
            let scan_angle_changed = changed_values & (1 << 3) != 0;

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
                if changed_values & (1 << 2) != 0 {
                    n = self.decoders.channel_returns_xy.decode_symbol(
                        the_context.number_of_returns_models[last_n as usize]
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
                            &mut the_context.return_number[last_r as usize]
                                .get_or_insert_with(|| ArithmeticModelBuilder::new(16).build()),
                        )?;
                    } else {
                        let sym = self
                            .decoders
                            .channel_returns_xy
                            .decode_symbol(&mut the_context.return_number_gps_same_model)?;
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
                diff = the_context.id_dx.decompress(
                    &mut self.decoders.channel_returns_xy,
                    median,
                    if n == 1 { 1 } else { 0 },
                )?;
                last_point.x = last_point.x.wrapping_add(diff);
                the_context.last_x_diff_median5[idx].add(diff);

                // Decompress Y
                let idx = usize::from((m << 1) | (gps_time_changed as usize));
                median = the_context.last_y_diff_median5[idx].get();
                k_bits = the_context.id_dx.k();
                let mut context = if n == 1 { 1 } else { 0 };
                context += if k_bits < 20 {
                    u32_zero_bit_0(k_bits)
                } else {
                    20
                };
                diff = the_context.id_dy.decompress(
                    &mut self.decoders.channel_returns_xy,
                    median,
                    context,
                )?;
                last_point.y = last_point.y.wrapping_add(diff);
                the_context.last_y_diff_median5[idx].add(diff);

                // Decompress Z
                if self.changed_z {
                    k_bits = (the_context.id_dx.k() + the_context.id_dy.k()) / 2;
                    let mut context = if n == 1 { 1 } else { 0 };
                    context += if k_bits < 18 {
                        u32_zero_bit_0(k_bits)
                    } else {
                        18
                    };
                    last_point.z = the_context.id_z.decompress(
                        &mut self.decoders.z,
                        the_context.last_z[l],
                        context,
                    )?;
                    the_context.last_z[l] = last_point.z;
                }

                // Decompress classification
                if self.changed_classification {
                    let last_classification = last_point.classification;
                    let ccc = (((last_classification & 0x1F) << 1) + (if cpr == 3 { 1 } else { 0 }))
                        as usize;
                    last_point.classification = self.decoders.classification.decode_symbol(
                        &mut the_context.classification_models[ccc]
                            .get_or_insert_with(|| ArithmeticModelBuilder::new(256).build()),
                    )? as u8;
                }

                // Decompress flags
                if self.changed_flags {
                    let last_flags = (last_point.edge_of_flight_line() as u8) << 5
                        | (last_point.scan_direction_flag() as u8) << 4
                        | last_point.classification_flags();
                    let last_flags = last_flags as usize;
                    let flags = self.decoders.flags.decode_symbol(
                        &mut the_context.classification_flags[last_flags]
                            .get_or_insert_with(|| ArithmeticModelBuilder::new(64).build()),
                    )?;

                    // FIXME
                    last_point.flags = ((flags >> 5 & 1) << 7
                        | (flags >> 4 & 1) << 6
                        | ((last_point.scanner_channel() << 4) & 0b00110000) as u32
                        | (flags & 0b00001111)) as u8;
                }

                if self.changed_intensity {
                    let idx = (cpr << 1 | (gps_time_changed as u32)) as usize;
                    last_point.intensity = the_context.id_intensity.decompress(
                        &mut self.decoders.intensity,
                        the_context.last_intensities[idx] as i32,
                        cpr,
                    )? as u16;
                    the_context.last_intensities[idx] = last_point.intensity;
                }

                if self.changed_scan_angle && scan_angle_changed {
                    last_point.scan_angle_rank = the_context.id_scan_angle.decompress(
                        &mut self.decoders.scan_angle,
                        last_point.scan_angle_rank as i32,
                        gps_time_changed as u32,
                    )? as u16;
                }

                if self.changed_user_data {
                    let user_data = self.decoders.user_data.decode_symbol(
                        the_context.user_data[(last_point.user_data / 4) as usize]
                            .get_or_insert_with(|| ArithmeticModelBuilder::new(256).build()),
                    )?;
                    last_point.set_user_data(user_data as u8);
                }

                if self.changed_point_source && point_source_changed {
                    last_point.point_source_id = the_context.id_source_id.decompress(
                        &mut self.decoders.point_source,
                        last_point.point_source_id as i32,
                        DEFAULT_DECOMPRESS_CONTEXTS,
                    )? as u16;
                }
                last_point.gps_time_change = gps_time_changed;
            }

            if self.changed_gps_time && gps_time_changed {
                self.read_gps_time()?;
                let gps_context = &self.contexts[self.current_context].gps_time_context;
                self.contexts[self.current_context].last_point.gps_time =
                    gps_context.last_gps_times[gps_context.last].gps_time();
            }

            let last_point = &mut self.contexts[self.current_context].last_point;
            current_point.set_fields_from(last_point);
            Ok(())
        }

        fn read_layers_sizes(&mut self, src: &mut R) -> std::io::Result<()> {
            self.layers_sizes = LayerSizes::read_from(src)?;
            Ok(())
        }

        fn read_layers(&mut self, mut src: &mut R) -> std::io::Result<()> {
            let num_bytes = &self.layers_sizes;

            copy_bytes_into_decoder(
                true, // Always decode x,y,channel & returns,
                num_bytes.channel_returns_xy,
                &mut self.decoders.channel_returns_xy,
                &mut src,
            )?;

            self.changed_z = copy_bytes_into_decoder(
                self.decompression_selector.z_requested(),
                num_bytes.z,
                &mut self.decoders.z,
                &mut src,
            )?;

            self.changed_classification = copy_bytes_into_decoder(
                self.decompression_selector.classification_requested(),
                num_bytes.classification,
                &mut self.decoders.classification,
                &mut src,
            )?;

            self.changed_flags = copy_bytes_into_decoder(
                self.decompression_selector.flags_requested(),
                num_bytes.flags,
                &mut self.decoders.flags,
                &mut src,
            )?;

            self.changed_intensity = copy_bytes_into_decoder(
                self.decompression_selector.intensity_requested(),
                num_bytes.intensity,
                &mut self.decoders.intensity,
                &mut src,
            )?;

            self.changed_scan_angle = copy_bytes_into_decoder(
                self.decompression_selector.scan_angle_requested(),
                num_bytes.scan_angle,
                &mut self.decoders.scan_angle,
                &mut src,
            )?;

            self.changed_user_data = copy_bytes_into_decoder(
                self.decompression_selector.user_data_requested(),
                num_bytes.user_data,
                &mut self.decoders.user_data,
                &mut src,
            )?;

            self.changed_point_source = copy_bytes_into_decoder(
                self.decompression_selector.point_source_requested(),
                num_bytes.point_source,
                &mut self.decoders.point_source,
                &mut src,
            )?;

            self.changed_gps_time = copy_bytes_into_decoder(
                self.decompression_selector.gps_time_requested(),
                num_bytes.gps_time,
                &mut self.decoders.gps_time,
                &mut src,
            )?;
            Ok(())
        }
    }

    impl_buffer_decompressor_for_typed_decompressor!(LasPoint6Decompressor, Point6);
}
