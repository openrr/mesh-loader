use std::{io, marker::PhantomData};

#[cfg(test)]
#[path = "tests/int.rs"]
mod tests;

pub(crate) fn parse_partial<T: Int>(bytes: &[u8]) -> Option<(T, usize)> {
    T::parse_partial(bytes)
}

pub(crate) trait Int: Sized {
    const MAX_DIGIT_COUNT: usize;

    fn parse_partial(bytes: &[u8]) -> Option<(Self, usize)>;
}

macro_rules! uint {
    ($ty:ident, $max_digit_count:expr) => {
        impl Int for $ty {
            const MAX_DIGIT_COUNT: usize = $max_digit_count;

            // Source: https://github.com/fastfloat/fast_float/issues/86#issuecomment-866329749
            fn parse_partial(bytes: &[u8]) -> Option<(Self, usize)> {
                const LIMIT: $ty = (10 as $ty).pow($max_digit_count - 1);

                let mut n: $ty = 0;
                let mut digit_count = 0;
                for &b in bytes {
                    match parse_digit(b) {
                        Some(digit) => {
                            // might overflow, we will handle the overflow later
                            n = n.wrapping_mul(10).wrapping_add(digit as $ty);
                            digit_count += 1;
                        }
                        None => break,
                    }
                }
                if digit_count == 0 || digit_count > Self::MAX_DIGIT_COUNT {
                    return None;
                }
                if digit_count == Self::MAX_DIGIT_COUNT && n < LIMIT {
                    return None;
                }
                Some((n, digit_count))
            }
        }
    };
}

uint!(u128, 39);
uint!(u64, 20);
uint!(u32, 10);
uint!(u16, 5);
uint!(u8, 3);
// int!(i128, 39);
// int!(i64, 19);
// int!(i32, 10);
// int!(i16, 5);
// int!(i8, 3);

#[inline]
fn parse_digit(b: u8) -> Option<u8> {
    let digit = b.wrapping_sub(b'0');
    if digit > 9 {
        return None;
    }
    Some(digit)
}

/// Parses integer array "<int> <int> <int>...".
pub(crate) fn parse_array<T>(text: &str) -> ParseIntArray<'_, T>
where
    T: Int,
{
    ParseIntArray {
        text: text.trim_start(),
        _marker: PhantomData,
    }
}

pub(crate) struct ParseIntArray<'a, T> {
    text: &'a str,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Iterator for ParseIntArray<'_, T>
where
    T: Int,
{
    type Item = io::Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.text.is_empty() {
            return None;
        }
        match parse_partial(self.text.as_bytes()) {
            Some((value, n)) => {
                self.text = self.text.get(n..).unwrap_or_default().trim_start();
                Some(Ok(value))
            }
            None => Some(Err(format_err!("error while parsing an integer"))),
        }
    }
}
