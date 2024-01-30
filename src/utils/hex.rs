// Based on https://github.com/KokaKiwi/rust-hex/pull/62, but with several additional optimizations.

use std::{io, mem};

const __: u8 = u8::MAX;

// Lookup table for ascii to hex decoding.
#[rustfmt::skip]
static DECODE_TABLE: [u8; 256] = [
    //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
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
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // A
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // B
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // C
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // D
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // E
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // F
];

#[inline]
pub(crate) fn decode(bytes: &[u8]) -> io::Result<Vec<u8>> {
    if bytes.len() % 2 != 0 {
        bail!("invalid length {}", bytes.len());
    }
    let mut out = vec![0; bytes.len() / 2];
    // Using hex2byte16 instead of hex2byte here increases throughput by 1.5x,
    // but it also increases binary size.
    // let hex2byte = hex2byte16;
    let hex2byte = hex2byte;
    decode_to_slice(bytes, &mut out, hex2byte)?;
    Ok(out)
}

#[inline]
fn decode_to_slice(
    bytes: &[u8],
    out: &mut [u8],
    hex2byte: fn(&[u8], &mut u8) -> io::Result<()>,
) -> io::Result<()> {
    const CHUNK_SIZE: usize = mem::size_of::<usize>();
    // First, process the data in usize units. This improves performance by
    // reducing the number of writes to memory.
    let mut bytes = bytes.chunks_exact(CHUNK_SIZE);
    let mut out = out.chunks_exact_mut(CHUNK_SIZE / 2);
    for (bytes, out) in bytes.by_ref().zip(out.by_ref()) {
        let mut num = [0; CHUNK_SIZE / 2];
        for (bytes, num) in bytes.chunks_exact(2).zip(&mut num) {
            hex2byte(bytes, num)?;
        }
        out.copy_from_slice(&num);
    }
    // Then process the remaining data.
    let bytes = bytes.remainder();
    let out = out.into_remainder();
    for (bytes, out) in bytes.chunks_exact(2).zip(out) {
        hex2byte(bytes, out)?;
    }
    Ok(())
}

#[inline]
fn hex2byte(bytes: &[u8], out: &mut u8) -> io::Result<()> {
    let upper = DECODE_TABLE[bytes[0] as usize];
    let lower = DECODE_TABLE[bytes[1] as usize];
    if upper == u8::MAX {
        bail!("invalid hex character {}", bytes[0] as char);
    }
    if lower == u8::MAX {
        bail!("invalid hex character {}", bytes[1] as char);
    }
    *out = (upper << 4) | lower;
    Ok(())
}

#[cfg(test)]
static ENCODE_LOWER_TABLE: &[u8; 16] = b"0123456789abcdef";
#[cfg(test)]
static ENCODE_UPPER_TABLE: &[u8; 16] = b"0123456789ABCDEF";
#[cfg(test)]
#[inline]
const fn byte2hex(byte: u8, table: &[u8; 16]) -> [u8; 2] {
    let upper = table[((byte & 0xf0) >> 4) as usize];
    let lower = table[(byte & 0x0f) as usize];
    [upper, lower]
}

#[cfg(test)]
#[inline]
fn hex2byte16(bytes: &[u8], out: &mut u8) -> io::Result<()> {
    static DECODE_TABLE: [u8; 65536] = {
        let mut table = [__; 65536];
        let mut i = 0;
        while i != u8::MAX {
            let lower = u16::from_ne_bytes(byte2hex(i, ENCODE_LOWER_TABLE));
            let upper = u16::from_ne_bytes(byte2hex(i, ENCODE_UPPER_TABLE));
            table[lower as usize] = i;
            table[upper as usize] = i;
            i += 1;
        }
        table
    };
    let n = u16::from_ne_bytes(bytes.try_into().unwrap());
    let num = DECODE_TABLE[n as usize];
    if num == u8::MAX {
        bail!(
            "invalid hex character {}{}",
            bytes[0] as char,
            bytes[1] as char
        );
    }
    *out = num;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_naive(bytes: &[u8], table: &[u8; 16]) -> Vec<u8> {
        let mut out = vec![0; bytes.len() * 2];
        for (&byte, out) in bytes.iter().zip(out.chunks_exact_mut(2)) {
            out.copy_from_slice(&byte2hex(byte, table));
        }
        out
    }
    fn decode_naive(
        bytes: &[u8],
        hex2byte: fn(&[u8], &mut u8) -> io::Result<()>,
    ) -> io::Result<Vec<u8>> {
        if bytes.len() % 2 != 0 {
            bail!("invalid length {}", bytes.len());
        }
        let mut out = vec![0; bytes.len() / 2];
        for (bytes, out) in bytes.chunks_exact(2).zip(&mut out) {
            hex2byte(bytes, out)?;
        }
        Ok(out)
    }
    #[inline]
    fn decode16(bytes: &[u8]) -> io::Result<Vec<u8>> {
        if bytes.len() % 2 != 0 {
            bail!("invalid length {}", bytes.len());
        }
        let mut out = vec![0; bytes.len() / 2];
        decode_to_slice(bytes, &mut out, hex2byte16)?;
        Ok(out)
    }

    ::quickcheck::quickcheck! {
        fn decode_valid(x: String) -> bool {
            if x.is_empty() {
                return true;
            }
            let x = x.as_bytes();
            let hex_lower = encode_naive(x, ENCODE_LOWER_TABLE);
            assert_eq!(decode(&hex_lower).unwrap(), x);
            assert_eq!(decode16(&hex_lower).unwrap(), x);
            assert_eq!(decode_naive(&hex_lower, hex2byte).unwrap(), x);
            assert_eq!(decode_naive(&hex_lower, hex2byte16).unwrap(), x);
            let hex_upper = encode_naive(x, ENCODE_UPPER_TABLE);
            assert_eq!(decode(&hex_upper).unwrap(), x);
            assert_eq!(decode16(&hex_lower).unwrap(), x);
            assert_eq!(decode_naive(&hex_upper, hex2byte).unwrap(), x);
            assert_eq!(decode_naive(&hex_upper, hex2byte16).unwrap(), x);
            true
        }
        fn decode_invalid(x: String) -> bool {
            if x.is_empty() {
                return true;
            }
            let mut x = x.as_bytes();
            if x.len() < 2 {
                return true;
            }
            if x.len() % 2 != 0 {
                x = &x[..x.len() - 2];
            }
            let res = decode(x).ok();
            assert_eq!(res, decode16(x).ok());
            assert_eq!(res, decode_naive(x, hex2byte).ok());
            assert_eq!(res, decode_naive(x, hex2byte16).ok());
            true
        }
    }
}
