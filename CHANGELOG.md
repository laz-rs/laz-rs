# 0.9.3

- Fixed Initialization of LasZipAppender when the input file is empty (no points)

# 0.9.2

- Fixed Initialization of booleans that tracked whether a field
   changed when compressing fields from point format 6 (so some fields of
   point format 7, 8, 9, 10 also affected).
   This only occurred if the fields values did not change AND where not all 0,
   i.e:
      point_source_id.iter().all(|value| value == 0) -> no problem
      point_source_id.iter().all(|value| value == 1) -> problem

- Fixed the AC_BUFFER_SIZE value from 1024 to 4096.
   4096 is the value used in LASZip and if we don't use this value
   some of the compressed values won't be correct.
   This only seemed to affect the user_data field in point format >= 6

# 0.9.1

- Add `decompress_one` to LazDecompressor trait
- Add `compress_one` to LazCompressor trait

# 0.9.0

- Add LasZipAppender & ParLasZipAppender, structs that allow to easily append points to
  LAZ data

# 0.8.3

- Fixed seeking to a point (with format >= 6) that falls in the last chunk of a file.

# 0.8.2

- Fixed creation of LazVlr for point formats with Wavepacket
 (id 4, 5, 9, 10).

# 0.8.1

- Fixed potential division by zero on bad data.
   A division by zero could sometimes happen in the arithmetic decoder
   when decompressing invalid data, this would trigger a panic.
   Now an End Of File error will be returned.

# 0.8.0

- Added support for Wavepacket compression/decompression
   for point format 4 & 5 (compressor version 1 & 2) and
   point format 9 & 10 (compressor version 3)
- Added support for selective decompression, which enables to selectively decompress or not
   some fields. Only works on layered fields (point format >= 6)

# 0.7.0

- Changed the license from LGPL to Apache 2.0 as LASZip (the projects laz-rs derives from)
   was re-licensed to Apache 2.0
- Fixed seeking to a point that is a multiple of chunk size.
- Fixed seeking in variable size chunk for `ParLasZipDecompressor`

# 0.6.4

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
