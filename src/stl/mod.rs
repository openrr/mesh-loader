//! [STL] (.stl) parser.
//!
//! [STL]: https://en.wikipedia.org/wiki/STL_(file_format)

use std::{io, path::Path, str};

use self::error::ErrorKind;
use crate::{
    utils::{
        bytes::{memchr_naive_table, starts_with},
        float,
    },
    Color4, Material, Mesh, Scene, Vec3,
};

/// Parses meshes from bytes of binary or ASCII STL.
#[inline]
pub fn from_slice(bytes: &[u8]) -> io::Result<Scene> {
    from_slice_internal(bytes, None, false)
}

pub(crate) fn from_slice_internal(
    bytes: &[u8],
    path: Option<&Path>,
    parse_color: bool,
) -> io::Result<Scene> {
    let mut meshes = Vec::with_capacity(1);
    if is_ascii_stl(bytes) {
        match read_ascii_stl(bytes, &mut meshes) {
            Ok(()) => {
                let materials = (0..meshes.len()).map(|_| Material::default()).collect();
                return Ok(Scene { materials, meshes });
            }
            // If there is solid but no space or line break after solid or no
            // facet normal, even valid ASCII text may be binary STL.
            Err(
                ErrorKind::NotAscii("solid", _)
                | ErrorKind::ExpectedSpace("solid", _)
                | ErrorKind::ExpectedNewline("solid", _)
                | ErrorKind::Expected("facet", _),
            ) if meshes.is_empty() => {}
            Err(e) => return Err(e.into_io_error(bytes, path)),
        }
    }
    match read_binary_header(bytes, parse_color) {
        Ok(header) => {
            let mesh = read_binary_triangles(&header);
            let mut material = Material::default();
            if header.reverse_color && mesh.colors[0].is_empty() {
                let color = header.default_color;
                material.color.diffuse = Some(color);
                material.color.specular = Some(color);
            }
            meshes.push(mesh);
            Ok(Scene {
                materials: vec![material],
                meshes,
            })
        }
        Err(e) => Err(e.into_io_error(bytes, path)),
    }
}

// An ASCII STL buffer will begin with "solid NAME", where NAME is optional.
// Note: The "solid NAME" check is necessary, but not sufficient, to determine
// if the buffer is ASCII; a binary header could also begin with "solid NAME".
fn is_ascii_stl(mut bytes: &[u8]) -> bool {
    // Use skip_spaces_and_lines_until_token instead of starts_with here
    // because some ASCII STL files has space before solid.
    // https://grep.app/search?q=%5E%20endsolid&regexp=true&case=true
    let is_ascii = skip_spaces_and_lines_until_token(&mut bytes, b"solid");

    if is_ascii {
        // This check is now performed with a delay within read_ascii_stl.
        // See the comment on ASCII check for stings after solid for more.
        // // A lot of importers are write solid even if the file is binary.
        // // So we have to check for ASCII-characters.
        // if !bytes.is_ascii() {
        //     is_ascii = false;
        // }
    }
    is_ascii
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
const TRIANGLE_START: usize = HEADER_SIZE + TRIANGLE_COUNT_SIZE;
const TRIANGLE_SIZE: usize = 50;

struct BinaryHeader<'a> {
    default_color: Color4,
    parse_color: bool,
    reverse_color: bool,
    triangle_bytes: &'a [u8],
}

fn read_binary_header(bytes: &[u8], parse_color: bool) -> Result<BinaryHeader<'_>, ErrorKind> {
    if bytes.len() < TRIANGLE_START {
        return Err(ErrorKind::TooSmall);
    }

    let header = &bytes[..HEADER_SIZE];
    let triangle_bytes = &bytes[TRIANGLE_START..];

    let extra_bytes = triangle_bytes.len() % TRIANGLE_SIZE;
    if extra_bytes != 0 {
        if extra_bytes == 1 && triangle_bytes.ends_with(b"\n")
            || extra_bytes == 2 && triangle_bytes.ends_with(b"\r\n")
        {
            // Some buggy STL files have a newline after triangles...
        } else {
            return Err(ErrorKind::InvalidSize);
        }
    }

    // Some STL files contain bogus count.
    // So we calculate num_triangles based on the size of the input.
    // let num_triangles = &bytes[HEADER_SIZE..TRIANGLE_START];
    // let num_triangles = u32::from_le_bytes(num_triangles.try_into().unwrap());
    // assert_eq!(triangle_bytes.len() / TRIANGLE_SIZE, num_triangles as usize);
    let num_triangles = triangle_bytes.len() / TRIANGLE_SIZE;
    let num_vertices = num_triangles * 3;
    if u32::try_from(num_vertices).is_err() {
        // face is [u32; 3], so num_vertices must not exceed u32::MAX.
        return Err(ErrorKind::TooManyTriangles);
    }

    // Use the same default color (light gray) as assimp: https://github.com/assimp/assimp/blob/v5.3.1/code/AssetLib/STL/STLLoader.cpp#L183-L184
    let mut default_color = [0.6, 0.6, 0.6, 0.6];
    let mut reverse_color = false;
    if parse_color {
        // Handling colors in STL is not standardized. We use the same way as assimp.
        // https://github.com/assimp/assimp/blob/v5.3.1/code/AssetLib/STL/STLLoader.cpp#L413-L431
        let mut s = header;
        let expect = b"COLOR=";
        while s.len() >= expect.len() + 4 {
            if token(&mut s, expect) {
                const INV_BYTE: f32 = 1. / 255.;
                reverse_color = true;
                default_color = [
                    s[0] as f32 * INV_BYTE,
                    s[1] as f32 * INV_BYTE,
                    s[2] as f32 * INV_BYTE,
                    s[3] as f32 * INV_BYTE,
                ];
                break;
            }
            s = &s[1..];
        }
    }

    Ok(BinaryHeader {
        default_color,
        parse_color,
        reverse_color,
        triangle_bytes,
    })
}

fn read_binary_triangles(header: &BinaryHeader<'_>) -> Mesh {
    let bytes = header.triangle_bytes;

    let chunks = bytes.chunks_exact(TRIANGLE_SIZE);
    let num_triangles = chunks.len();
    let num_vertices = num_triangles * 3;
    // Even if we allocate capacity with reserve_exact, the compiler does not
    // seem to be able to remove the capacity check in push/extend_from_slice,
    // so we first allocate zeros and then copy the actual data to it.
    // If the size is relatively small, the fastest way here is to allocate Vec,
    // write to it using unsafe ways, and finally call set_len.
    // However, as the size increases, this way becomes equivalent performance
    // (at least on x86_64 Linux & AArch64 macOS), and in some cases this way is
    // finally 10% faster (at least on AArch64 macOS).
    let mut mesh = Mesh {
        vertices: vec![[0., 0., 0.]; num_vertices],
        normals: vec![[0., 0., 0.]; num_vertices],
        faces: vec![[0, 0, 0]; num_triangles],
        ..Default::default()
    };

    let mut vertices_len = 0;
    let has_color_mask = if header.parse_color { 1 << 15 } else { 0 };

    for (((chunk, vertices), normals), face) in chunks
        .zip(mesh.vertices.chunks_exact_mut(3))
        .zip(mesh.normals.chunks_exact_mut(3))
        .zip(&mut mesh.faces)
    {
        let triangle = read_binary_triangle(chunk);

        vertices.clone_from_slice(&triangle.vertices);
        normals.clone_from_slice(&[triangle.normal; 3]);
        *face = [vertices_len, vertices_len + 1, vertices_len + 2];

        // Handling colors in STL is not standardized. We use the same way as assimp.
        // https://github.com/assimp/assimp/blob/v5.3.1/code/AssetLib/STL/STLLoader.cpp#L502-L529
        if triangle.color & has_color_mask != 0 {
            const INV_VAL: f32 = 1. / 31.;
            if mesh.colors[0].is_empty() {
                mesh.colors[0] = vec![header.default_color; num_vertices];
            }
            let a = 1.;
            let color = if header.reverse_color {
                let r = (triangle.color & 0x1f) as f32 * INV_VAL;
                let g = ((triangle.color & (0x1f << 5)) >> 5) as f32 * INV_VAL;
                let b = ((triangle.color & (0x1f << 10)) >> 10) as f32 * INV_VAL;
                [r, g, b, a]
            } else {
                let b = (triangle.color & 0x1f) as f32 * INV_VAL;
                let g = ((triangle.color & (0x1f << 5)) >> 5) as f32 * INV_VAL;
                let r = ((triangle.color & (0x1f << 10)) >> 10) as f32 * INV_VAL;
                [r, g, b, a]
            };
            mesh.colors[0][vertices_len as usize..vertices_len as usize + 3]
                .copy_from_slice(&[color, color, color]);
        }

        vertices_len += 3;
    }

    mesh
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
    let color = u16::from_le_bytes(buf[..2].try_into().unwrap());
    Triangle {
        normal,
        vertices: [vertex1, vertex2, vertex3],
        color,
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
fn read_ascii_stl(mut s: &[u8], meshes: &mut Vec<Mesh>) -> Result<(), ErrorKind> {
    loop {
        let mut mesh = Mesh::default();

        // solid [name]
        let expected = "solid";
        if !skip_spaces_and_lines_until_token(&mut s, expected.as_bytes()) {
            if s.is_empty() {
                // eof
                if meshes.is_empty() {
                    return Err(ErrorKind::Expected(expected, s.len()));
                }
                break;
            }
            return Err(ErrorKind::Expected(expected, s.len()));
        }
        if !skip_spaces(&mut s) {
            return Err(ErrorKind::ExpectedSpace(expected, s.len()));
        }
        match memchr_naive_table(LINE, &TABLE, s) {
            Some(n) => {
                let mut name = &s[..n];
                // The only strings we need to explicitly check for ASCII are the
                // strings after solid and endsolid. Any other occurrence of
                // a non-ASCII character elsewhere will result in the normal syntax
                // error of simply not finding the expected character or whitespace.
                if !name.is_ascii() {
                    return Err(ErrorKind::NotAscii(expected, s.len()));
                }
                if let Some(n) = memchr_naive_table(SPACE, &TABLE, name) {
                    // Ignore contents after the name.
                    // https://en.wikipedia.org/wiki/STL_(file_format)#ASCII
                    // > The remainder of the line is ignored and is sometimes used to
                    // > store metadata (e.g., filename, author, modification date, etc).
                    name = &name[..n];
                }
                let name = str::from_utf8(name).unwrap();
                Mesh::set_name(&mut mesh, name);
                s = &s[n + 1..];
            }
            None => return Err(ErrorKind::ExpectedNewline(expected, s.len())),
        }

        loop {
            // facet normal <n1> <n2> <n3>
            // Note: space in facet and normal can be multiple
            // https://github.com/apache/commons-geometry/blob/fb537c8505644262f70fde6e4a0b109e06363340/commons-geometry-io-euclidean/src/test/java/org/apache/commons/geometry/io/euclidean/threed/stl/TextStlFacetDefinitionReaderTest.java#L124-L125
            let expected = "facet";
            if !skip_spaces_and_lines_until_token(&mut s, expected.as_bytes()) {
                break;
            }
            if !skip_spaces(&mut s) {
                return Err(ErrorKind::ExpectedSpace(expected, s.len()));
            }
            let expected = "normal";
            if !token(&mut s, expected.as_bytes()) {
                return Err(ErrorKind::Expected(expected, s.len()));
            }
            let mut normal = [0.; 3];
            for normal in &mut normal {
                if !skip_spaces(&mut s) {
                    return Err(ErrorKind::ExpectedSpace(expected, s.len()));
                }
                match float::parse_partial::<f32>(s) {
                    Some((f, n)) => {
                        *normal = f;
                        s = &s[n..];
                    }
                    None => return Err(ErrorKind::Float(s.len())),
                }
            }
            if !skip_spaces_until_line(&mut s) {
                return Err(ErrorKind::ExpectedNewline(expected, s.len()));
            }

            // outer loop
            // Note: space in facet and normal can be multiple
            // https://github.com/apache/commons-geometry/blob/fb537c8505644262f70fde6e4a0b109e06363340/commons-geometry-io-euclidean/src/test/java/org/apache/commons/geometry/io/euclidean/threed/stl/TextStlFacetDefinitionReaderTest.java#L124-L125
            let expected = "outer";
            if !skip_spaces_and_lines_until_token(&mut s, expected.as_bytes()) {
                return Err(ErrorKind::Expected(expected, s.len()));
            }
            if !skip_spaces(&mut s) {
                return Err(ErrorKind::ExpectedSpace(expected, s.len()));
            }
            let expected = "loop";
            if !token(&mut s, expected.as_bytes()) {
                return Err(ErrorKind::Expected(expected, s.len()));
            }
            if !skip_spaces_until_line(&mut s) {
                return Err(ErrorKind::ExpectedNewline(expected, s.len()));
            }

            // vertex <v1x> <v1y> <v1z>
            // vertex <v2x> <v2y> <v2z>
            // vertex <v3x> <v3y> <v3z>
            let expected = "vertex";
            let mut vertices = [[0.; 3]; 3];
            for vertex in &mut vertices {
                if !skip_spaces_and_lines_until_token(&mut s, expected.as_bytes()) {
                    return Err(ErrorKind::Expected(expected, s.len()));
                }
                for vertex in vertex {
                    if !skip_spaces(&mut s) {
                        return Err(ErrorKind::ExpectedSpace(expected, s.len()));
                    }
                    match float::parse_partial::<f32>(s) {
                        Some((f, n)) => {
                            *vertex = f;
                            s = &s[n..];
                        }
                        None => return Err(ErrorKind::Float(s.len())),
                    }
                }
                if !skip_spaces_until_line(&mut s) {
                    return Err(ErrorKind::ExpectedNewline(expected, s.len()));
                }
            }

            // endloop
            let expected = "endloop";
            if !skip_spaces_and_lines_until_token(&mut s, expected.as_bytes()) {
                return Err(ErrorKind::Expected(expected, s.len()));
            }
            if !skip_spaces_until_line(&mut s) {
                return Err(ErrorKind::ExpectedNewline(expected, s.len()));
            }

            // endfacet
            let expected = "endfacet";
            if !skip_spaces_and_lines_until_token(&mut s, expected.as_bytes()) {
                return Err(ErrorKind::Expected(expected, s.len()));
            }
            if !skip_spaces_until_line(&mut s) {
                return Err(ErrorKind::ExpectedNewline(expected, s.len()));
            }

            Mesh::push_triangle(
                &mut mesh,
                Triangle {
                    normal,
                    vertices,
                    color: 0,
                },
            );
        }

        // endsolid [name]
        let expected = "endsolid";
        if !token(&mut s, expected.as_bytes()) {
            return Err(ErrorKind::Expected(expected, s.len()));
        }
        // Skip checking endsolid because some exporters have generated the wrong STL about endsolid.
        // https://github.com/assimp/assimp/issues/3756
        match memchr_naive_table(LINE, &TABLE, s) {
            Some(n) => {
                if !s[..n].is_ascii() {
                    return Err(ErrorKind::NotAscii(expected, s.len())); // See the comment on ASCII check for stings after solid for more.
                }
                s = &s[n + 1..];
            }
            None => {
                if !s.is_ascii() {
                    return Err(ErrorKind::NotAscii(expected, s.len())); // See the comment on ASCII check for stings after solid for more.
                }
                s = &[];
            }
        }

        meshes.push(mesh);
    }

    Ok(())
}

const __: u8 = 0;
// [ \r\n\t]
// Note: Unlike is_ascii_whitespace, FORM FEED ('\x0C') is not included.
// https://en.wikipedia.org/wiki/STL_(file_format)#ASCII
// > Whitespace (spaces, tabs, newlines) may be used anywhere in the file except within numbers or words.
const WS: u8 = SPACE | LINE;
// [ \t]
const SPACE: u8 = 1 << 0;
// [\r\n]
const LINE: u8 = 1 << 1;
const LN: u8 = LINE;
const NL: u8 = SPACE;
// [s]
const S_: u8 = 1 << 2;
// [e]
const E_: u8 = 1 << 3;
// [f]
const F_: u8 = 1 << 4;
// [o]
const O_: u8 = 1 << 5;
// [v]
const V_: u8 = 1 << 6;

static TABLE: [u8; 256] = [
    //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
    __, __, __, __, __, __, __, __, __, NL, LN, __, __, LN, __, __, // 0
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 1
    NL, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 2
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 3
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 4
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 5
    __, __, __, __, __, E_, F_, __, __, __, __, __, __, __, __, O_, // 6
    __, __, __, S_, __, __, V_, __, __, __, __, __, __, __, __, __, // 7
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
fn skip_whitespace_until_byte(s: &mut &[u8], byte_mask: u8, whitespace_mask: u8) -> bool {
    while let Some((&b, s_next)) = s.split_first() {
        let b = TABLE[b as usize];
        if b & byte_mask != 0 {
            *s = s_next;
            return true;
        }
        if b & whitespace_mask != 0 {
            *s = s_next;
            continue;
        }
        break;
    }
    false
}

#[inline]
fn skip_spaces_until_line(s: &mut &[u8]) -> bool {
    skip_whitespace_until_byte(s, LINE, SPACE)
}

#[inline]
fn skip_spaces(s: &mut &[u8]) -> bool {
    let start = *s;
    while let Some((&b, s_next)) = s.split_first() {
        let b = TABLE[b as usize];
        if b & SPACE != 0 {
            *s = s_next;
            continue;
        }
        break;
    }
    start.len() != s.len()
}

#[inline]
fn token(s: &mut &[u8], token: &'static [u8]) -> bool {
    if starts_with(s, token) {
        *s = &s[token.len()..];
        true
    } else {
        false
    }
}

#[inline(always)] // Ensure the code creating token_start_mask and check_start is inlined.
fn skip_spaces_and_lines_until_token(s: &mut &[u8], token: &'static [u8]) -> bool {
    let token_start_mask = TABLE[token[0] as usize];
    debug_assert_ne!(token_start_mask, __);
    let check_start = match token.len() {
        4 | 8 | 12 | 16 => 0,
        _ => 1,
    };
    while let Some((&b, s_next)) = s.split_first() {
        let b = TABLE[b as usize];
        if b & token_start_mask != 0 {
            if starts_with(&s[check_start..], &token[check_start..]) {
                *s = &s[token.len()..];
                return true;
            }
            break;
        }
        if b & WS != 0 {
            *s = s_next;
            continue;
        }
        break;
    }
    false
}

struct Triangle {
    normal: Vec3,
    vertices: [Vec3; 3],
    color: u16,
}

trait FromStl: Sized {
    type Context;

    /// Appends a triangle.
    fn push_triangle(cx: &mut Self::Context, triangle: Triangle);

    /// Sets the name.
    fn set_name(cx: &mut Self::Context, name: &str);
}

impl FromStl for Mesh {
    type Context = Self;

    #[inline]
    fn push_triangle(mesh: &mut Self::Context, triangle: Triangle) {
        // With binary STL, reserve checks that the max length of cx.vertices
        // will not be greater than u32::MAX.
        // With ASCII STL, the max length of cx.vertices will not be too large,
        // since much more bytes is required per triangle than for binary STL.
        #[allow(clippy::cast_possible_truncation)]
        let vertices_indices = [
            mesh.vertices.len() as u32,
            (mesh.vertices.len() + 1) as u32,
            (mesh.vertices.len() + 2) as u32,
        ];

        mesh.vertices.extend_from_slice(&triangle.vertices);
        mesh.normals.resize(mesh.normals.len() + 3, triangle.normal);
        mesh.faces.push(vertices_indices);
    }

    fn set_name(mesh: &mut Self::Context, name: &str) {
        mesh.name = name.to_owned();
    }
}

mod error {
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
                        write!(f, "expected space before floats")
                    } else {
                        write!(f, "expected space after {msg}")
                    }
                }
                Self::ExpectedNewline(msg, ..) => {
                    if msg == "solid" {
                        write!(f, "expected newline after solid name")
                    } else if msg == "normal" || msg == "vertex" {
                        write!(f, "expected newline after floats")
                    } else {
                        write!(f, "expected newline after {msg}")
                    }
                }
                Self::Expected(msg, remaining) => {
                    if msg == "solid" && remaining != 0 {
                        write!(f, "expected solid or eof")
                    } else if msg == "endsolid" {
                        write!(f, "expected facet normal or endsolid")
                    } else {
                        write!(f, "expected {msg}")
                    }
                }
                Self::Float(..) => write!(f, "error while parsing a float"),
                Self::NotAscii(..) => write!(f, "invalid ASCII"),
                // binary STL error
                Self::TooSmall => write!(
                    f,
                    "failed to determine STL storage representation: \
                     not valid ASCII STL and size is too small as binary STL"
                ),
                Self::InvalidSize => write!(
                    f,
                    "failed to determine STL storage representation: \
                     not valid ASCII STL and size is invalid as binary STL"
                ),
                Self::TooManyTriangles => write!(f, "too many triangles"),
            }
        }
    }
}
