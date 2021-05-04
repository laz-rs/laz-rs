//! This module contains re-exports of the different version of
//! LAS data compressors & decompressors as well as
//! the definition of the point types.

#[macro_use]
mod utils;
mod pointtypes;

pub mod point0;
pub mod point6;

pub mod extra_bytes;
pub mod file;
pub mod gps;
pub mod laszip;
pub mod nir;
pub mod rgb;

pub use pointtypes::{Point0, Point1, Point2, Point3, Point6, Point7, Point8, LasPoint};

pub mod v1 {
    //! This module only contains re exports of compressors / decompressors
    //! of the corresponding version for easier access
    pub use crate::las::extra_bytes::v1::{LasExtraByteCompressor, LasExtraByteDecompressor};
    pub use crate::las::gps::v1::{LasGpsTimeCompressor, LasGpsTimeDecompressor};
    pub use crate::las::point0::v1::{LasPoint0Compressor, LasPoint0Decompressor};
    pub use crate::las::rgb::v1::{LasRGBCompressor, LasRGBDecompressor};
}

pub mod v2 {
    //! This module only contains re exports of compressors / decompressors
    //! of the corresponding version for easier access
    pub use crate::las::extra_bytes::v2::{LasExtraByteCompressor, LasExtraByteDecompressor};
    pub use crate::las::gps::v2::{GpsTimeCompressor, GpsTimeDecompressor};
    pub use crate::las::point0::v2::{LasPoint0Compressor, LasPoint0Decompressor};
    pub use crate::las::rgb::v2::{LasRGBCompressor, LasRGBDecompressor};
}

pub mod v3 {
    //! This module only contains re exports of compressors / decompressors
    //! of the corresponding version for easier access
    pub use crate::las::extra_bytes::v3::{LasExtraByteCompressor, LasExtraByteDecompressor};
    pub use crate::las::nir::v3::{LasNIRCompressor, LasNIRDecompressor};
    pub use crate::las::point6::v3::{LasPoint6Compressor, LasPoint6Decompressor};
    pub use crate::las::rgb::v3::{LasRGBCompressor, LasRGBDecompressor};
}
