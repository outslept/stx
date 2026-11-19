use memchr::memchr_iter;

pub struct LineIndex {
    line_starts: Vec<u32>,
    is_ascii: bool,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let bytes = source.as_bytes();
        let mut line_starts = Vec::with_capacity(bytes.len() / 32 + 1);
        line_starts.push(0);

        for pos in memchr_iter(b'\n', bytes) {
            line_starts.push((pos + 1) as u32);
        }

        Self {
            line_starts,
            is_ascii: source.is_ascii(),
        }
    }

    #[inline]
    pub fn line_col(&self, source: &str, byte_offset: u32) -> (usize, usize) {
        let line_idx = self
            .line_starts
            .partition_point(|&start| start <= byte_offset)
            .saturating_sub(1);

        let line_start = self.line_starts[line_idx] as usize;
        let offset_usize = byte_offset as usize;
        let line_number = line_idx + 1;

        let col_number = if self.is_ascii {
            offset_usize - line_start + 1
        } else {
            source[line_start..offset_usize].chars().count() + 1
        };

        (line_number, col_number)
    }
}
