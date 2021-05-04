pub use crate::las::gps::LasGpsTime;
use crate::las::laszip::{DefaultVersion, LazItem, LazItemType, Version1, Version2, Version3};
use crate::las::nir::{Nir, LasNIR};
pub use crate::las::point0::{LasPoint0, Point0};
pub use crate::las::point6::{LasPoint6, Point6};
pub use crate::las::rgb::{LasRGB, RGB};
use crate::las::extra_bytes::LasExtraBytes;


pub trait LegacyLasPoint: LasPoint0 + LasRGB + LasGpsTime + LasExtraBytes {}
pub trait ExtendedLasPoint: LasPoint6 + LasRGB + LasNIR + LasExtraBytes {}
pub trait LasPoint: LegacyLasPoint + ExtendedLasPoint {}

pub trait Point0Based {
    fn point0(&self) -> &Point0;
    fn point0_mut(&mut self) -> &mut Point0;
}

pub trait Point6Based {
    fn point6(&self) -> &Point6;
    fn point6_mut(&mut self) -> &mut Point6;
}

macro_rules! vec_of_laz_items {
    (
        vec_capacity: $capacity:expr,
        extra_bytes_type: LazItemType::$EbType:ident($eb_num:expr), version: $eb_version:expr,
        $(LazItemType::$Type:ident, version: $version:expr),*
    ) => {{
        let mut items = Vec::<LazItem>::with_capacity($capacity);
        $(items.push(LazItem::new(LazItemType::$Type, $version));)*

        if $eb_num > 0 {
            items.push(
                LazItem::new(LazItemType::$EbType($eb_num), $eb_version)
            );
        }

        items
    }};
    (
        vec_capacity: $capacity:expr,
        version: $version:expr,
        extra_bytes_type: LazItemType::$EbType:ident($eb_num:expr),
        $(LazItemType::$Type:ident),*
    ) => {{
        vec_of_laz_items![
            vec_capacity: $capacity,
            extra_bytes_type: LazItemType::$EbType($eb_num), version: $version,
             $(LazItemType::$Type, version: $version),*
        ]
    }};
}

/***************************************************************************************************
                    Point Format 1
***************************************************************************************************/

impl Version2 for Point0 {
    fn version_2(num_extra_bytes: u16) -> Vec<LazItem> {
        vec_of_laz_items!(
            vec_capacity: 2,
            version: 2,
            extra_bytes_type: LazItemType::Byte(num_extra_bytes),
            LazItemType::Point10
        )
    }
}

impl Version1 for Point0 {
    fn version_1(num_extra_bytes: u16) -> Vec<LazItem> {
        vec_of_laz_items!(
            vec_capacity: 2,
            version: 1,
            extra_bytes_type: LazItemType::Byte(num_extra_bytes),
            LazItemType::Point10
        )
    }
}

impl DefaultVersion for Point0 {
    fn default_version(num_extra_bytes: u16) -> Vec<LazItem> {
        <Self as Version2>::version_2(num_extra_bytes)
    }
}

/***************************************************************************************************
                    Point Format 1
***************************************************************************************************/
#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub struct Point1 {
    base: Point0,
    gps_time: f64,
}

impl Point0Based for Point1 {
    fn point0(&self) -> &Point0 {
        &self.base
    }

    fn point0_mut(&mut self) -> &mut Point0 {
        &mut self.base
    }
}

impl LasGpsTime for Point1 {
    fn gps_time(&self) -> f64 {
        self.gps_time
    }

    fn set_gps_time(&mut self, new_value: f64) {
        self.gps_time = new_value;
    }
}

impl Version2 for Point1 {
    fn version_2(num_extra_bytes: u16) -> Vec<LazItem> {
        vec_of_laz_items!(
            vec_capacity: 3,
            version: 2,
            extra_bytes_type: LazItemType::Byte(num_extra_bytes),
            LazItemType::Point10,
            LazItemType::GpsTime
        )
    }
}

impl Version1 for Point1 {
    fn version_1(num_extra_bytes: u16) -> Vec<LazItem> {
        vec_of_laz_items!(
            vec_capacity: 3,
            version: 1,
            extra_bytes_type: LazItemType::Byte(num_extra_bytes),
            LazItemType::Point10,
            LazItemType::GpsTime
        )
    }
}

impl DefaultVersion for Point1 {
    fn default_version(num_extra_bytes: u16) -> Vec<LazItem> {
        <Self as Version2>::version_2(num_extra_bytes)
    }
}

/***************************************************************************************************
                    Point Format 2
***************************************************************************************************/

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub struct Point2 {
    base: Point0,
    rgb: RGB,
}

impl Point0Based for Point2 {
    fn point0(&self) -> &Point0 {
        &self.base
    }

    fn point0_mut(&mut self) -> &mut Point0 {
        &mut self.base
    }
}

impl LasRGB for Point2 {
    fn red(&self) -> u16 {
        self.rgb.red()
    }

    fn green(&self) -> u16 {
        self.rgb.green()
    }

    fn blue(&self) -> u16 {
        self.rgb.blue()
    }

    fn set_red(&mut self, new_val: u16) {
        self.rgb.set_red(new_val)
    }

    fn set_green(&mut self, new_val: u16) {
        self.rgb.set_green(new_val)
    }

    fn set_blue(&mut self, new_val: u16) {
        self.rgb.set_blue(new_val)
    }
}

impl Version2 for Point2 {
    fn version_2(num_extra_bytes: u16) -> Vec<LazItem> {
        vec_of_laz_items!(
            vec_capacity: 3,
            version: 2,
            extra_bytes_type: LazItemType::Byte(num_extra_bytes),
            LazItemType::Point10,
            LazItemType::RGB12
        )
    }
}

impl Version1 for Point2 {
    fn version_1(num_extra_bytes: u16) -> Vec<LazItem> {
        vec_of_laz_items!(
            vec_capacity: 3,
            version: 1,
            extra_bytes_type: LazItemType::Byte(num_extra_bytes),
            LazItemType::Point10,
            LazItemType::RGB12
        )
    }
}

impl DefaultVersion for Point2 {
    fn default_version(num_extra_bytes: u16) -> Vec<LazItem> {
        <Self as Version2>::version_2(num_extra_bytes)
    }
}

/***************************************************************************************************
                    Point Format 3
***************************************************************************************************/

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub struct Point3 {
    base: Point0,
    gps_time: f64,
    rgb: RGB,
}

impl Point0Based for Point3 {
    fn point0(&self) -> &Point0 {
        &self.base
    }

    fn point0_mut(&mut self) -> &mut Point0 {
        &mut self.base
    }
}

impl LasGpsTime for Point3 {
    fn gps_time(&self) -> f64 {
        self.gps_time
    }

    fn set_gps_time(&mut self, new_value: f64) {
        self.gps_time = new_value;
    }
}

impl LasRGB for Point3 {
    fn red(&self) -> u16 {
        self.rgb.red()
    }

    fn green(&self) -> u16 {
        self.rgb.green()
    }

    fn blue(&self) -> u16 {
        self.rgb.blue()
    }

    fn set_red(&mut self, new_val: u16) {
        self.rgb.set_red(new_val)
    }

    fn set_green(&mut self, new_val: u16) {
        self.rgb.set_green(new_val)
    }

    fn set_blue(&mut self, new_val: u16) {
        self.rgb.set_blue(new_val)
    }
}

impl Version2 for Point3 {
    fn version_2(num_extra_bytes: u16) -> Vec<LazItem> {
        vec_of_laz_items!(
            vec_capacity: 4,
            version: 2,
            extra_bytes_type: LazItemType::Byte(num_extra_bytes),
            LazItemType::Point10,
            LazItemType::GpsTime,
            LazItemType::RGB12
        )
    }
}

impl Version1 for Point3 {
    fn version_1(num_extra_bytes: u16) -> Vec<LazItem> {
        vec_of_laz_items!(
            vec_capacity: 4,
            version: 1,
            extra_bytes_type: LazItemType::Byte(num_extra_bytes),
            LazItemType::Point10,
            LazItemType::GpsTime,
            LazItemType::RGB12
        )
    }
}

impl DefaultVersion for Point3 {
    fn default_version(num_extra_bytes: u16) -> Vec<LazItem> {
        <Self as Version2>::version_2(num_extra_bytes)
    }
}

/***************************************************************************************************
                    Point Format 6
***************************************************************************************************/
impl Version3 for Point6 {
    fn version_3(num_extra_bytes: u16) -> Vec<LazItem> {
        vec_of_laz_items![
            vec_capacity: 2,
            version: 3,
            extra_bytes_type: LazItemType::Byte14(num_extra_bytes),
            LazItemType::Point14
        ]
    }
}

impl DefaultVersion for Point6 {
    fn default_version(num_extra_bytes: u16) -> Vec<LazItem> {
        Self::version_3(num_extra_bytes)
    }
}

/***************************************************************************************************
                    Point Format 7
***************************************************************************************************/

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub struct Point7 {
    base: Point6,
    rgb: RGB,
}

impl Point6Based for Point7 {
    fn point6(&self) -> &Point6 {
        &self.base
    }

    fn point6_mut(&mut self) -> &mut Point6 {
        &mut self.base
    }
}

impl LasRGB for Point7 {
    fn red(&self) -> u16 {
        self.rgb.red()
    }

    fn green(&self) -> u16 {
        self.rgb.green()
    }

    fn blue(&self) -> u16 {
        self.rgb.blue()
    }

    fn set_red(&mut self, new_val: u16) {
        self.rgb.set_red(new_val)
    }

    fn set_green(&mut self, new_val: u16) {
        self.rgb.set_green(new_val)
    }

    fn set_blue(&mut self, new_val: u16) {
        self.rgb.set_blue(new_val)
    }
}

impl Version3 for Point7 {
    fn version_3(num_extra_bytes: u16) -> Vec<LazItem> {
        vec_of_laz_items![
            vec_capacity: 3,
            version: 3,
            extra_bytes_type: LazItemType::Byte14(num_extra_bytes),
            LazItemType::Point14,
            LazItemType::RGB14
        ]
    }
}

impl DefaultVersion for Point7 {
    fn default_version(num_extra_bytes: u16) -> Vec<LazItem> {
        Self::version_3(num_extra_bytes)
    }
}

/***************************************************************************************************
                    Point Format 8
***************************************************************************************************/

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub struct Point8 {
    base: Point6,
    rgb: RGB,
    nir: Nir,
}

impl Version3 for Point8 {
    fn version_3(num_extra_bytes: u16) -> Vec<LazItem> {
        vec_of_laz_items![
            vec_capacity: 3,
            version: 3,
            extra_bytes_type: LazItemType::Byte14(num_extra_bytes),
            LazItemType::Point14,
            LazItemType::RGBNIR14
        ]
    }
}

impl DefaultVersion for Point8 {
    fn default_version(num_extra_bytes: u16) -> Vec<LazItem> {
        Self::version_3(num_extra_bytes)
    }
}

/***************************************************************************************************
                    Auto implementation of some traits
***************************************************************************************************/

impl<T: Point0Based> LasPoint0 for T {
    fn x(&self) -> i32 {
        self.point0().x()
    }

    fn y(&self) -> i32 {
        self.point0().y()
    }

    fn z(&self) -> i32 {
        self.point0().z()
    }

    fn intensity(&self) -> u16 {
        self.point0().intensity()
    }

    fn bit_fields(&self) -> u8 {
        self.point0().bit_fields()
    }

    fn number_of_returns_of_given_pulse(&self) -> u8 {
        self.point0().number_of_returns_of_given_pulse()
    }

    fn scan_direction_flag(&self) -> bool {
        self.point0().scan_direction_flag()
    }

    fn edge_of_flight_line(&self) -> bool {
        self.point0().edge_of_flight_line()
    }

    fn return_number(&self) -> u8 {
        self.point0().return_number()
    }

    fn classification(&self) -> u8 {
        self.point0().classification()
    }

    fn scan_angle_rank(&self) -> i8 {
        self.point0().scan_angle_rank()
    }

    fn user_data(&self) -> u8 {
        self.point0().user_data()
    }

    fn point_source_id(&self) -> u16 {
        self.point0().point_source_id()
    }

    fn set_x(&mut self, new_val: i32) {
        self.point0_mut().set_x(new_val);
    }
    fn set_y(&mut self, new_val: i32) {
        self.point0_mut().set_y(new_val);
    }
    fn set_z(&mut self, new_val: i32) {
        self.point0_mut().set_z(new_val);
    }

    fn set_intensity(&mut self, new_val: u16) {
        self.point0_mut().set_intensity(new_val);
    }

    fn set_bit_fields(&mut self, new_val: u8) {
        self.point0_mut().set_bit_fields(new_val);
    }

    fn set_classification(&mut self, new_val: u8) {
        self.point0_mut().set_classification(new_val)
    }

    fn set_scan_angle_rank(&mut self, new_val: i8) {
        self.point0_mut().set_scan_angle_rank(new_val)
    }

    fn set_user_data(&mut self, new_val: u8) {
        self.point0_mut().set_user_data(new_val)
    }

    fn set_point_source_id(&mut self, new_val: u16) {
        self.point0_mut().set_point_source_id(new_val)
    }
}

impl<T: Point6Based> LasPoint6 for T {
    fn x(&self) -> i32 {
        self.point6().x()
    }

    fn y(&self) -> i32 {
        self.point6().y()
    }

    fn z(&self) -> i32 {
        self.point6().z()
    }

    fn intensity(&self) -> u16 {
        self.point6().intensity()
    }

    fn bit_fields(&self) -> u8 {
        self.point6().bit_fields()
    }

    fn return_number(&self) -> u8 {
        self.point6().return_number()
    }

    fn number_of_returns_of_given_pulse(&self) -> u8 {
        self.point6().number_of_returns_of_given_pulse()
    }

    fn flags(&self) -> u8 {
        self.point6().flags()
    }

    fn classification_flags(&self) -> u8 {
        self.point6().classification_flags()
    }

    fn scanner_channel(&self) -> u8 {
        self.point6().scanner_channel()
    }

    fn scan_direction_flag(&self) -> bool {
        self.point6().scan_direction_flag()
    }

    fn edge_of_flight_line(&self) -> bool {
        self.point6().edge_of_flight_line()
    }

    fn classification(&self) -> u8 {
        self.point6().classification()
    }

    fn scan_angle_rank(&self) -> i16 {
        self.point6().scan_angle_rank()
    }

    fn user_data(&self) -> u8 {
        self.point6().user_data()
    }

    fn point_source_id(&self) -> u16 {
        self.point6().point_source_id()
    }

    fn gps_time(&self) -> f64 {
        self.point6().gps_time()
    }

    fn set_x(&mut self, new_val: i32) {
        self.point6_mut().set_x(new_val);
    }

    fn set_y(&mut self, new_val: i32) {
        self.point6_mut().set_y(new_val);
    }

    fn set_z(&mut self, new_val: i32) {
        self.point6_mut().set_z(new_val);
    }

    fn set_intensity(&mut self, new_val: u16) {
        self.point6_mut().set_intensity(new_val)
    }

    fn set_bit_fields(&mut self, new_val: u8) {
        self.point6_mut().set_bit_fields(new_val)
    }

    fn set_flags(&mut self, new_val: u8) {
        self.point6_mut().set_flags(new_val)
    }

    fn set_number_of_returns(&mut self, new_val: u8) {
        self.point6_mut().set_number_of_returns(new_val)
    }

    fn set_return_number(&mut self, new_val: u8) {
        self.point6_mut().set_return_number(new_val)
    }

    fn set_scanner_channel(&mut self, new_val: u8) {
        self.point6_mut().set_scanner_channel(new_val);
    }

    fn set_classification(&mut self, new_val: u8) {
        self.point6_mut().set_classification(new_val)
    }

    fn set_scan_angle_rank(&mut self, new_val: i16) {
        self.point6_mut().set_scan_angle_rank(new_val)
    }

    fn set_user_data(&mut self, new_val: u8) {
        self.point6_mut().set_user_data(new_val)
    }

    fn set_point_source_id(&mut self, new_val: u16) {
        self.point6_mut().set_point_source_id(new_val)
    }

    fn set_gps_time(&mut self, new_val: f64) {
        self.point6_mut().set_gps_time(new_val)
    }
}
