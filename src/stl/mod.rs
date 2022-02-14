mod types;

use std::{fmt, io, mem, str};

pub use self::types::{FromStl, IndexMesh, IndexTriangle, Mesh, Triangle, Vector3D};
use crate::utils::Never;

/// Parses an instance of type `T` from bytes of binary or ascii STL.
#[inline]
pub fn from_slice<T>(bytes: &[u8]) -> io::Result<T>
where
    T: FromStl,
{
    from_slice_internal(bytes)
}

#[inline]
fn from_slice_internal<T>(bytes: &[u8]) -> io::Result<T>
where
    T: FromStl,
{
    match read_binary_header(bytes) {
        Ok(header) => {
            if !header.maybe_ascii || header.correct_triangle_count {
                read_binary_stl(bytes, header)
            } else if is_ascii_stl(bytes, Some(&header))? {
                AsciiStlParser::new(bytes)?.read_contents()
            } else {
                read_binary_stl(bytes, header)
            }
        }
        Err(_) => {
            if is_ascii_stl(bytes, None)? {
                AsciiStlParser::new(bytes)?.read_contents()
            } else {
                Err(invalid_data(
                    "failed to determine STL storage representation",
                ))
            }
        }
    }
}

// An ascii STL buffer will begin with "solid NAME", where NAME is optional.
// Note: The "solid NAME" check is necessary, but not sufficient, to determine
// if the buffer is ASCII; a binary header could also begin with "solid NAME".
fn is_ascii_stl(bytes: &[u8], header: Option<&BinaryHeader>) -> io::Result<bool> {
    let mut is_ascii = if let Some(header) = header {
        header.maybe_ascii
    } else {
        bytes.get(..5).ok_or_else(|| invalid_data("too small"))? == b"solid"
    };
    if is_ascii {
        // A lot of importers are write solid even if the file is binary.
        // So we have to check for ASCII-characters.
        if !bytes[5..].iter().all(u8::is_ascii) {
            is_ascii = false;
        }
    }
    Ok(is_ascii)
}

/*
https://en.wikipedia.org/wiki/STL_(file_format)#Binary_STL

UINT8[80]    – Header                 -     80 bytes
UINT32       – Number of triangles    -      4 bytes

foreach triangle                      - 50 bytes:
    REAL32[3] – Normal vector             - 12 bytes
    REAL32[3] – Vertex 1                  - 12 bytes
    REAL32[3] – Vertex 2                  - 12 bytes
    REAL32[3] – Vertex 3                  - 12 bytes
    UINT16    – Attribute byte count      -  2 bytes
end
*/
pub(crate) const HEADER_SIZE: usize = 80;
pub(crate) const TRIANGLE_COUNT_SIZE: usize = 4;
pub(crate) const TRIANGLE_SIZE: usize = 50;

struct BinaryHeader {
    num_triangles: u32,
    correct_triangle_count: bool,
    maybe_ascii: bool,
}

fn read_binary_header(bytes: &[u8]) -> io::Result<BinaryHeader> {
    let header = bytes
        .get(..HEADER_SIZE)
        .ok_or_else(|| invalid_data("too small"))?;

    let num_triangles = bytes
        .get(HEADER_SIZE..HEADER_SIZE + TRIANGLE_COUNT_SIZE)
        .ok_or_else(|| invalid_data("too small"))?
        .try_into()
        .unwrap();
    let mut num_triangles = u32::from_le_bytes(num_triangles);

    // Many STL files contain bogus count.
    // So verify num_triangles with the length of the input.
    let mut size = bytes.len() as u64;
    size -= (HEADER_SIZE + TRIANGLE_COUNT_SIZE) as u64;
    size /= TRIANGLE_SIZE as u64;
    let size: u32 = size
        .try_into()
        .map_err(|_| invalid_data("number of triangles is greater than u32::MAX"))?;

    let correct_triangle_count = num_triangles == size;
    if !correct_triangle_count {
        num_triangles = size;
    }

    // An ASCII STL will begin with "solid NAME", where NAME is optional.
    // Note: The "solid NAME" check is necessary, but not sufficient, to determine
    // if the input is ASCII; a binary header could also begin with "solid NAME".
    let maybe_ascii = header.starts_with(b"solid");

    Ok(BinaryHeader {
        num_triangles,
        correct_triangle_count,
        maybe_ascii,
    })
}

#[inline]
fn read_binary_stl<T>(mut bytes: &[u8], header: BinaryHeader) -> io::Result<T>
where
    T: FromStl,
{
    bytes = &bytes[HEADER_SIZE + TRIANGLE_COUNT_SIZE..];

    let mut cx = T::start();

    T::reserve(&mut cx, header.num_triangles);
    read_binary_triangles_from_slice::<T>(&mut cx, bytes);

    Ok(T::end(cx))
}

#[inline]
fn read_binary_triangles_from_slice<T>(cx: &mut T::Context, bytes: &[u8])
where
    T: FromStl,
{
    for chunk in bytes.chunks_exact(TRIANGLE_SIZE) {
        let triangle = read_binary_triangle(chunk);
        T::push_triangle(cx, triangle);
    }
}

#[doc(hidden)] // for tests
#[inline]
pub fn read_binary_triangle(mut buf: &[u8]) -> Triangle {
    #[inline]
    fn f32le(buf: &mut &[u8]) -> f32 {
        let f = f32::from_le_bytes(buf[..4].try_into().unwrap());
        *buf = &buf[4..];
        f
    }

    let normal = [f32le(&mut buf), f32le(&mut buf), f32le(&mut buf)];
    let vertex1 = [f32le(&mut buf), f32le(&mut buf), f32le(&mut buf)];
    let vertex2 = [f32le(&mut buf), f32le(&mut buf), f32le(&mut buf)];
    let vertex3 = [f32le(&mut buf), f32le(&mut buf), f32le(&mut buf)];
    Triangle {
        normal,
        vertices: [vertex1, vertex2, vertex3],
    }
}

/*
https://en.wikipedia.org/wiki/STL_(file_format)#ASCII_STL

solid name

facet normal ni nj nk
  outer loop
    vertex v1x v1y v1z
    vertex v2x v2y v2z
    vertex v3x v3y v3z
  endloop
endfacet

endsolid name
*/
#[doc(hidden)] // for tests
#[allow(missing_debug_implementations)]
pub struct AsciiStlParser<'a> {
    lines: Lines<'a>,
    column: usize,
}

struct Lines<'a> {
    bytes: &'a [u8],
    iter: memchr::Memchr<'a>,
    next_line_start: usize,
    line_start: usize,
    line_end: usize,
}

impl<'a> Lines<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            iter: memchr::memchr_iter(b'\n', bytes),
            next_line_start: 0,
            line_start: 0,
            line_end: 0,
        }
    }

    fn current_line(&self) -> &[u8] {
        self.bytes
            .get(self.line_start..self.line_end)
            .unwrap_or_default()
    }
}

impl Iterator for Lines<'_> {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        let line_end = match self.iter.next() {
            Some(line_end) => line_end,
            None => {
                self.bytes.get(self.next_line_start)?;
                self.bytes.len()
            }
        };
        self.line_end = line_end;
        self.line_start = mem::replace(&mut self.next_line_start, line_end + 1);
        Some(())
    }
}

impl<'a> AsciiStlParser<'a> {
    #[doc(hidden)] // for tests
    pub fn new(bytes: &'a [u8]) -> io::Result<Self> {
        Ok(Self {
            lines: Lines::new(bytes),
            column: 0,
        })
    }

    #[doc(hidden)] // for tests
    pub fn read_line(&mut self) -> io::Result<()> {
        self.column = 0;
        while self.lines.next().is_some() {
            self.skip_spaces();
            if !self.bytes().is_empty() {
                return Ok(());
            }
        }
        self.error("unexpected eof")?;
    }

    fn bytes(&mut self) -> &[u8] {
        self.lines
            .current_line()
            .get(self.column..)
            .unwrap_or_default()
    }

    fn skip_spaces(&mut self) -> bool {
        let prev = self.column;
        while self.bytes().get(0).map_or(false, u8::is_ascii_whitespace) {
            self.column += 1;
        }
        self.column != prev
    }

    fn expected(&mut self, pat: &str) -> io::Result<()> {
        if !self.bytes().starts_with(pat.as_bytes()) {
            self.error(format!("expected `{}`", pat))?;
        }
        self.column += pat.len();
        Ok(())
    }

    fn read_contents<T>(mut self) -> io::Result<T>
    where
        T: FromStl,
    {
        let mut cx = T::start();

        // solid [name]
        if self.lines.next().is_none() {
            self.error("unexpected eof")?;
        }
        self.expected("solid")?;
        let has_space = self.skip_spaces();
        if !self.bytes().is_empty() {
            if !has_space {
                self.error("unexpected token after `solid`")?;
            }
            let text = str::from_utf8(self.bytes()).map_err(invalid_data)?;
            let mut text = text.splitn(2, |c: char| c.is_ascii_whitespace());
            if let Some(s) = text.next() {
                T::set_name(&mut cx, s.trim());
                if let Some(s) = text.next() {
                    if !s.trim().is_empty() {
                        self.error("unexpected token after name")?;
                    }
                }
            }
        }

        loop {
            self.read_line()?;
            // endsolid [name]
            if self.bytes().starts_with(b"endsolid") {
                // TODO: check name
                break;
            }

            let triangle = self.read_triangle()?;
            T::push_triangle(&mut cx, triangle);
        }

        Ok(T::end(cx))
    }

    #[doc(hidden)] // for tests
    pub fn read_triangle(&mut self) -> io::Result<Triangle> {
        // facet normal <f32> <f32> <f32>
        self.expected("facet normal ")?;
        self.skip_spaces();
        let normal = self.read_vec3d()?;
        self.skip_spaces();
        if !self.bytes().is_empty() {
            self.error("unexpected token after normal")?;
        }

        // outer loop
        self.read_line()?;
        self.expected("outer loop")?;
        self.skip_spaces();
        if !self.bytes().is_empty() {
            self.error("unexpected token after `outer loop`")?;
        }

        // vertex <f32> <f32> <f32>
        self.read_line()?;
        self.expected("vertex ")?;
        self.skip_spaces();
        let vertex1 = self.read_vec3d()?;
        self.skip_spaces();
        if !self.bytes().is_empty() {
            self.error("unexpected token after vertex")?;
        }

        // vertex <f32> <f32> <f32>
        self.read_line()?;
        self.expected("vertex ")?;
        self.skip_spaces();
        let vertex2 = self.read_vec3d()?;
        self.skip_spaces();
        if !self.bytes().is_empty() {
            self.error("unexpected token after vertex")?;
        }

        // vertex <f32> <f32> <f32>
        self.read_line()?;
        self.expected("vertex ")?;
        self.skip_spaces();
        let vertex3 = self.read_vec3d()?;
        self.skip_spaces();
        if !self.bytes().is_empty() {
            self.error("unexpected token after vertex")?;
        }

        // endloop
        self.read_line()?;
        self.expected("endloop")?;
        self.skip_spaces();
        if !self.bytes().is_empty() {
            self.error("unexpected token after `endloop`")?;
        }

        // endfacet
        self.read_line()?;
        self.expected("endfacet")?;
        self.skip_spaces();
        if !self.bytes().is_empty() {
            self.error("unexpected token after `endfacet`")?;
        }

        Ok(Triangle {
            normal,
            vertices: [vertex1, vertex2, vertex3],
        })
    }

    fn read_vec3d(&mut self) -> io::Result<Vector3D> {
        let x = self.read_float()?;
        if !self.bytes().get(0).map_or(false, u8::is_ascii_whitespace) {
            self.error("expected whitespace after float")?;
        }
        self.skip_spaces();

        let y = self.read_float()?;
        if !self.bytes().get(0).map_or(false, u8::is_ascii_whitespace) {
            self.error("expected whitespace after float")?;
        }
        self.skip_spaces();

        let z = self.read_float()?;

        Ok([x, y, z])
    }

    fn read_float(&mut self) -> io::Result<f32> {
        let (f, n) = match fast_float::parse_partial::<f32, _>(self.bytes()) {
            Ok(n) => n,
            Err(e) => self.error(e)?,
        };
        self.column += n;
        Ok(f)
    }

    #[cold]
    fn error(&self, e: impl fmt::Display) -> io::Result<Never> {
        // TODO: get line number based on position
        let msg = format!("{} (line: {}, column: {})", e, 0, self.column);
        Err(io::Error::new(io::ErrorKind::InvalidData, msg))
    }
}

#[cold]
fn invalid_data(e: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    let e = e.into();
    let kind = e
        .downcast_ref::<io::Error>()
        .map_or(io::ErrorKind::InvalidData, io::Error::kind);
    io::Error::new(kind, e)
}
