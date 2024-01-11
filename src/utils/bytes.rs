use std::mem;

#[inline]
pub(crate) fn trim_ascii(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_ascii_whitespace())
}

pub(crate) struct Split<'a> {
    bytes: &'a [u8],
    separator: u8,
    iter: memchr::Memchr<'a>,
    next_start: usize,
    start: usize,
    end: usize,
}

impl<'a> Split<'a> {
    pub(crate) fn new(separator: u8, bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            separator,
            iter: memchr::memchr_iter(separator, bytes),
            next_start: 0,
            start: 0,
            end: 0,
        }
    }

    pub(crate) fn current(&self) -> &'a [u8] {
        self.bytes.get(self.start..self.end).unwrap_or_default()
    }

    #[cold]
    pub(crate) fn current_number_slow(&self) -> usize {
        self.bytes[..self.start]
            .iter()
            .filter(|&&b| b == self.separator)
            .count()
    }
}

impl<'a> Iterator for Split<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let line_end = match self.iter.next() {
            Some(line_end) => line_end,
            None => {
                self.bytes.get(self.next_start)?;
                self.bytes.len()
            }
        };
        self.end = line_end;
        self.start = mem::replace(&mut self.next_start, self.end + 1);
        Some(self.current())
    }
}

pub(crate) struct Lines<'a>(Split<'a>);

impl<'a> Lines<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self(Split::new(b'\n', bytes))
    }

    pub(crate) fn current(&self) -> &'a [u8] {
        self.0.current()
    }

    #[cold]
    pub(crate) fn line_number(&self) -> usize {
        self.0.current_number_slow()
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
