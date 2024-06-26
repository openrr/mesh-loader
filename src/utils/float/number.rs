//! Representation of a float as the significant digits and exponent.

use super::float::RawFloat;

#[rustfmt::skip]
static INT_POW10: [u64; 16] = [
    1,
    10,
    100,
    1000,
    10000,
    100000,
    1000000,
    10000000,
    100000000,
    1000000000,
    10000000000,
    100000000000,
    1000000000000,
    10000000000000,
    100000000000000,
    1000000000000000,
];

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Number {
    pub(crate) exponent: i64,
    pub(crate) mantissa: u64,
    pub(crate) negative: bool,
    pub(crate) many_digits: bool,
}

impl Number {
    /// Detect if the float can be accurately reconstructed from native floats.
    #[inline]
    fn is_fast_path<F: RawFloat>(&self) -> bool {
        F::MIN_EXPONENT_FAST_PATH <= self.exponent
            && self.exponent <= F::MAX_EXPONENT_DISGUISED_FAST_PATH
            && self.mantissa <= F::MAX_MANTISSA_FAST_PATH
            && !self.many_digits
    }

    /// The fast path algorithm using machine-sized integers and floats.
    ///
    /// This is extracted into a separate function so that it can be attempted before constructing
    /// a Decimal. This only works if both the mantissa and the exponent
    /// can be exactly represented as a machine float, since IEE-754 guarantees
    /// no rounding will occur.
    ///
    /// There is an exception: disguised fast-path cases, where we can shift
    /// powers-of-10 from the exponent to the significant digits.
    #[inline]
    pub(crate) fn try_fast_path<F: RawFloat>(&self) -> Option<F> {
        if self.is_fast_path::<F>() {
            let mut value = if self.exponent <= F::MAX_EXPONENT_FAST_PATH {
                // normal fast path
                let value = F::from_u64(self.mantissa);
                if self.exponent < 0 {
                    value / F::pow10_fast_path((-self.exponent) as usize)
                } else {
                    value * F::pow10_fast_path(self.exponent as usize)
                }
            } else {
                // disguised fast path
                let shift = self.exponent - F::MAX_EXPONENT_FAST_PATH;
                let mantissa = self.mantissa.checked_mul(INT_POW10[shift as usize])?;
                if mantissa > F::MAX_MANTISSA_FAST_PATH {
                    return None;
                }
                F::from_u64(mantissa) * F::pow10_fast_path(F::MAX_EXPONENT_FAST_PATH as usize)
            };
            if self.negative {
                value = -value;
            }
            Some(value)
        } else {
            None
        }
    }
}
