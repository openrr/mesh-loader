use std::io;
#[cfg(feature = "stl")]
use std::{fmt, path::Path};

macro_rules! format_err {
    ($msg:expr $(,)?) => {
        crate::error::invalid_data($msg)
    };
    ($($tt:tt)*) => {
        crate::error::invalid_data(format!($($tt)*))
    };
}

macro_rules! bail {
    ($($tt:tt)*) => {
        return Err(format_err!($($tt)*))
    };
}

#[cold]
pub(crate) fn invalid_data(e: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    let e = e.into();
    let kind = e
        .downcast_ref::<io::Error>()
        .map_or(io::ErrorKind::InvalidData, io::Error::kind);
    io::Error::new(kind, e)
}

#[cfg(feature = "stl")]
#[cold]
pub(crate) fn with_location(e: io::Error, location: Location<'_>) -> io::Error {
    io::Error::new(e.kind(), format!("{} ({})", e, location))
}

#[cfg(feature = "stl")]
pub(crate) struct Location<'a> {
    file: Option<&'a Path>,
    line: usize,
    column: usize,
}

#[cfg(feature = "stl")]
impl<'a> Location<'a> {
    pub(crate) fn new(file: Option<&'a Path>, line: usize, column: usize) -> Self {
        Self {
            file: file.filter(|&p| p != Path::new("")),
            line,
            column,
        }
    }
}

#[cfg(feature = "stl")]
impl fmt::Display for Location<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(file) = self.file {
            write!(
                f,
                "{}:{}:{}",
                file.to_string_lossy(),
                self.line,
                self.column
            )
        } else {
            write!(f, "{}:{}", self.line, self.column)
        }
    }
}
