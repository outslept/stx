use memchr::memchr_iter;

pub struct LineIndex {
    line_starts: Vec<u32>,
    is_ascii: bool,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let bytes = source.as_bytes();
        let is_ascii = source.is_ascii();
        let mut line_starts = Vec::with_capacity(bytes.len() / 32 + 1);
        line_starts.push(0);
        for pos in memchr_iter(b'\n', bytes) {
            line_starts.push((pos + 1) as u32);
        }
        Self {
            line_starts,
            is_ascii,
        }
    }

    #[inline]
    pub fn line_col(&self, source: &str, byte_offset: u32) -> (usize, usize) {
        let offset_usize = (byte_offset as usize).min(source.len());
        let clamped_offset = offset_usize as u32;
        let line_idx = self
            .line_starts
            .partition_point(|&start| start <= clamped_offset)
            .saturating_sub(1);
        let line_start = self.line_starts[line_idx] as usize;
        let line_number = line_idx + 1;
        let col_number = if self.is_ascii {
            offset_usize.saturating_sub(line_start) + 1
        } else {
            source
                .get(line_start..offset_usize)
                .map_or(0, |s| s.chars().count())
                + 1
        };
        (line_number, col_number)
    }
}

#[cfg(test)]
mod tests {
    use super::LineIndex;

    #[test]
    fn handles_mixed_line_endings_and_clamps_past_eof() {
        let source = "a\r\n\nb\n";
        let index = LineIndex::new(source);
        for (offset, expected) in [
            (0, (1, 1)),
            (3, (2, 1)),
            (4, (3, 1)),
            (5, (3, 2)),
            (6, (4, 1)),
            (u32::MAX, (4, 1)),
        ] {
            assert_eq!(index.line_col(source, offset), expected, "offset {offset}");
        }
        assert_eq!(LineIndex::new("").line_col("", 0), (1, 1));
    }

    #[test]
    fn counts_unicode_scalars_instead_of_utf8_bytes() {
        let source = "é😀x\r\n界z";
        let index = LineIndex::new(source);
        for (offset, expected) in [
            (2, (1, 2)),
            (6, (1, 3)),
            (9, (2, 1)),
            (12, (2, 2)),
            (13, (2, 3)),
        ] {
            assert_eq!(index.line_col(source, offset), expected, "offset {offset}");
        }
    }
}
