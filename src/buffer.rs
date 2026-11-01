//! Helpers for buffers whose size comes from the data being read
//!
//! As the data may be corrupted or malicious, these size cannot be blindly trusted
//! and so we have to avoid oom aborts
use std::io::Read;

/// Returns the size in bytes of `count` elements of `element_size` bytes,
/// or an `OutOfMemory` error if that size cannot be held in memory.
pub(crate) fn buffer_len(count: u64, element_size: u64) -> std::io::Result<usize> {
    count
        .checked_mul(element_size)
        .and_then(|len| usize::try_from(len).ok())
        .ok_or_else(|| std::io::ErrorKind::OutOfMemory.into())
}

/// Resizes `buf` to `new_len`, returning an `OutOfMemory` error
/// instead of aborting if the memory cannot be allocated.
pub(crate) fn try_resize(buf: &mut Vec<u8>, new_len: usize) -> std::io::Result<()> {
    if let Some(additional) = new_len.checked_sub(buf.len()) {
        buf.try_reserve_exact(additional)?;
    }
    buf.resize(new_len, 0);
    Ok(())
}

/// Replaces the content of `buf` with exactly `num_bytes` read from `src`.
///
/// `buf` is not allocated `num_bytes` upfront, it grows as bytes are read
/// (each step at most doubles it), so a bogus `num_bytes` results in an `UnexpectedEof`
/// error once `src` is exhausted, having allocated memory proportional to the bytes
/// that actually exist, not to `num_bytes`.
pub(crate) fn read_exact_into_vec<R: Read>(
    src: &mut R,
    buf: &mut Vec<u8>,
    num_bytes: usize,
) -> std::io::Result<()> {
    /// What may be allocated before knowing if the bytes exist
    const MIN_STEP: usize = 1 << 20;

    buf.clear();
    while buf.len() < num_bytes {
        let filled = buf.len();
        // Selects the largest number between MIN_STEP, memory already allocated
        // in the `buf` and number of bytes filled
        let step = filled.max(buf.capacity() - filled).max(MIN_STEP);
        // Cannot overflow, a Vec<u8> holds at most isize::MAX bytes
        try_resize(buf, num_bytes.min(filled + step))?;
        src.read_exact(&mut buf[filled..])?;
    }
    Ok(())
}

/// Reads and discards exactly `num_bytes` from `src`, without buffering them.
pub(crate) fn skip_exact<R: Read>(src: &mut R, num_bytes: u64) -> std::io::Result<()> {
    let num_skipped = std::io::copy(&mut src.by_ref().take(num_bytes), &mut std::io::sink())?;
    if num_skipped < num_bytes {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "failed to skip whole buffer",
        ));
    }
    Ok(())
}
