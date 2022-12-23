//! Selective decompression builder

/// Struct for selective decompression
///
/// Contains the information of which fields the user wants
/// to decompress or not.
///
/// # Note
///
/// Selective decompression is not supported by all
/// point formats. On point formats which do not support it,
/// it will be ignored and all data will be decompressed.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
// The inner value is pub to facilitate language bindings
pub struct DecompressionSelection(pub u32);

macro_rules! decompress_setter {
    (
        $fn_name:ident = $bit_mask:expr
    ) => {
        pub fn $fn_name(self) -> Self {
            self.set($bit_mask)
        }
    };
}

macro_rules! skip_setter {
    (
        $fn_name:ident = $bit_mask:expr
    ) => {
        pub fn $fn_name(self) -> Self {
            self.unset($bit_mask)
        }
    };
}

macro_rules! accessor {
    (
        $fn_name:ident = $bit_mask:expr
    ) => {
        pub fn $fn_name(self) -> bool {
            self.is_set($bit_mask)
        }
    };
}

// The consts are pub to facilitate bindings with other languages
//
// # Note
//
// The values are not 100% the same as laszip API
impl DecompressionSelection {
    pub const ALL: u32 = u32::MAX;

    // These are always decompressed
    // and cannot be skipped
    pub const XY_RETURNS_CHANNEL: u32 = 0;
    pub const Z: u32 = 1 << 0;
    pub const CLASSIFICATION: u32 = 1 << 1;
    pub const FLAGS: u32 = 1 << 2;
    pub const INTENSITY: u32 = 1 << 3;
    pub const SCAN_ANGLE: u32 = 1 << 4;
    pub const USER_DATA: u32 = 1 << 5;
    pub const POINT_SOURCE_ID: u32 = 1 << 6;
    pub const GPS_TIME: u32 = 1 << 7;
    pub const RGB: u32 = 1 << 8;
    pub const NIR: u32 = 1 << 9;
    pub const WAVEPACKET: u32 = 1 << 10;
    pub const ALL_EXTRA_BYTES: u32 = 1 << 11;

    /// To decompress all the possible fields
    pub fn all() -> Self {
        Self(Self::ALL)
    }

    /// To decompress only the 'base' fields
    /// that is:
    ///
    /// - x
    /// - y
    /// - return number
    /// - number of returns
    /// - scanner channel
    pub fn base() -> Self {
        Self::xy_returns_channel()
    }

    pub fn xy_returns_channel() -> Self {
        Self(Self::XY_RETURNS_CHANNEL)
    }

    decompress_setter!(decompress_z = Self::Z);
    decompress_setter!(decompress_classification = Self::CLASSIFICATION);
    decompress_setter!(decompress_flags = Self::FLAGS);
    decompress_setter!(decompress_intensity = Self::INTENSITY);
    decompress_setter!(decompress_scan_angle = Self::SCAN_ANGLE);
    decompress_setter!(decompress_user_data = Self::USER_DATA);
    decompress_setter!(decompress_point_source_id = Self::POINT_SOURCE_ID);
    decompress_setter!(decompress_gps_time = Self::GPS_TIME);
    decompress_setter!(decompress_rgb = Self::RGB);
    decompress_setter!(decompress_nir = Self::NIR);
    decompress_setter!(decompress_wavepacket = Self::WAVEPACKET);
    decompress_setter!(decompress_extra_bytes = Self::ALL_EXTRA_BYTES);

    skip_setter!(skip_z = Self::Z);
    skip_setter!(skip_classification = Self::CLASSIFICATION);
    skip_setter!(skip_flags = Self::FLAGS);
    skip_setter!(skip_intensity = Self::INTENSITY);
    skip_setter!(skip_scan_angle = Self::SCAN_ANGLE);
    skip_setter!(skip_user_data = Self::USER_DATA);
    skip_setter!(skip_point_source_id = Self::POINT_SOURCE_ID);
    skip_setter!(skip_gps_time = Self::GPS_TIME);
    skip_setter!(skip_rgb = Self::RGB);
    skip_setter!(skip_nir = Self::NIR);
    skip_setter!(skip_wavepacket = Self::WAVEPACKET);
    skip_setter!(skip_extra_bytes = Self::ALL_EXTRA_BYTES);

    accessor!(should_decompress_z = Self::Z);
    accessor!(should_decompress_classification = Self::CLASSIFICATION);
    accessor!(should_decompress_flags = Self::FLAGS);
    accessor!(should_decompress_intensity = Self::INTENSITY);
    accessor!(should_decompress_scan_angle = Self::SCAN_ANGLE);
    accessor!(should_decompress_user_data = Self::USER_DATA);
    accessor!(should_decompress_point_source_id = Self::POINT_SOURCE_ID);
    accessor!(should_decompress_gps_time = Self::GPS_TIME);
    accessor!(should_decompress_rgb = Self::RGB);
    accessor!(should_decompress_nir = Self::NIR);
    accessor!(should_decompress_wavepacket = Self::WAVEPACKET);
    accessor!(should_decompress_extra_bytes = Self::ALL_EXTRA_BYTES);

    fn set(self, bit_mask: u32) -> Self {
        Self(self.0 | bit_mask)
    }

    fn unset(self, bit_mask: u32) -> Self {
        Self(self.0 & (!bit_mask))
    }

    fn is_set(self, bit_mask: u32) -> bool {
        (self.0 & bit_mask) != 0
    }
}
