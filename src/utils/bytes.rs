// This is the same as s.starts_with(needle), but faster if the length of the
// needle is known at compile time.
#[inline(always)] // Ensure the code getting the length of the needle is inlined.
pub(crate) fn starts_with(mut s: &[u8], mut needle: &'static [u8]) -> bool {
    if s.len() < needle.len() {
        return false;
    }
    if needle.len() < 4 {
        return s.starts_with(needle);
    }
    if needle.len() < 8 {
        // u32 (4 bytes) + 0-3 bytes
        return u32::from_ne_bytes(needle[..4].try_into().unwrap())
            == u32::from_ne_bytes(s[..4].try_into().unwrap())
            && s[4..].starts_with(&needle[4..]);
    }
    if needle.len() < 12 {
        // u64 (8 bytes) + 0-3 bytes
        return u64::from_ne_bytes(needle[..8].try_into().unwrap())
            == u64::from_ne_bytes(s[..8].try_into().unwrap())
            && s[8..].starts_with(&needle[8..]);
    }
    if needle.len() < 16 {
        // u64 (8 bytes) + u32 (4 bytes) + 0-3 bytes
        return u64::from_ne_bytes(needle[..8].try_into().unwrap())
            == u64::from_ne_bytes(s[..8].try_into().unwrap())
            && u32::from_ne_bytes(needle[8..12].try_into().unwrap())
                == u32::from_ne_bytes(s[8..12].try_into().unwrap())
            && s[12..].starts_with(&needle[12..]);
    }
    // u64 (8 bytes) + u64 (8 bytes) + N bytes
    while needle.len() >= 8 {
        if u64::from_ne_bytes(needle[..8].try_into().unwrap())
            != u64::from_ne_bytes(s[..8].try_into().unwrap())
        {
            return false;
        }
        needle = &needle[..8];
        s = &s[..8];
    }
    s.starts_with(needle)
}

#[inline]
pub(crate) const fn memchr_naive(needle: u8, mut s: &[u8]) -> Option<usize> {
    let start = s;
    while let Some((&b, s_next)) = s.split_first() {
        if b == needle {
            return Some(start.len() - s.len());
        }
        s = s_next;
    }
    None
}

#[inline]
pub(crate) const fn memchr_naive_table(
    needle_mask: u8,
    table: &[u8; 256],
    mut s: &[u8],
) -> Option<usize> {
    let start = s;
    while let Some((&b, s_next)) = s.split_first() {
        if table[b as usize] & needle_mask != 0 {
            return Some(start.len() - s.len());
        }
        s = s_next;
    }
    None
}

#[inline]
pub(crate) const fn memrchr_naive(needle: u8, mut s: &[u8]) -> Option<usize> {
    let start = s;
    while let Some((&b, s_next)) = s.split_last() {
        if b == needle {
            return Some(start.len() - s.len());
        }
        s = s_next;
    }
    None
}

#[inline]
pub(crate) const fn bytecount_naive(needle: u8, mut s: &[u8]) -> usize {
    let mut n = 0;
    while let Some((&b, s_next)) = s.split_first() {
        n += (b == needle) as usize;
        s = s_next;
    }
    n
}
