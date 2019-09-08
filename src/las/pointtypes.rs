use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;

pub use crate::las::gps::LasGpsTime;
pub use crate::las::point0::{LasPoint0, Point0};
pub use crate::las::point6::{LasPoint6, Point6};
pub use crate::las::rgb::{LasRGB, RGB};

pub trait Point0Based {
    fn point0(&self) -> &Point0;
    fn point0_mut(&mut self) -> &mut Point0;
}

pub trait Point6Based {
    fn point6(&self) -> &Point6;
    fn point6_mut(&mut self) -> &mut Point6;
}

/***************************************************************************************************
                    Point Format 1
***************************************************************************************************/
#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub struct Point1 {
    base: Point0,
    gps_time: f64,
}

impl Point1 {
    pub fn read_from<R: Read>(&mut self, mut src: &mut R) -> std::io::Result<()> {
        self.base.read_from(&mut src)?;
        self.gps_time = src.read_f64::<LittleEndian>()?;
        Ok(())
    }
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

/***************************************************************************************************
                    Point Format 2
***************************************************************************************************/

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub struct Point2 {
    base: Point0,
    rgb: RGB,
}

impl Point2 {
    pub fn read_from<R: Read>(&mut self, mut src: &mut R) -> std::io::Result<()> {
        self.base.read_from(&mut src)?;
        self.rgb.read_from(&mut src)?;
        Ok(())
    }
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

/***************************************************************************************************
                    Point Format 3
***************************************************************************************************/

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub struct Point3 {
    base: Point0,
    gps_time: f64,
    rgb: RGB,
}

impl Point3 {
    pub fn read_from<R: Read>(&mut self, mut src: &mut R) -> std::io::Result<()> {
        self.base.read_from(&mut src)?;
        self.gps_time = src.read_f64::<LittleEndian>()?;
        self.rgb.read_from(&mut src)?;
        Ok(())
    }
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

/***************************************************************************************************
                    Point Format 7
***************************************************************************************************/

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub struct Point7 {
    base: Point6,
    rgb: RGB,
}

impl Point7 {
    pub fn read_from<R: Read>(&mut self, src: &mut R) -> std::io::Result<()> {
        self.base.read_from(src)?;
        self.rgb.read_from(src)?;
        Ok(())
    }
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

    fn scan_angle_rank(&self) -> u16 {
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

    fn set_scan_angle_rank(&mut self, new_val: u16) {
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
