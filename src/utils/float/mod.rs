// Rust port of fast_float's float parser.
//
// Adapted from core::num::dec2flt with partial parsing support added, and very tricky and unsafe x87 FPU-related hack removed.
//
// Source: https://github.com/rust-lang/rust/tree/1.80.0/library/core/src/num/dec2flt
//
// Copyright & License of the original code:
// - https://github.com/rust-lang/rust/blob/1.80.0/COPYRIGHT
// - https://github.com/rust-lang/rust/blob/1.80.0/LICENSE-APACHE
// - https://github.com/rust-lang/rust/blob/1.80.0/LICENSE-MIT
//
// # References
//
// - Daniel Lemire, Number Parsing at a Gigabyte per Second, Software: Practice and Experience 51 (8), 2021.
//   https://arxiv.org/abs/2101.11408
// - https://github.com/fastfloat/fast_float

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::items_after_statements,
    clippy::module_inception,
    clippy::redundant_else
)]

pub(crate) mod common;
mod decimal;
mod float;
mod lemire;
mod number;
pub(crate) mod parse;
mod slow;
mod table;

use self::{
    common::{BiasedFp, ByteSlice},
    float::RawFloat,
    lemire::compute_float,
    parse::{parse_inf_nan, parse_partial_number},
    slow::parse_long_mantissa,
};

#[inline]
pub fn parse<T: Float>(bytes: &[u8]) -> Option<T> {
    T::parse(bytes)
}

#[inline]
pub fn parse_partial<T: Float>(bytes: &[u8]) -> Option<(T, usize)> {
    T::parse_partial(bytes)
}

pub trait Float: float::RawFloat {
    #[inline]
    fn parse(bytes: &[u8]) -> Option<Self> {
        match Self::parse_partial(bytes) {
            Some((v, n)) if n == bytes.len() => Some(v),
            _ => None,
        }
    }

    #[inline]
    fn parse_partial(bytes: &[u8]) -> Option<(Self, usize)> {
        dec2flt(bytes)
    }
}

impl Float for f32 {}
impl Float for f64 {}

/// Converts a `BiasedFp` to the closest machine float type.
fn biased_fp_to_float<T: RawFloat>(x: BiasedFp) -> T {
    let mut word = x.f;
    word |= (x.e as u64) << T::MANTISSA_EXPLICIT_BITS;
    T::from_u64_bits(word)
}

/// Converts a decimal string into a floating point number.
#[inline]
pub(crate) fn dec2flt<F: RawFloat>(mut s: &[u8]) -> Option<(F, usize)> {
    let start = s;
    let c = if let Some(&c) = s.first() {
        c
    } else {
        return None;
    };
    let negative = c == b'-';
    if negative || c == b'+' {
        s = &s[1..];
        if s.is_empty() {
            return None;
        }
    }

    let (mut num, len) = match parse_partial_number(s, start) {
        Some(r) => r,
        None => match parse_inf_nan(s, negative) {
            Some((value, len)) => return Some((value, len + s.offset_from(start) as usize)),
            None => return None,
        },
    };
    num.negative = negative;
    if let Some(value) = num.try_fast_path::<F>() {
        return Some((value, len));
    }

    // If significant digits were truncated, then we can have rounding error
    // only if `mantissa + 1` produces a different result. We also avoid
    // redundantly using the Eisel-Lemire algorithm if it was unable to
    // correctly round on the first pass.
    let mut fp = compute_float::<F>(num.exponent, num.mantissa);
    if num.many_digits && fp.e >= 0 && fp != compute_float::<F>(num.exponent, num.mantissa + 1) {
        fp.e = -1;
    }
    // Unable to correctly round the float using the Eisel-Lemire algorithm.
    // Fallback to a slower, but always correct algorithm.
    if fp.e < 0 {
        fp = parse_long_mantissa::<F>(s);
    }

    let mut float = biased_fp_to_float::<F>(fp);
    if num.negative {
        float = -float;
    }
    Some((float, len))
}

#[cfg(test)]
#[path = "../tests/float.rs"]
mod tests;
