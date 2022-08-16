# Unreleased
  - Fixed compression of RGB and NIR for point format >= 6 where the NIR/RBG was
    always the same value.
  - Fixed compression and decompression of extra bytes for point format >= 6.

# 0.6.3
  - Added `ChunkTable::read` to public API.
  - Fixed support for non seekable stream (fix contained in 0.6.2 was incomplete)

# 0.6.2
  - Added `par_decompress` to the public API. (`parallel` feature required)
  - Fixed `LasZipDecompressor` to still be able to read points even when the source
    is not seekable. Seeking is only required to get the ChunkTable, and the ChunkTable
    is only required when wanting to use `LasZipDecompressor.seek` to get a point.

# 0.6.1
  - Added support for `PointWise` compressed data in `LasZipDecompressor`.
  - Fixed `LasZipCompressor` when no points where compressed.

# 0.6.0
  - Added a `seek` method to `ParLasZipDecompressor`.
  - Added `reserve_offset_to_chunk_table` to the `LasZipCompressor`& `ParLasZipCompressor` API.
  - Added `std::error::Error` implementation for `LasZipError`.
  - Added a `LazCompressor` & `LazDecompressor` traits handle the non-parallel & parallel compressors/decompressor using 
    generics.
  - Added **variable-size** chunks support for `LasZipDecompressor`, `ParLasZipDecompressor`, `LasZipCompressor` and `ParLasZipCompressor`.
  - Fixed memory usage of `ParLasZipCompressor` and `ParLasZipDecompressor`
    and slightly improve their performance as side effect. (commit 955d0938eb385966b85f0685b0e7719aa2c5fa4e, PR #23)
  - Removed `BuffeLenNotMultipleOfPointSize` error kind.
  - Removed `Remove LasZipDecompressor::new_with_record_data`

# 0.5.2
  - Changed: Ensure LasZipCompressor, LasZipDecompressor, ParLasZipCompressor, ParLasZipDecompressor
    all have `into_inner` `get` `get_mut`.

# 0.5.1
  - Fixed Scan Angle in point 6, 7, 8. It was treated as u16 instead of i16.

# 0.5.0
  - Added `laz::write_chunk_table` to the public API
  - Fixed `read_chunk_table` to catch more cases of when the chunk table is not written
    in the file
  - Changed the `into_stream` fn of `LasZipDecompressor` and `LasZipCompressor` to `into_inner` to
    match naming done in rust's std lib
  - Changed `LasZipCompressor::new_with_record_data` to `LasZipCompressor::new`
  - Changed `LasZipDecompressor` and `LasZipCompressor` to be `Send` which causes now
    the source and destination of the compressor/decompressor to also be marked `Send`.
    (Which is the case for `std::fs::File` and `std::io::Cursor<T>` if `T`is `Send`, which
    both are the source and dest used in 99.99% of the time) 

# 0.4.0

 - Added LICENSE exception inherited from LASzip that allows static linking
 - Added a `std::result::Result<T, LasZipError>` 'typedef'
 - Added `ParLasZipCompressor` and `ParLasZipDecompressor` to the `parallel` feature.
   They can compress/decompress using multiple threads 'little by little' 
   (as opposed to existing previous functions that required the whole points 
   data to be read beforehand)
 - Changed `LasZipError` enum to be `#[non_exhaustive]`

# 0.3.0
 - Added UnsupportedPointFormat error variant
 - Added `compress_many` to LasZipCompressor
 - Added `decompress_many` to LasZipDecompressor
 - Added `par_compress_buffer` to compress points from a buffer using multiple threads
 - Added `par_decompress_buffer` to decompress points using multiple threads
 - Changed `laz::las::laszip::compress_all` to `compress_buffer`
  and add it the re-exported functions.
 - Updated the documentation.

# 0.2.0
 - Changed the 'static lifetime requirements on the generic io types
      (std::io::Read, std::io::Write) it allows to create LasZipCompressors
    and LasZipDecompressors on std::io::Cursor(&[u8]), which will be needed to create a C FFI
 - Added version 3 of the compression and decompression for Point6, RGB,
        Nir, ExtraBytes)
 - Added functions to compress all points contained in a buffer
 - Added a parallel optional feature (off by default) that gives
   access to functions to compress or decompress points using multiple threads.
   It uses the rayon crate. 

# 0.1.0
 - Added implementation of version 1 & version 2
        of laszip for point formats 0, 1, 2, 3 (Point0, RGB, GpsTime ExtraBytes)
