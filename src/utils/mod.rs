pub(crate) mod bytes;
#[cfg(any(feature = "collada", feature = "obj", feature = "stl"))]
pub mod float;
#[cfg(feature = "collada")]
pub(crate) mod hex;
#[cfg(any(feature = "collada", feature = "obj"))]
pub mod int;
#[cfg(feature = "collada")]
pub(crate) mod xml;

#[cfg(any(feature = "collada", feature = "obj"))]
pub(crate) mod utf16 {
    use std::{borrow::Cow, io};

    const UTF32BE_BOM: &[u8] = &[0xFF, 0xFE, 00, 00];
    const UTF32LE_BOM: &[u8] = &[00, 00, 0xFE, 0xFF];
    const UTF16BE_BOM: &[u8] = &[0xFE, 0xFF];
    const UTF16LE_BOM: &[u8] = &[0xFF, 0xFE];
    const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

    /// Converts bytes to a string. Converts to UTF-8 if bytes are UTF-16 and have BOM.
    #[cfg(feature = "collada")]
    pub(crate) fn decode_string(bytes: &[u8]) -> io::Result<Cow<'_, str>> {
        if bytes.starts_with(UTF8_BOM) {
            std::str::from_utf8(&bytes[UTF8_BOM.len()..])
                .map(Cow::Borrowed)
                .map_err(crate::error::invalid_data)
        } else if bytes.starts_with(UTF32BE_BOM) || bytes.starts_with(UTF32LE_BOM) {
            bail!("utf-32 is not supported")
        } else if bytes.starts_with(UTF16BE_BOM) {
            from_utf16be(&bytes[UTF16BE_BOM.len()..]).map(Into::into)
        } else if bytes.starts_with(UTF16LE_BOM) {
            from_utf16le(&bytes[UTF16BE_BOM.len()..]).map(Into::into)
        } else {
            // UTF-16/UTF-32 without BOM will get an error here.
            std::str::from_utf8(bytes)
                .map(Cow::Borrowed)
                .map_err(crate::error::invalid_data)
        }
    }

    /// Converts to UTF-8 if bytes are UTF-16 and have BOM.
    /// This does not handle UTF-16 without BOM or other UTF-8 incompatible encodings,
    /// so the resulting bytes must not be trusted as a valid UTF-8.
    #[cfg(feature = "obj")]
    pub(crate) fn decode_bytes(bytes: &[u8]) -> io::Result<Cow<'_, [u8]>> {
        if bytes.starts_with(UTF8_BOM) {
            Ok(Cow::Borrowed(&bytes[UTF8_BOM.len()..]))
        } else if bytes.starts_with(UTF32BE_BOM) || bytes.starts_with(UTF32LE_BOM) {
            bail!("utf-32 is not supported")
        } else if bytes.starts_with(UTF16BE_BOM) {
            from_utf16be(&bytes[UTF16BE_BOM.len()..])
                .map(String::into_bytes)
                .map(Into::into)
        } else if bytes.starts_with(UTF16LE_BOM) {
            from_utf16le(&bytes[UTF16BE_BOM.len()..])
                .map(String::into_bytes)
                .map(Into::into)
        } else {
            Ok(Cow::Borrowed(bytes))
        }
    }

    #[cold]
    #[inline(never)]
    fn from_utf16be(bytes: &[u8]) -> io::Result<String> {
        if bytes.len() % 2 != 0 {
            bail!("invalid utf-16: lone surrogate found");
        }
        char::decode_utf16(
            bytes
                .chunks_exact(2)
                .map(|b| u16::from_be_bytes(b.try_into().unwrap())),
        )
        .collect::<Result<String, _>>()
        .map_err(crate::error::invalid_data)
    }

    #[cold]
    #[inline(never)]
    fn from_utf16le(bytes: &[u8]) -> io::Result<String> {
        if bytes.len() % 2 != 0 {
            bail!("invalid utf-16: lone surrogate found");
        }
        char::decode_utf16(
            bytes
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes(b.try_into().unwrap())),
        )
        .collect::<Result<String, _>>()
        .map_err(crate::error::invalid_data)
    }
}
