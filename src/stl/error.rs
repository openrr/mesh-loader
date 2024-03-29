use std::{fmt, io, path::Path};

#[cfg_attr(test, derive(Debug))]
pub(super) enum ErrorKind {
    // ASCII STL error
    ExpectedSpace(&'static str, usize),
    ExpectedNewline(&'static str, usize),
    Expected(&'static str, usize),
    Float(usize),
    NotAscii(&'static str, usize),
    // binary STL error
    TooSmall,
    InvalidSize,
    TooManyTriangles,
}

impl ErrorKind {
    #[cold]
    #[inline(never)]
    pub(super) fn into_io_error(self, start: &[u8], path: Option<&Path>) -> io::Error {
        let remaining = match self {
            // ASCII STL error
            Self::Expected(.., n)
            | Self::ExpectedNewline(.., n)
            | Self::ExpectedSpace(.., n)
            | Self::Float(n)
            | Self::NotAscii(.., n) => n,
            // binary STL error (always points file:1:1, as error occurs only during reading the header)
            _ => start.len(),
        };
        crate::error::with_location(
            &crate::error::invalid_data(self.to_string()),
            &crate::error::Location::find(remaining, start, path),
        )
    }
}

impl fmt::Display for ErrorKind {
    #[cold]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            // ASCII STL error
            Self::ExpectedSpace(msg, ..) => {
                if msg == "normal" || msg == "vertex" {
                    f.write_str("expected space before floats")
                } else {
                    write!(f, "expected space after {msg}")
                }
            }
            Self::ExpectedNewline(msg, ..) => {
                if msg == "solid" {
                    f.write_str("expected newline after solid name")
                } else if msg == "normal" || msg == "vertex" {
                    f.write_str("expected newline after floats")
                } else {
                    write!(f, "expected newline after {msg}")
                }
            }
            Self::Expected(msg, remaining) => {
                if msg == "solid" && remaining != 0 {
                    f.write_str("expected solid or eof")
                } else if msg == "endsolid" {
                    f.write_str("expected facet normal or endsolid")
                } else {
                    write!(f, "expected {msg}")
                }
            }
            Self::Float(..) => f.write_str("error while parsing a float"),
            Self::NotAscii(..) => f.write_str("invalid ASCII"),
            // binary STL error
            Self::TooSmall => f.write_str(
                "failed to determine STL storage representation: \
                 not valid ASCII STL and size is too small as binary STL",
            ),
            Self::InvalidSize => f.write_str(
                "failed to determine STL storage representation: \
                 not valid ASCII STL and size is invalid as binary STL",
            ),
            Self::TooManyTriangles => f.write_str("too many triangles"),
        }
    }
}
