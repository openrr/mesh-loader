//! Common utilities, for internal use only.

/// Helper methods to process immutable bytes.
pub(crate) trait ByteSlice {
    /// Read 8 bytes as a 64-bit integer in little-endian order.
    fn read_u64le(&self) -> u64;

    /// Write a 64-bit integer as 8 bytes in little-endian order.
    fn write_u64le(&mut self, value: u64);

    /// Calculate the offset of a slice from another.
    fn offset_from(&self, other: &Self) -> isize;

    /// Iteratively parse and consume digits from bytes.
    /// Returns the same bytes with consumed digits being
    /// elided.
    #[allow(clippy::impl_trait_in_params)] // clippy bug: should not warn method of private trait
    fn parse_digits(&self, func: impl FnMut(u8)) -> &Self;
}

impl ByteSlice for [u8] {
    #[inline(always)] // inlining this is crucial to remove bound checks
    fn read_u64le(&self) -> u64 {
        u64::from_le_bytes(self[..8].try_into().unwrap())
    }

    #[inline(always)] // inlining this is crucial to remove bound checks
    fn write_u64le(&mut self, value: u64) {
        self[..8].copy_from_slice(&value.to_le_bytes());
    }

    #[inline]
    fn offset_from(&self, other: &Self) -> isize {
        other.len() as isize - self.len() as isize
    }

    #[inline]
    fn parse_digits(&self, mut func: impl FnMut(u8)) -> &Self {
        let mut s = self;

        // FIXME: Can't use s.split_first() here yet,
        // see https://github.com/rust-lang/rust/issues/109328
        // (fixed in LLVM 17)
        while let [c, s_next @ ..] = s {
            let c = c.wrapping_sub(b'0');
            if c < 10 {
                func(c);
                s = s_next;
            } else {
                break;
            }
        }

        s
    }
}

/// Determine if 8 bytes are all decimal digits.
/// This does not care about the order in which the bytes were loaded.
pub(crate) const fn is_8digits(v: u64) -> bool {
    let a = v.wrapping_add(0x4646_4646_4646_4646);
    let b = v.wrapping_sub(0x3030_3030_3030_3030);
    (a | b) & 0x8080_8080_8080_8080 == 0
}

/// A custom 64-bit floating point type, representing `f * 2^e`.
/// e is biased, so it be directly shifted into the exponent bits.
#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub(crate) struct BiasedFp {
    /// The significant digits.
    pub(crate) f: u64,
    /// The biased, binary exponent.
    pub(crate) e: i32,
}

impl BiasedFp {
    #[inline]
    pub(crate) const fn zero_pow2(e: i32) -> Self {
        Self { f: 0, e }
    }
}
