//! [Wavefront OBJ] (.obj) parser.
//!
//! [Wavefront OBJ]: https://en.wikipedia.org/wiki/Wavefront_.obj_file

use std::{
    collections::{hash_map, HashMap},
    io,
    path::Path,
};

use crate::{
    error::{self, Location},
    utils::bytes::{Lines, Split},
    Face, Mesh, Vec2, Vec3,
};

/// Parses meshes from bytes of Wavefront OBJ text.
pub fn from_slice(bytes: &[u8], file: Option<&Path>) -> io::Result<Vec<Mesh>> {
    let mut p = Parser::new(bytes, file);
    match p.read_contents() {
        Ok(meshes) => Ok(meshes),
        Err(e) => Err(error::with_location(e, p.location())),
    }
}

struct Parser<'a> {
    lines: Lines<'a>,
    file: Option<&'a Path>,
    column: usize,
}

impl<'a> Parser<'a> {
    fn new(bytes: &'a [u8], file: Option<&'a Path>) -> Self {
        Self {
            lines: Lines::new(bytes),
            file,
            column: 0,
        }
    }

    fn read_line(&mut self) -> Option<()> {
        self.column = 0;
        while self.lines.next().is_some() {
            self.skip_spaces();
            let bytes = self.bytes();
            if !bytes.is_empty() && bytes[0] != b'#' {
                return Some(());
            }
        }
        None
    }

    fn bytes(&mut self) -> &'a [u8] {
        self.lines.current().get(self.column..).unwrap_or_default()
    }

    fn skip_spaces(&mut self) -> bool {
        let prev = self.column;
        while self.bytes().first().map_or(false, u8::is_ascii_whitespace) {
            self.column += 1;
        }
        self.column != prev
    }

    fn read_contents(&mut self) -> io::Result<Vec<Mesh>> {
        let mut cx = ReadContext::default();

        if self.lines.next().is_none() {
            bail!("unexpected eof");
        }

        while self.read_line().is_some() {
            let mut bytes = Split::new(b' ', self.bytes());
            let tag = bytes.next().unwrap();
            match tag {
                b"v" => {
                    cx.vertices.push(self.read_vec3d()?);
                }
                b"vt" => {
                    cx.texcoords.push(self.read_vt()?);
                }
                b"vn" => {
                    cx.normals.push(self.read_vec3d()?);
                }
                b"f" => {
                    let mut i = 0;
                    for word in bytes {
                        let mut curr_ids: [i32; 3] = [i32::MAX; 3];

                        for (i, w) in word.split(|b| *b == b'/').enumerate() {
                            if i == 0 || !w.is_empty() {
                                if let Ok(w) = std::str::from_utf8(w) {
                                    let idx = w.parse::<i32>();
                                    if let Ok(id) = idx {
                                        curr_ids[i] = id - 1;
                                        continue;
                                    }
                                }
                                bail!(
                                    "could not parse '{}' as i32",
                                    crate::error::utf8_or_byte_array(w)
                                );
                            }
                        }

                        if i > 2 {
                            // on the fly triangulation as trangle fan
                            let g = &mut cx.groups_ids[cx.current_group];
                            let p1 = (*g)[g.len() - i];
                            let p2 = (*g)[g.len() - 1];
                            g.push(p1);
                            g.push(p2);
                        }

                        if curr_ids[1] == i32::MAX {
                            cx.ignore_texcoords = true;
                        }
                        if curr_ids[2] == i32::MAX {
                            cx.ignore_normals = true;
                        }

                        // Handle relatives indice
                        if curr_ids[0] < 0 {
                            curr_ids[0] = cx.vertices.len() as i32 + curr_ids[0] + 1
                        }
                        if curr_ids[1] < 0 {
                            curr_ids[1] = cx.texcoords.len() as i32 + curr_ids[1] + 1
                        }
                        if curr_ids[2] < 0 {
                            curr_ids[2] = cx.normals.len() as i32 + curr_ids[2] + 1
                        }
                        assert!(curr_ids[0] >= 0 && curr_ids[1] >= 0 && curr_ids[2] >= 0);
                        cx.groups_ids[cx.current_group].push([
                            curr_ids[0] as u32,
                            curr_ids[1] as u32,
                            curr_ids[2] as u32,
                        ]);

                        i += 1;
                    }

                    // there is not enough vertex to form a triangle. Complete it.
                    if i < 2 {
                        for _ in 0usize..3 - i {
                            let last = *(*cx.groups_ids)[cx.current_group].last().unwrap();
                            cx.groups_ids[cx.current_group].push(last);
                        }
                    }
                }
                b"mtllib" => { /* TODO */ }
                b"usemtl" => { /* TODO */ }
                b"g" => {
                    let mut name = self
                        .file
                        .map(Path::as_os_str)
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                        .into_bytes();
                    let mut first = true;
                    for part in bytes {
                        if first {
                            first = false;
                            name.push(b'/');
                        } else {
                            name.push(b' ');
                        }
                        name.copy_from_slice(part);
                    }
                    match cx.groups.entry(name) {
                        hash_map::Entry::Occupied(e) => cx.current_group = *e.into_mut(),
                        hash_map::Entry::Vacant(e) => {
                            cx.current_group = cx.groups_ids.len();
                            cx.groups_ids.push(vec![]);
                            e.insert(cx.current_group);
                        }
                    };
                }
                _ => {
                    debug!(
                        "unrecognized tag '{}' ({})",
                        crate::error::utf8_or_byte_array(tag),
                        self.location()
                    )
                }
            }
        }

        Ok(cx.end())
    }

    fn read_vt(&mut self) -> io::Result<Vec2> {
        let x = self.read_float()?;
        if !self.bytes().first().map_or(true, u8::is_ascii_whitespace) {
            bail!("expected whitespace after float");
        }
        self.skip_spaces();
        if self.bytes().is_empty() {
            return Ok([x, 0.0]);
        }

        let y = self.read_float()?;

        Ok([x, y])
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

#[derive(Default)]
struct ReadContext {
    vertices: Vec<Vec3>,
    texcoords: Vec<Vec2>,
    normals: Vec<Vec3>,
    ignore_texcoords: bool,
    ignore_normals: bool,
    groups: HashMap<Vec<u8>, usize>,
    groups_ids: Vec<Vec<Face>>,
    current_group: usize,
}

impl ReadContext {
    fn end(self) -> Vec<Mesh> {
        let mut mesh = Mesh {
            vertices: self.vertices,
            ..Default::default()
        };
        if !self.ignore_texcoords {
            mesh.texcoords[0] = self.texcoords;
        }
        if !self.ignore_normals {
            mesh.normals = self.normals;
        }
        // TODO: handle groups
        vec![mesh]
    }
}
