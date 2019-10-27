pub(crate) mod compressors;
pub(crate) mod decoders;
pub(crate) mod decompressors;
pub(crate) mod encoders;
pub(crate) mod models;

pub mod checking;
pub mod errors;
pub mod las;
pub mod packers;
#[macro_use]
pub mod record;

pub use errors::LasZipError;


use crate::las::laszip::compress_all as rust_compress_all;
use std::io::Cursor;
use std::cell::RefCell;

thread_local! {
    pub static LAST_ERROR_MESSAGE: RefCell<Option<String>> = RefCell::new(None);
}

#[repr(C)]
pub struct BytesBuffer {
    ptr: *mut u8,
    size: usize,
    capacity: usize,
}

#[no_mangle]
pub extern "C" fn free_bytes_buffer(bb: BytesBuffer) {
    unsafe {
        if !bb.ptr.is_null() {
            drop(Vec::from_raw_parts(bb.ptr, bb.size, bb.capacity));
        }
    }
}

#[no_mangle]
pub extern "C" fn compress_all(
    in_uncompressed_points: *const u8,
    uncompressed_points_buffer_size: usize,
    laszip_vlr_record_data: *const u8,
    record_data_size: usize,
) -> * mut BytesBuffer {

    let in_uncompressed_points = unsafe {
        std::slice::from_raw_parts(in_uncompressed_points, uncompressed_points_buffer_size)
    };

    let laszip_vlr_record_data = unsafe {
        std::slice::from_raw_parts(laszip_vlr_record_data, record_data_size)
    };

    let laz_vlr = match crate::las::laszip::LazVlr::from_buffer(laszip_vlr_record_data) {
        Ok(vlr) => vlr,
        Err(e) => {
            LAST_ERROR_MESSAGE.with(|value| value.replace_with(|_old| Some(format!("{}", e))));
            return std::ptr::null_mut();
        }
    };
    let mut compression_output = Cursor::new(Vec::<u8>::new());
    if let Err(e) = rust_compress_all(&mut compression_output, in_uncompressed_points, laz_vlr) {
        LAST_ERROR_MESSAGE.with(|value| value.replace_with(|_old| Some(format!("{}", e))));
        std::ptr::null_mut()
    } else {
        let mut vec = compression_output.into_inner();
        let bb = Box::new(BytesBuffer {
            ptr: vec.as_mut_ptr(),
            size: vec.len(),
            capacity: vec.capacity(),
        });
        std::mem::forget(vec);
        Box::into_raw(bb)
    }
}

