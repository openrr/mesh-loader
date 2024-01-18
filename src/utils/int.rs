// Rust port of fast_float's integer parser.
//
// Source: https://github.com/fastfloat/fast_float/blob/68b9475585be0839fa0bf3d6bfad3e4a6357d90a/include/fast_float/ascii_number.h#L445

#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use self::integer::RawInteger;
use crate::utils::float::{common::ByteSlice, parse::try_parse_digits};

#[inline]
pub fn parse<T: Integer>(bytes: &[u8]) -> Option<T> {
    T::parse(bytes)
}

#[inline]
pub fn parse_partial<T: Integer>(bytes: &[u8]) -> Option<(T, usize)> {
    T::parse_partial(bytes)
}

mod integer {
    pub trait RawInteger: Copy {
        const MAX_DIGITS: usize;
        const MIN_SAFE: u64;
        const MAX: u64;
        const IS_SIGNED: bool;
        fn from_u64(v: u64, negative: bool) -> Self;
    }
}

pub trait Integer: integer::RawInteger {
    #[inline]
    fn parse(bytes: &[u8]) -> Option<Self> {
        match Self::parse_partial(bytes) {
            Some((v, n)) if n == bytes.len() => Some(v),
            _ => None,
        }
    }
    #[inline]
    fn parse_partial(bytes: &[u8]) -> Option<(Self, usize)> {
        dec2int(bytes)
    }
}

const BASE: u8 = 10;

macro_rules! max_digit_count {
    ($ty:ident) => {{
        let mut max = $ty::MAX;
        let mut count = 0;
        while max > 0 {
            count += 1;
            max /= BASE as $ty;
        }
        count
    }};
}

macro_rules! uint {
    ($ty:ident) => {
        impl RawInteger for $ty {
            const MAX_DIGITS: usize = max_digit_count!($ty);
            const MIN_SAFE: u64 = (BASE as u64).pow($ty::MAX_DIGITS as u32 - 1);
            const MAX: u64 = $ty::MAX as u64;
            const IS_SIGNED: bool = false;
            #[inline]
            fn from_u64(v: u64, negative: bool) -> Self {
                debug_assert!(!negative);
                v as $ty
            }
        }
        impl Integer for $ty {}
    };
}
macro_rules! int {
    ($ty:ident) => {
        impl RawInteger for $ty {
            const MAX_DIGITS: usize = max_digit_count!($ty);
            const MIN_SAFE: u64 = (BASE as u64).pow($ty::MAX_DIGITS as u32 - 1);
            const MAX: u64 = $ty::MAX as u64;
            const IS_SIGNED: bool = true;
            #[inline]
            fn from_u64(v: u64, negative: bool) -> Self {
                if negative {
                    (-$ty::MAX).wrapping_sub((v.wrapping_sub($ty::MAX as u64)) as $ty)
                } else {
                    v as $ty
                }
            }
        }
        impl Integer for $ty {}
    };
}
// uint!(u128);
uint!(u64);
uint!(u32);
uint!(u16);
uint!(u8);
// int!(i128);
int!(i64);
int!(i32);
int!(i16);
int!(i8);

#[inline]
fn dec2int<I: RawInteger>(mut s: &[u8]) -> Option<(I, usize)> {
    let start = s;
    let c = if let Some(&c) = s.first() {
        c
    } else {
        return None;
    };
    let negative;
    if I::IS_SIGNED {
        negative = c == b'-';
        if negative || c == b'+' {
            s = &s[1..];
            if s.is_empty() {
                return None;
            }
        }
    } else {
        negative = false;
        if c == b'+' {
            s = &s[1..];
            if s.is_empty() {
                return None;
            }
        }
    }

    let (v, len) = parse_partial_number(s, start, negative, I::MAX_DIGITS, I::MIN_SAFE, I::MAX)?;
    Some((I::from_u64(v, negative), len))
}

#[inline(always)]
fn parse_partial_number(
    mut s: &[u8],
    full_start: &[u8],
    negative: bool,
    max_digits: usize,
    min_safe: u64,
    max: u64,
) -> Option<(u64, usize)> {
    debug_assert!(!s.is_empty());

    // parse leading zeros
    let start = s;
    // FIXME: Can't use s.split_first() here yet,
    // see https://github.com/rust-lang/rust/issues/109328
    while let [b'0', s_next @ ..] = s {
        s = s_next;
    }

    // parse digits
    let mut v = 0_u64;
    let digits_start = s;
    if max_digits >= 8 {
        try_parse_digits(&mut s, &mut v);
    } else {
        s = s.parse_digits(|digit| {
            v = v.wrapping_mul(10).wrapping_add(digit as _);
        });
    }
    let n_digits = s.offset_from(digits_start) as usize;

    if n_digits == 0 && s.offset_from(start) == 0 {
        return None;
    }

    // check overflow
    if n_digits > max_digits {
        return None;
    }
    if n_digits == max_digits && v < min_safe {
        return None;
    }
    if max != u64::MAX && v > max + negative as u64 {
        return None;
    }

    let len = s.offset_from(full_start) as _;
    Some((v, len))
}

#[cfg(test)]
#[path = "tests/int.rs"]
mod tests;
