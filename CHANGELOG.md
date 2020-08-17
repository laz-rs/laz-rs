# Unreleased

 - Added LICENSE exception inherited from LASzip that allows static linking
 - Added a `std::result::Result<T, LasZipError>` 'typedef'
 - Added `ParLasZipCompressor` and `ParLasZipDecompressor` to the `parallel`feature.
   They can compress/decompress using multiple threads 'little by little' 
   (as opposed to existing previous functions that required the whole points 
   data to be read beforehand)
 - Changed LasZipError enum to be `#[non_exhaustive]`

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