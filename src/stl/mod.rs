//! [STL] (.stl) parser.
//!
//! [STL]: https://en.wikipedia.org/wiki/STL_(file_format)

use std::{io, path::Path, str};

use rustc_hash::FxHashMap;

use crate::{
    error::{self, invalid_data, Location},
    utils::bytes::Lines,
    Mesh, Vec3,
};

/// Parses a mesh from bytes of binary or ascii STL.
#[inline]
pub fn from_slice(bytes: &[u8]) -> io::Result<Mesh> {
    from_slice_internal(bytes)
}

#[inline]
fn from_slice_internal<T>(bytes: &[u8]) -> io::Result<T>
where
    T: FromStl,
{
    match read_binary_header(bytes) {
        Ok(header) => {
            if !header.maybe_ascii {
                // fast path
                read_binary_stl(bytes, header)
            } else if is_ascii_stl(bytes, Some(&header))? {
                read_ascii_stl(bytes, None)
            } else {
                read_binary_stl(bytes, header)
            }
        }
        Err(_) => {
            if is_ascii_stl(bytes, None)? {
                read_ascii_stl(bytes, None)
            } else {
                bail!("failed to determine STL storage representation");
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
const HEADER_SIZE: usize = 80;
const TRIANGLE_COUNT_SIZE: usize = 4;
const TRIANGLE_SIZE: usize = 50;

struct BinaryHeader {
    num_triangles: u32,
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

#[inline]
fn read_binary_triangle(mut buf: &[u8]) -> Triangle {
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
struct AsciiStlParser<'a> {
    lines: Lines<'a>,
    file: Option<&'a Path>,
    column: usize,
}

fn read_ascii_stl<T>(bytes: &[u8], file: Option<&Path>) -> io::Result<T>
where
    T: FromStl,
{
    let mut p = AsciiStlParser::new(bytes, file);
    match p.read_contents() {
        Ok(mesh) => Ok(mesh),
        Err(e) => Err(error::with_location(e, p.location())),
    }
}

impl<'a> AsciiStlParser<'a> {
    fn new(bytes: &'a [u8], file: Option<&'a Path>) -> Self {
        Self {
            lines: Lines::new(bytes),
            file,
            column: 0,
        }
    }

    fn read_line(&mut self) -> io::Result<()> {
        self.column = 0;
        while self.lines.next().is_some() {
            self.skip_spaces();
            if !self.bytes().is_empty() {
                return Ok(());
            }
        }
        bail!("unexpected eof")
    }

    fn bytes(&mut self) -> &[u8] {
        self.lines.current().get(self.column..).unwrap_or_default()
    }

    fn skip_spaces(&mut self) -> bool {
        let prev = self.column;
        while self.bytes().first().map_or(false, u8::is_ascii_whitespace) {
            self.column += 1;
        }
        self.column != prev
    }

    fn expected(&mut self, pat: &str) -> io::Result<()> {
        if !self.bytes().starts_with(pat.as_bytes()) {
            bail!("expected '{}'", pat);
        }
        self.column += pat.len();
        Ok(())
    }

    fn read_contents<T>(&mut self) -> io::Result<T>
    where
        T: FromStl,
    {
        let mut cx = T::start();

        // solid [name]
        if self.lines.next().is_none() {
            bail!("unexpected eof");
        }
        self.expected("solid")?;
        let has_space = self.skip_spaces();
        if !self.bytes().is_empty() {
            if !has_space {
                bail!("unexpected token after `solid`");
            }
            let text = str::from_utf8(self.bytes()).map_err(invalid_data)?;
            let mut text = text.splitn(2, |c: char| c.is_ascii_whitespace());
            if let Some(s) = text.next() {
                T::set_name(&mut cx, s.trim());
                if let Some(s) = text.next() {
                    if !s.trim().is_empty() {
                        bail!("unexpected token after name");
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

    fn read_triangle(&mut self) -> io::Result<Triangle> {
        // facet normal <f32> <f32> <f32>
        self.expected("facet normal ")?;
        self.skip_spaces();
        let normal = self.read_vec3d()?;
        self.skip_spaces();
        if !self.bytes().is_empty() {
            bail!("unexpected token after normal");
        }

        // outer loop
        self.read_line()?;
        self.expected("outer loop")?;
        self.skip_spaces();
        if !self.bytes().is_empty() {
            bail!("unexpected token after `outer loop`");
        }

        // vertex <f32> <f32> <f32>
        self.read_line()?;
        self.expected("vertex ")?;
        self.skip_spaces();
        let vertex1 = self.read_vec3d()?;
        self.skip_spaces();
        if !self.bytes().is_empty() {
            bail!("unexpected token after vertex");
        }

        // vertex <f32> <f32> <f32>
        self.read_line()?;
        self.expected("vertex ")?;
        self.skip_spaces();
        let vertex2 = self.read_vec3d()?;
        self.skip_spaces();
        if !self.bytes().is_empty() {
            bail!("unexpected token after vertex");
        }

        // vertex <f32> <f32> <f32>
        self.read_line()?;
        self.expected("vertex ")?;
        self.skip_spaces();
        let vertex3 = self.read_vec3d()?;
        self.skip_spaces();
        if !self.bytes().is_empty() {
            bail!("unexpected token after vertex");
        }

        // endloop
        self.read_line()?;
        self.expected("endloop")?;
        self.skip_spaces();
        if !self.bytes().is_empty() {
            bail!("unexpected token after `endloop`");
        }

        // endfacet
        self.read_line()?;
        self.expected("endfacet")?;
        self.skip_spaces();
        if !self.bytes().is_empty() {
            bail!("unexpected token after `endfacet`");
        }

        Ok(Triangle {
            normal,
            vertices: [vertex1, vertex2, vertex3],
        })
    }

    fn read_vec3d(&mut self) -> io::Result<Vec3> {
        let x = self.read_float()?;
        if !self.bytes().first().map_or(false, u8::is_ascii_whitespace) {
            bail!("expected whitespace after float");
        }
        self.skip_spaces();

        let y = self.read_float()?;
        if !self.bytes().first().map_or(false, u8::is_ascii_whitespace) {
            bail!("expected whitespace after float");
        }
        self.skip_spaces();

        let z = self.read_float()?;

        Ok([x, y, z])
    }

    fn read_float(&mut self) -> io::Result<f32> {
        let (f, n) = match fast_float::parse_partial::<f32, _>(self.bytes()) {
            Ok(n) => n,
            Err(e) => bail!("{}", e),
        };
        self.column += n;
        Ok(f)
    }

    #[cold]
    fn location(&self) -> Location<'_> {
        Location::new(self.file, self.lines.line_number(), self.column)
    }
}

trait FromStl: Sized {
    type Context;

    fn start() -> Self::Context;

    fn end(cx: Self::Context) -> Self;

    /// Appends a triangle.
    fn push_triangle(cx: &mut Self::Context, triangle: Triangle);

    /// Reserves capacity for at least `num_triangles` more triangles to be inserted.
    ///
    /// - If the format is ASCII STL, `num_triangles` is always 0.
    /// - If the format is binary STL and the input is slice, `num_triangles`
    ///   is the exact number of triangles.
    /// - If the format is binary STL and the input is IO stream,
    ///   `num_triangles` is normally exact, but if the size of the IO stream
    ///   is very large, this may result in a smaller number being passed to
    ///   protect against input with an incorrect `Seek` implementation.
    fn reserve(cx: &mut Self::Context, num_triangles: u32);

    /// Sets the name.
    fn set_name(cx: &mut Self::Context, name: &str);
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Triangle {
    normal: Vec3,
    vertices: [Vec3; 3],
}

#[derive(Default)]
struct ReadContext {
    mesh: Mesh,
    vertices_to_indices: FxHashMap<[u32; 3], usize>,
    vertices_indices: [usize; 3],
}

impl FromStl for Mesh {
    type Context = ReadContext;

    fn start() -> Self::Context {
        ReadContext::default()
    }

    fn end(mut cx: Self::Context) -> Self {
        cx.mesh.vertices.shrink_to_fit();
        cx.mesh.faces.shrink_to_fit();
        cx.mesh.normals.shrink_to_fit();
        cx.mesh
    }

    fn push_triangle(cx: &mut Self::Context, triangle: Triangle) {
        for (i, vertex) in triangle.vertices.iter().enumerate() {
            let bits = [
                vertex[0].to_bits(),
                vertex[1].to_bits(),
                vertex[2].to_bits(),
            ];

            if let Some(&index) = cx.vertices_to_indices.get(&bits) {
                cx.vertices_indices[i] = index;
            } else {
                let index = cx.mesh.vertices.len();
                cx.vertices_to_indices.insert(bits, index);
                cx.vertices_indices[i] = index;
                cx.mesh.vertices.push(*vertex);
            }
        }

        cx.mesh.normals.push(triangle.normal);
        cx.mesh.faces.push([
            cx.vertices_indices[0].try_into().unwrap(),
            cx.vertices_indices[1].try_into().unwrap(),
            cx.vertices_indices[2].try_into().unwrap(),
        ]);
    }

    fn reserve(cx: &mut Self::Context, num_triangles: u32) {
        // Use reserve_exact because binary stl has information on the exact number of triangles.
        cx.mesh.faces.reserve_exact(num_triangles as _);
        cx.mesh.normals.reserve_exact(num_triangles as _);
        // The number of vertices can be up to three times the number of triangles,
        // but is usually less than the number of triangles because of deduplication.
        let cap = (num_triangles as f64 / 1.6) as usize;
        cx.mesh.vertices.reserve(cap);
        cx.vertices_to_indices.reserve(cap);
    }

    fn set_name(cx: &mut Self::Context, name: &str) {
        cx.mesh.name = name.to_owned();
    }
}
