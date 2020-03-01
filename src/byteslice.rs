use std::iter::FusedIterator;

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
