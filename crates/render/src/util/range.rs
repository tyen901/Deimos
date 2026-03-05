use std::ops::Range;

pub struct RangeChunks {
    range: Range<usize>,
    chunk_size: usize,
    index: usize,
}

impl RangeChunks {
    pub const fn new(range: Range<usize>, chunk_size: usize) -> Self {
        Self {
            range,
            chunk_size,
            index: 0,
        }
    }
}

impl Iterator for RangeChunks {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.index * self.chunk_size;
        let end = (start + self.chunk_size).min(self.range.end);
        if start >= end {
            return None;
        }

        self.index += 1;

        Some(start..end)
    }
}
