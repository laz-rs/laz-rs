use std::io::Read;

use crate::las::rgb::LasRGB;
use byteorder::{LittleEndian, ReadBytesExt};

pub mod extra_bytes;
pub mod gps;
pub mod laszip;
pub mod point10;
pub mod rgb;

pub use point10::{LasPoint0, Point0};

mod utils;

pub trait Point0Based {
    fn point0(&self) -> &Point0;
    fn point0_mut(&mut self) -> &mut Point0;
}

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub struct Point1 {
    base: point10::Point0,
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

impl gps::LasGpsTime for Point1 {
    fn gps_time(&self) -> f64 {
        self.gps_time
    }

    fn set_gps_time(&mut self, new_value: f64) {
        self.gps_time = new_value;
    }
}

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub struct Point2 {
    base: point10::Point0,
    rgb: rgb::RGB,
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

impl rgb::LasRGB for Point2 {
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

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub struct Point3 {
    base: point10::Point0,
    gps_time: f64,
    rgb: rgb::RGB,
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

impl gps::LasGpsTime for Point3 {
    fn gps_time(&self) -> f64 {
        self.gps_time
    }

    fn set_gps_time(&mut self, new_value: f64) {
        self.gps_time = new_value;
    }
}

impl rgb::LasRGB for Point3 {
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

pub mod v1 {
    pub use crate::las::extra_bytes::v1::{LasExtraByteCompressor, LasExtraByteDecompressor};
    pub use crate::las::gps::v1::{LasGpsTimeCompressor, LasGpsTimeDecompressor};
    pub use crate::las::point10::v1::{LasPoint0Compressor, LasPoint0Decompressor};
    pub use crate::las::rgb::v1::{LasRGBCompressor, LasRGBDecompressor};
}

pub mod v2 {
    pub use crate::las::extra_bytes::v2::{LasExtraByteCompressor, LasExtraByteDecompressor};
    pub use crate::las::gps::v2::{GpsTimeCompressor, GpsTimeDecompressor};
    pub use crate::las::point10::v2::{LasPoint0Compressor, LasPoint0Decompressor};
    pub use crate::las::rgb::v2::{LasRGBCompressor, LasRGBDecompressor};
}

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
