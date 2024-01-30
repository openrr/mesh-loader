use std::{fmt, io, path::Path, str};

#[cfg_attr(test, derive(Debug))]
pub(super) enum ErrorKind {
    ExpectedSpace(&'static str, usize),
    ExpectedNewline(&'static str, usize),
    Expected(&'static str, usize),
    Float(usize),
    Int(usize),
    InvalidW(usize),
    InvalidFaceIndex(usize),
    Oob(usize, usize),
    Io(io::Error),
}

impl ErrorKind {
    #[cold]
    #[inline(never)]
    pub(super) fn into_io_error(self, start: &[u8], path: Option<&Path>) -> io::Error {
        let remaining = match self {
            Self::Expected(.., n)
            | Self::ExpectedNewline(.., n)
            | Self::ExpectedSpace(.., n)
            | Self::Float(n)
            | Self::Int(n)
            | Self::InvalidW(n)
            | Self::InvalidFaceIndex(n)
            | Self::Oob(.., n) => n,
            Self::Io(e) => return e,
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
            Self::ExpectedSpace(msg, ..) => write!(f, "expected space after {msg}"),
            Self::ExpectedNewline(msg, ..) => write!(f, "expected newline after {msg}"),
            Self::Expected(msg, ..) => write!(f, "expected {msg}"),
            Self::InvalidW(..) => write!(f, "w in homogeneous vector must not zero"),
            Self::InvalidFaceIndex(..) => write!(f, "invalid face index"),
            Self::Float(..) => write!(f, "error while parsing a float"),
            Self::Int(..) => write!(f, "error while parsing an integer"),
            Self::Oob(i, ..) => write!(f, "face index out of bounds ({i})"),
            Self::Io(ref e) => write!(f, "{e}"),
        }
    }
}
