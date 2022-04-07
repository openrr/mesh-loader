use std::{io, marker::PhantomData};

/// Parses float array "<float> <float> <float>..."
#[allow(dead_code)] // TODO
pub(crate) fn parse_array<T>(text: &str) -> ParseFloatArray<'_, T>
where
    T: fast_float::FastFloat,
{
    ParseFloatArray {
        text: text.trim_start(),
        _marker: PhantomData,
    }
}

pub(crate) struct ParseFloatArray<'a, T> {
    text: &'a str,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Iterator for ParseFloatArray<'_, T>
where
    T: fast_float::FastFloat,
{
    type Item = fast_float::Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.text.is_empty() {
            return None;
        }
        match fast_float::parse_partial::<T, _>(self.text.as_bytes()) {
            Ok((value, n)) => {
                self.text = self.text.get(n..).unwrap_or_default().trim_start();
                Some(Ok(value))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

/// Parses float array "<float> <float> <float>..."
pub(crate) fn parse_array_exact<T>(text: &str, num: usize) -> ParseFloatArrayExact<'_, T>
where
    T: fast_float::FastFloat,
{
    ParseFloatArrayExact {
        text: text.trim_start(),
        num,
        count: 0,
        _marker: PhantomData,
    }
}

pub(crate) struct ParseFloatArrayExact<'a, T> {
    text: &'a str,
    num: usize,
    count: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Iterator for ParseFloatArrayExact<'_, T>
where
    T: fast_float::FastFloat,
{
    type Item = io::Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count >= self.num {
            if self.text.is_empty() {
                return None;
            }
            return Some(Err(format_err!(
                "unexpected text {:?} after {} floats",
                self.text,
                self.num
            )));
        }
        match fast_float::parse_partial::<T, _>(self.text.as_bytes()) {
            Ok((value, n)) => {
                self.text = self.text.get(n..).unwrap_or_default().trim_start();
                self.count += 1;
                Some(Ok(value))
            }
            Err(e) => Some(Err(crate::error::invalid_data(e))),
        }
    }
}
