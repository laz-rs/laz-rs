use std::iter::FusedIterator;

/// Iterator over non-overlaping chunks of `&[u8]`, non-evenly-size
///
/// The idea is extremely similar to [std::slice::Chunks] however,
/// the chunks returned by this do not necessarily have the same size.
pub struct ChunksIrregular<'a> {
    remainder: &'a [u8],
    sizes: std::slice::Iter<'a, usize>,
}

impl<'a> ChunksIrregular<'a> {
    pub fn new(slc: &'a [u8], sizes: &'a [usize]) -> Self {
        Self {
            remainder: slc,
            sizes: sizes.iter(),
        }
    }
}

impl<'a> Iterator for ChunksIrregular<'a> {
    type Item = &'a [u8];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let size = self.sizes.next()?;
        let (head, tail) = self.remainder.split_at(*size);
        self.remainder = tail;
        Some(head)
    }
}

impl<'a> FusedIterator for ChunksIrregular<'a> {}

/// Iterator over non-overlaping chunks of `&mut [u8]`, non-evenly-size
///
/// The idea is extremely similar to [std::slice::ChunksMut] however,
/// the chunks returned by this do not necessarily have the same size.
pub struct ChunksIrregularMut<'a> {
    remainder: &'a mut [u8],
    sizes: std::slice::Iter<'a, usize>,
}

impl<'a> ChunksIrregularMut<'a> {
    pub fn new(slc: &'a mut [u8], sizes: &'a [usize]) -> Self {
        Self {
            remainder: slc,
            sizes: sizes.iter(),
        }
    }
}

impl<'a> Iterator for ChunksIrregularMut<'a> {
    type Item = &'a mut [u8];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Heavily inspired from the implementation of std::slice::ChunksMut
        let size = self.sizes.next()?;
        let tmp = std::mem::replace(&mut self.remainder, &mut []);
        let (head, tail) = tmp.split_at_mut(*size);
        self.remainder = tail;
        Some(head)
    }
}

impl<'a> FusedIterator for ChunksIrregularMut<'a> {}
