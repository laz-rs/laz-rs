pub use compression::{par_compress, par_compress_buffer, ParLasZipCompressor};
pub use decompression::{par_decompress_buffer, ParLasZipDecompressor};

mod compression;
mod decompression;
