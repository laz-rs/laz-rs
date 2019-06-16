pub mod extra_bytes;
pub mod gps;
pub mod laszip;
pub mod point10;
pub mod rgb;

mod utils;

pub mod v1 {
    pub use crate::las::point10::v1::{Point10Compressor, Point10Decompressor};
    pub use crate::las::gps::v1::{GpsTimeCompressor, GpsTimeDecompressor};
    pub use crate::las::rgb::v1::{RGBCompressor, RGBDecompressor};
    pub use crate::las::extra_bytes::v1::{ExtraBytesCompressor, ExtraBytesDecompressor};
}

pub mod v2 {
    pub use crate::las::point10::v2::{Point10Compressor, Point10Decompressor};
    pub use crate::las::gps::v2::{GpsTimeCompressor, GpsTimeDecompressor};
    pub use crate::las::rgb::v2::{RGBCompressor, RGBDecompressor};
    pub use crate::las::extra_bytes::v2::{ExtraBytesCompressor, ExtraBytesDecompressor};
}
