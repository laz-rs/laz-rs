# Unreleased
    - Changed the 'static lifetime requirements on the generic io types
    (std::io::Read, std::io::Write) it allows to create LasZipCompressors
    and LasZipDecompressors on std::io::Cursor(&[u8]), which will be needed to create a C FFI
    - Added version 3 of the compression and decompression for Point6, RGB,
        Nir, ExtraBytes)
    - Added functions to compress or decompress all points contained in a buffer
    - Added a parallel optional feature (off by default) that gives
    access to functions to compress or decompress points using multiple threads.
    It uses the rayon crate. 

# 0.0.1
    - Added implementation of version 1 & version 2
        of laszip for point formats 0, 1, 2, 3 (Point0, RGB, GpsTime ExtraBytes)