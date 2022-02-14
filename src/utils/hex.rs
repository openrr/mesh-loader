// Based on https://github.com/KokaKiwi/rust-hex/pull/62.

use anyhow::{bail, Result};

const __: u8 = u8::MAX;

// Lookup table for ascii to hex decoding.
#[rustfmt::skip]
static DECODE_TABLE: [u8; 256] = [
    //   1   2   3   4   5   6   7   8   9   a   b   c   d   e   f
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 0
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 1
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 2
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9, __, __, __, __, __, __, // 3
    __, 10, 11, 12, 13, 14, 15, __, __, __, __, __, __, __, __, __, // 4
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 5
    __, 10, 11, 12, 13, 14, 15, __, __, __, __, __, __, __, __, __, // 6
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 7
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 8
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 9
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // a
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // b
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // c
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // d
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // e
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // f
];

#[inline]
pub(crate) fn decode(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() % 2 != 0 {
        bail!("invalid length {}", bytes.len());
    }

    let mut out = vec![0; bytes.len() / 2];
    for (bytes, out) in bytes.chunks_exact(2).zip(&mut out) {
        let upper = DECODE_TABLE[bytes[0] as usize];
        let lower = DECODE_TABLE[bytes[1] as usize];
        if upper == u8::MAX {
            bail!("invalid hex character {}", bytes[0] as char);
        }
        if lower == u8::MAX {
            bail!("invalid hex character {}", bytes[1] as char);
        }
        *out = (upper << 4) | lower;
    }
    Ok(out)
}
