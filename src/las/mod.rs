#[macro_use]
mod utils;

pub mod extra_bytes;
pub mod file;
pub mod gps;
pub mod laszip;
pub mod nir;
pub mod point0;
pub mod point6;
mod pointtypes;
pub mod rgb;
pub mod rgbnir;

pub use pointtypes::{Point0, Point1, Point2, Point3, Point6, Point7};

pub mod v1 {
    pub use crate::las::extra_bytes::v1::{LasExtraByteCompressor, LasExtraByteDecompressor};
    pub use crate::las::gps::v1::{LasGpsTimeCompressor, LasGpsTimeDecompressor};
    pub use crate::las::point0::v1::{LasPoint0Compressor, LasPoint0Decompressor};
    pub use crate::las::rgb::v1::{LasRGBCompressor, LasRGBDecompressor};
}

pub mod v2 {
    pub use crate::las::extra_bytes::v2::{LasExtraByteCompressor, LasExtraByteDecompressor};
    pub use crate::las::gps::v2::{GpsTimeCompressor, GpsTimeDecompressor};
    pub use crate::las::point0::v2::{LasPoint0Compressor, LasPoint0Decompressor};
    pub use crate::las::rgb::v2::{LasRGBCompressor, LasRGBDecompressor};
}

pub mod v3 {
    pub use crate::las::extra_bytes::v3::LasExtraByteDecompressor;
    pub use crate::las::nir::v3::LasNIRDecompressor;
    pub use crate::las::point6::v3::LasPoint6Decompressor;
    pub use crate::las::rgb::v3::LasRGBDecompressor;
    pub use crate::las::rgbnir::v3::LasRGBNIRDecompressor;
}
