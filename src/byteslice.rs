use std::iter::FusedIterator;

/// Iterator over non-overlaping chunks of `&[u8]`, non-evenly-size
///
/// The idea is extremely similar to [std::slice::Chunks] however,
/// the chunks returned by this do not necessarily have the same size.
pub struct ChunksIrregular<'a, T> {
    remainder: &'a [u8],
    size_provider: T,
}

impl<'a, T> ChunksIrregular<'a, T> {
    pub fn new<P>(slc: &'a [u8], size_provider: P) -> Self
    where
        P: IntoIterator<IntoIter = T, Item = usize>,
        T: Iterator<Item = usize>,
    {
        Self {
            remainder: slc,
            size_provider: size_provider.into_iter(),
        }
    }
}

impl<'a, T> Iterator for ChunksIrregular<'a, T>
where
    T: Iterator<Item = usize>,
{
    type Item = &'a [u8];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let size = self.size_provider.next()?;
        let (head, tail) = self.remainder.split_at(size);
        self.remainder = tail;
        Some(head)
    }
}

impl<'a, T> FusedIterator for ChunksIrregular<'a, T> where T: Iterator<Item = usize> {}

/// Iterator over non-overlaping chunks of `&mut [u8]`, non-evenly-size
///
/// The idea is extremely similar to [std::slice::ChunksMut] however,
/// the chunks returned by this do not necessarily have the same size.
pub struct ChunksIrregularMut<'a, T> {
    remainder: &'a mut [u8],
    size_provider: T,
}

impl<'a, T> ChunksIrregularMut<'a, T> {
    pub fn new<P>(slc: &'a mut [u8], size_provider: P) -> Self
    where
        P: IntoIterator<IntoIter = T, Item = usize>,
        T: Iterator<Item = usize>,
    {
        Self {
            remainder: slc,
            size_provider: size_provider.into_iter(),
        }
    }
}

impl<'a, T> Iterator for ChunksIrregularMut<'a, T>
where
    T: Iterator<Item = usize>,
{
    type Item = &'a mut [u8];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Heavily inspired from the implementation of std::slice::ChunksMut
        let size = self.size_provider.next()?;
        let tmp = std::mem::replace(&mut self.remainder, &mut []);
        let (head, tail) = tmp.split_at_mut(size);
        self.remainder = tail;
        Some(head)
    }
}

impl<'a, T> FusedIterator for ChunksIrregularMut<'a, T> where T: Iterator<Item = usize> {}
