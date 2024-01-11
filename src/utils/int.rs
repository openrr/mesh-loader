// Rust port of fast_float's integer parser.
//
// Source: https://github.com/fastfloat/fast_float/blob/68b9475585be0839fa0bf3d6bfad3e4a6357d90a/include/fast_float/ascii_number.h#L445

#[inline]
pub(crate) fn parse_partial<T: Int>(bytes: &[u8]) -> Option<(T, usize)> {
    T::parse_partial(bytes)
}

pub(crate) trait Int: Sized {
    #[inline]
    fn parse(bytes: &[u8]) -> Option<Self> {
        match Self::parse_partial(bytes) {
            Some((v, n)) if n == bytes.len() => Some(v),
            _ => None,
        }
    }
    fn parse_partial(bytes: &[u8]) -> Option<(Self, usize)>;
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
        impl Int for $ty {
            #[inline]
            fn parse_partial(mut bytes: &[u8]) -> Option<(Self, usize)> {
                const MAX_DIGIT_COUNT: usize = max_digit_count!($ty);
                const LIMIT: u64 = (BASE as u64).pow(MAX_DIGIT_COUNT as u32 - 1);

                let mut has_sign = false;
                if let Some(&b'+') = bytes.get(0) {
                    bytes = &bytes[1..];
                    has_sign = true;
                }
                let mut start: usize = 0;
                while let Some(&b'0') = bytes.get(0) {
                    bytes = &bytes[1..];
                    start += 1;
                }
                let mut n: u64 = 0;
                let mut digit_count: usize = 0;
                if MAX_DIGIT_COUNT >= 8 {
                    loop_parse_if_8digits(&mut bytes, &mut digit_count, &mut n);
                }
                for &b in bytes {
                    // Note: this is correct only when BASE <= 10
                    let digit = b.wrapping_sub(b'0');
                    if digit >= BASE {
                        break;
                    }
                    // we will handle the overflow later
                    n = n.wrapping_mul(BASE as _).wrapping_add(digit as u64);
                    digit_count += 1;
                }
                if digit_count == 0 && start == 0 {
                    return None;
                }
                if digit_count > MAX_DIGIT_COUNT {
                    return None;
                }
                if digit_count == MAX_DIGIT_COUNT && n < LIMIT {
                    return None;
                }
                Some((n as $ty, has_sign as usize + start + digit_count))
            }
        }
    };
}

macro_rules! int {
    ($ty:ident) => {
        impl Int for $ty {
            #[inline]
            fn parse_partial(mut bytes: &[u8]) -> Option<(Self, usize)> {
                const MAX_DIGIT_COUNT: usize = max_digit_count!($ty);
                const LIMIT: u64 = (BASE as u64).pow(MAX_DIGIT_COUNT as u32 - 1);

                let mut has_sign = false;
                let mut negative = false;
                match bytes.get(0) {
                    Some(&b'-') => {
                        bytes = &bytes[1..];
                        has_sign = true;
                        negative = true;
                    }
                    Some(&b'+') => {
                        bytes = &bytes[1..];
                        has_sign = true;
                    }
                    _ => {}
                }
                let mut start: usize = 0;
                while let Some(&b'0') = bytes.get(0) {
                    bytes = &bytes[1..];
                    start += 1;
                }
                let mut n: u64 = 0;
                let mut digit_count: usize = 0;
                if MAX_DIGIT_COUNT >= 8 {
                    loop_parse_if_8digits(&mut bytes, &mut digit_count, &mut n);
                }
                for &b in bytes {
                    // Note: this is correct only when BASE <= 10
                    let digit = b.wrapping_sub(b'0');
                    if digit >= BASE {
                        break;
                    }
                    // we will handle the overflow later
                    n = n.wrapping_mul(BASE as _).wrapping_add(digit as u64);
                    digit_count += 1;
                }
                if digit_count == 0 && start == 0 {
                    return None;
                }
                if digit_count > MAX_DIGIT_COUNT {
                    return None;
                }
                if digit_count == MAX_DIGIT_COUNT && n < LIMIT {
                    return None;
                }
                let v = if negative {
                    (-$ty::MAX).wrapping_sub((n.wrapping_sub($ty::MAX as u64)) as $ty)
                } else {
                    n as $ty
                };
                Some((v, has_sign as usize + start + digit_count))
            }
        }
    };
}

#[inline]
fn is_made_of_8digits_fast(v: u64) -> bool {
    (v.wrapping_add(0x4646_4646_4646_4646) | v.wrapping_sub(0x3030_3030_3030_3030))
        & 0x8080_8080_8080_8080
        == 0
}

#[inline]
fn parse_8digits_unrolled(mut v: u64) -> u32 {
    const MASK: u64 = 0x0000_00FF_0000_00FF;
    const MUL1: u64 = 0x000F_4240_0000_0064;
    const MUL2: u64 = 0x0000_2710_0000_0001;
    v = v.wrapping_sub(0x3030_3030_3030_3030);
    v = (v * 10) + (v >> 8);
    ((v & MASK)
        .wrapping_mul(MUL1)
        .wrapping_add(((v >> 16) & MASK).wrapping_mul(MUL2))
        >> 32) as u32
}

#[inline]
fn loop_parse_if_8digits(bytes: &mut &[u8], digit_count: &mut usize, n: &mut u64) {
    while let Some(b) = bytes.get(..8) {
        let v = u64::from_ne_bytes(b.try_into().unwrap());
        if is_made_of_8digits_fast(v) {
            // we will handle the overflow later
            *n = n
                .wrapping_mul(1_0000_0000)
                .wrapping_add(parse_8digits_unrolled(v) as u64);
            *bytes = &bytes[8..];
            *digit_count += 8;
        } else {
            break;
        }
    }
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

#[cfg(test)]
#[path = "tests/int.rs"]
mod tests;
