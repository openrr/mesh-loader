//! [Wavefront OBJ] (.obj) parser.
//!
//! [Wavefront OBJ]: https://en.wikipedia.org/wiki/Wavefront_.obj_file

#![allow(clippy::collapsible_if, clippy::many_single_char_names)]

mod error;

use std::{
    collections::HashMap,
    io, mem,
    path::{Path, PathBuf},
    str,
};

use self::error::ErrorKind;
use crate::{
    common,
    utils::{
        bytes::{from_utf8_lossy, memchr_naive, memchr_naive_table, path_from_bytes, starts_with},
        float, int,
        utf16::decode_bytes,
    },
    Color4, Mesh, Scene, ShadingModel, Vec2, Vec3,
};

/// Parses meshes from bytes of Wavefront OBJ text.
pub fn from_slice<B: AsRef<[u8]>, F: FnMut(&Path) -> io::Result<B>>(
    bytes: &[u8],
    path: Option<&Path>,
    mut reader: F,
) -> io::Result<Scene> {
    // If it is UTF-16 with BOM, it is converted to UTF-8, otherwise it is parsed as bytes.
    // We don't require UTF-8 here, as we want to support files that are partially non-UTF-8 like:
    // https://github.com/assimp/assimp/blob/v5.3.1/test/models/OBJ/regr01.mtl#L67
    let bytes = &decode_bytes(bytes)?;
    match read_obj(bytes, path, &mut |path, materials, material_map| {
        match reader(path) {
            Ok(bytes) => read_mtl(bytes.as_ref(), Some(path), materials, material_map),
            // ignore reader error for now
            // TODO: logging?
            Err(_e) => Ok(()),
        }
    }) {
        Ok((meshes, materials)) => {
            let materials = meshes
                .iter()
                .map(|m| {
                    materials
                        .get(m.material_index as usize)
                        .cloned()
                        .unwrap_or_default()
                })
                .collect();
            Ok(Scene { materials, meshes })
        }
        Err(e) => Err(e.into_io_error(bytes, path)),
    }
}

// -----------------------------------------------------------------------------
// OBJ

fn read_obj(
    mut s: &[u8],
    obj_path: Option<&Path>,
    reader: &mut dyn FnMut(
        &Path,
        &mut Vec<common::Material>,
        &mut HashMap<Vec<u8>, u32>,
    ) -> io::Result<()>,
) -> Result<(Vec<Mesh>, Vec<common::Material>), ErrorKind> {
    let mut meshes = Vec::with_capacity(1); // TODO: right default capacity?

    // TODO: use with_capacity
    let mut vertices = vec![];
    let mut normals = vec![];
    let mut texcoords = vec![];
    let mut colors = vec![];
    let mut face = Vec::with_capacity(3);
    let mut faces: Vec<Face> = vec![];
    let mut current_group: &[u8] = b"default";
    let mut current_material: &[u8] = &[];
    let mut materials = vec![];
    let mut material_map = HashMap::new();

    while let Some((&c, s_next)) = s.split_first() {
        match c {
            b'v' => {
                s = s_next;
                match s.first() {
                    Some(b' ' | b'\t') => {
                        skip_spaces(&mut s);
                        read_v(&mut s, &mut vertices, &mut colors)?;
                        if !colors.is_empty() && colors.len() < vertices.len() {
                            colors.resize(vertices.len(), [0.; 3]);
                        }
                        continue;
                    }
                    Some(b'n') => {
                        s = &s[1..];
                        if skip_spaces(&mut s) {
                            read_vn(&mut s, &mut normals)?;
                            continue;
                        }
                    }
                    Some(b't') => {
                        s = &s[1..];
                        if skip_spaces(&mut s) {
                            read_vt(&mut s, &mut texcoords)?;
                            continue;
                        }
                    }
                    // ignore vp or other unknown
                    _ => {}
                }
            }
            b'f' => {
                s = s_next;
                if skip_spaces(&mut s) {
                    read_f(
                        &mut s, &mut faces, &mut face, &vertices, &texcoords, &normals,
                    )?;
                    continue;
                }
            }
            b'u' => {
                s = s_next;
                if token(&mut s, &b"usemtl"[1..]) {
                    if skip_spaces(&mut s) {
                        let (name, s_next) = name(s);
                        if name != current_material {
                            let material_index = material_map.get(current_material).copied();
                            push_mesh(
                                &mut meshes,
                                &mut faces,
                                &vertices,
                                &texcoords,
                                &normals,
                                &colors,
                                current_group,
                                material_index,
                            )?;
                            current_material = name;
                        }
                        s = s_next;
                        continue;
                    }
                }
            }
            b'm' => {
                s = s_next;
                if token(&mut s, &b"mtllib"[1..]) {
                    if skip_spaces(&mut s) {
                        let (path, s_next) = name(s);
                        let path = if path.is_empty() {
                            None
                        } else {
                            path_from_bytes(path).ok()
                        };
                        if let Some(path) = path {
                            match obj_path.and_then(Path::parent) {
                                Some(parent) => {
                                    reader(&parent.join(path), &mut materials, &mut material_map)
                                        .map_err(ErrorKind::Io)?;
                                }
                                None => {} // ignored
                            }
                        }
                        s = s_next;
                        continue;
                    }
                }
                // ignore mg or other unknown
            }
            b'g' => {
                s = s_next;
                if skip_spaces(&mut s) {
                    let (mut name, s_next) = name(s);
                    if name.is_empty() {
                        name = b"default";
                    }
                    if name != current_group {
                        let material_index = material_map.get(current_material).copied();
                        push_mesh(
                            &mut meshes,
                            &mut faces,
                            &vertices,
                            &texcoords,
                            &normals,
                            &colors,
                            current_group,
                            material_index,
                        )?;
                        current_material = &[];
                        current_group = name;
                    }
                    s = s_next;
                    continue;
                }
            }
            _ => {}
        }
        // ignore comment, p, l, s, mg, o, or other unknown
        skip_any_until_line(&mut s);
    }

    let material_index = material_map.get(current_material).copied();
    push_mesh(
        &mut meshes,
        &mut faces,
        &vertices,
        &texcoords,
        &normals,
        &colors,
        current_group,
        material_index,
    )?;

    Ok((meshes, materials))
}

#[inline(always)]
fn read_v(
    s: &mut &[u8],
    vertices: &mut Vec<Vec3>,
    colors: &mut Vec<Vec3>,
) -> Result<(), ErrorKind> {
    // v <x> <y> <z> ([w] | [<r> <g> <b>])
    let vertex = read_float3(s, "v")?;
    let has_space = skip_spaces(s);
    match s.first() {
        Some(b'\n' | b'\r') | None => {
            vertices.push(vertex);
            *s = s.get(1..).unwrap_or_default();
            return Ok(());
        }
        _ if !has_space => return Err(ErrorKind::ExpectedSpace("v", s.len())),
        _ => {}
    }
    // [w] or [r]
    let w = match float::parse_partial::<f32>(s) {
        Some((f, n)) => {
            *s = &s[n..];
            f
        }
        None => return Err(ErrorKind::Float(s.len())),
    };
    let has_space = skip_spaces(s);
    match s.first() {
        Some(b'\n' | b'\r') | None => {
            // is homogeneous vector
            if w == 0. {
                return Err(ErrorKind::InvalidW(s.len()));
            }
            vertices.push([vertex[0] / w, vertex[1] / w, vertex[2] / w]);
            *s = s.get(1..).unwrap_or_default();
            return Ok(());
        }
        _ if !has_space => return Err(ErrorKind::ExpectedSpace("v", s.len())),
        _ => {}
    }
    vertices.push(vertex);
    // is vertex color
    let r = w;
    let g = match float::parse_partial::<f32>(s) {
        Some((f, n)) => {
            *s = &s[n..];
            f
        }
        None => return Err(ErrorKind::Float(s.len())),
    };
    if !skip_spaces(s) {
        return Err(ErrorKind::ExpectedSpace("v", s.len()));
    }
    let b = match float::parse_partial::<f32>(s) {
        Some((f, n)) => {
            *s = &s[n..];
            f
        }
        None => return Err(ErrorKind::Float(s.len())),
    };
    colors.push([r, g, b]);
    if !skip_spaces_until_line(s) {
        return Err(ErrorKind::ExpectedNewline("v", s.len()));
    }
    Ok(())
}

fn read_vn(s: &mut &[u8], normals: &mut Vec<Vec3>) -> Result<(), ErrorKind> {
    // vn <i> <j> <k>
    let normal = read_float3(s, "vn")?;
    normals.push(normal);
    if !skip_spaces_until_line(s) {
        return Err(ErrorKind::ExpectedNewline("vn", s.len()));
    }
    Ok(())
}

fn read_vt(s: &mut &[u8], texcoords: &mut Vec<Vec2>) -> Result<(), ErrorKind> {
    // vt <u> [v=0] [w=0]
    let mut texcoord = [0.; 2];
    // <u>
    match float::parse_partial::<f32>(s) {
        Some((f, n)) => {
            texcoord[0] = f;
            *s = &s[n..];
        }
        None => return Err(ErrorKind::Float(s.len())),
    }
    let has_space = skip_spaces(s);
    match s.first() {
        Some(b'\n' | b'\r') | None => {
            texcoords.push(texcoord);
            *s = s.get(1..).unwrap_or_default();
            return Ok(());
        }
        _ if !has_space => return Err(ErrorKind::ExpectedSpace("vt", s.len())),
        _ => {}
    }
    // [v=0]
    match float::parse_partial::<f32>(s) {
        Some((f, n)) => {
            texcoord[1] = f;
            *s = &s[n..];
        }
        None => return Err(ErrorKind::Float(s.len())),
    }
    texcoords.push(texcoord);
    let has_space = skip_spaces(s);
    match s.first() {
        Some(b'\n' | b'\r') | None => {
            *s = s.get(1..).unwrap_or_default();
            return Ok(());
        }
        _ if !has_space => return Err(ErrorKind::ExpectedSpace("vt", s.len())),
        _ => {}
    }
    // [w=0]
    match float::parse_partial::<f32>(s) {
        Some((_f, n)) => {
            // ignored
            *s = &s[n..];
        }
        None => return Err(ErrorKind::Float(s.len())),
    }
    if !skip_spaces_until_line(s) {
        return Err(ErrorKind::ExpectedNewline("vt", s.len()));
    }
    Ok(())
}

fn read_f(
    s: &mut &[u8],
    faces: &mut Vec<Face>,
    face: &mut Vec<[u32; 3]>,
    vertices: &[Vec3],
    texcoords: &[Vec2],
    normals: &[Vec3],
) -> Result<(), ErrorKind> {
    // f <v1>/[vt1]/[vn1] <v2>/[vt2]/[vn2] <v3>/[vt3]/[vn3] ...
    let mut f;
    match memchr_naive_table(LINE, &TABLE, s) {
        Some(n) => {
            f = &s[..n];
            *s = &s[n + 1..];
        }
        None => {
            f = s;
            *s = &[];
        }
    };
    while !f.is_empty() {
        let mut w;
        let f_next = match memchr_naive_table(SPACE, &TABLE, f) {
            Some(n) => {
                w = &f[..n];
                &f[n + 1..]
            }
            None => {
                w = f;
                &[]
            }
        };
        let mut idx = [u32::MAX; 3];
        let mut i;
        match memchr_naive(b'/', w) {
            Some(n) => {
                i = &w[..n];
                w = &w[n + 1..];
            }
            None => {
                i = w;
                w = &[];
            }
        };
        match int::parse::<i32>(i) {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_possible_wrap,
                clippy::cast_sign_loss
            )]
            Some(i) => {
                idx[0] = if i < 0 {
                    (vertices.len() as isize + i as isize) as u32
                } else {
                    (i - 1) as u32
                }
            }
            None => return Err(ErrorKind::Int(s.len() + !s.is_empty() as usize + f.len())),
        }
        match memchr_naive(b'/', w) {
            Some(n) => {
                i = &w[..n];
                w = &w[n + 1..];
            }
            None => {
                i = w;
                w = &[];
            }
        };
        if !i.is_empty() {
            match int::parse::<i32>(i) {
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_possible_wrap,
                    clippy::cast_sign_loss
                )]
                Some(i) => {
                    idx[1] = if i < 0 {
                        (texcoords.len() as isize + i as isize) as u32
                    } else {
                        (i - 1) as u32
                    }
                }
                None => return Err(ErrorKind::Int(s.len() + !s.is_empty() as usize + f.len())),
            }
        }
        i = w;
        if !i.is_empty() {
            match int::parse::<i32>(i) {
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_possible_wrap,
                    clippy::cast_sign_loss
                )]
                Some(i) => {
                    idx[2] = if i < 0 {
                        (normals.len() as isize + i as isize) as u32
                    } else {
                        (i - 1) as u32
                    }
                }
                None => return Err(ErrorKind::Int(s.len() + !s.is_empty() as usize + f.len())),
            }
        }
        f = f_next;
        skip_spaces(&mut f);
        face.push(idx);
    }
    match face.len() {
        1 => {
            faces.push(Face::Point([face[0]]));
            face.clear();
        }
        2 => {
            faces.push(Face::Line([face[0], face[1]]));
            face.clear();
        }
        3 => {
            faces.push(Face::Triangle([face[0], face[1], face[2]]));
            face.clear();
        }
        0 => return Err(ErrorKind::Expected("f", s.len())),
        // TODO: triangulate in place here?
        _ => faces.push(Face::Polygon(mem::take(face))),
    }
    Ok(())
}

fn read_float3(s: &mut &[u8], expected: &'static str) -> Result<[f32; 3], ErrorKind> {
    let mut floats = [0.; 3];
    match float::parse_partial::<f32>(s) {
        Some((f, n)) => {
            floats[0] = f;
            *s = &s[n..];
        }
        None => return Err(ErrorKind::Float(s.len())),
    }
    if !skip_spaces(s) {
        return Err(ErrorKind::ExpectedSpace(expected, s.len()));
    }
    match float::parse_partial::<f32>(s) {
        Some((f, n)) => {
            floats[1] = f;
            *s = &s[n..];
        }
        None => return Err(ErrorKind::Float(s.len())),
    }
    if !skip_spaces(s) {
        return Err(ErrorKind::ExpectedSpace(expected, s.len()));
    }
    match float::parse_partial::<f32>(s) {
        Some((f, n)) => {
            floats[2] = f;
            *s = &s[n..];
        }
        None => return Err(ErrorKind::Float(s.len())),
    }
    Ok(floats)
}

fn read_color(s: &mut &[u8], expected: &'static str) -> Result<[f32; 3], ErrorKind> {
    let mut floats = [0.; 3];
    // r
    match float::parse_partial::<f32>(s) {
        Some((f, n)) => {
            floats[0] = f;
            *s = &s[n..];
        }
        None => return Err(ErrorKind::Float(s.len())),
    }
    let has_space = skip_spaces(s);
    match s.first() {
        Some(b'\n' | b'\r') | None => {
            *s = s.get(1..).unwrap_or_default();
            return Ok(floats);
        }
        _ if !has_space => return Err(ErrorKind::ExpectedSpace(expected, s.len())),
        _ => {}
    }
    // g
    match float::parse_partial::<f32>(s) {
        Some((f, n)) => {
            floats[1] = f;
            *s = &s[n..];
        }
        None => return Err(ErrorKind::Float(s.len())),
    }
    if !skip_spaces(s) {
        return Err(ErrorKind::ExpectedSpace(expected, s.len()));
    }
    // b
    match float::parse_partial::<f32>(s) {
        Some((f, n)) => {
            floats[2] = f;
            *s = &s[n..];
        }
        None => return Err(ErrorKind::Float(s.len())),
    }
    if !skip_spaces_until_line(s) {
        return Err(ErrorKind::ExpectedNewline(expected, s.len()));
    }
    Ok(floats)
}

fn read_float1(s: &mut &[u8], expected: &'static str) -> Result<f32, ErrorKind> {
    match float::parse_partial::<f32>(s) {
        Some((f, n)) => {
            *s = &s[n..];
            if !skip_spaces_until_line(s) {
                return Err(ErrorKind::ExpectedNewline(expected, s.len()));
            }
            Ok(f)
        }
        None => Err(ErrorKind::Float(s.len())),
    }
}

#[inline(always)]
fn push_vertex(
    mesh: &mut Mesh,
    vert: [u32; 3],
    vertices: &[Vec3],
    colors: &[Vec3],
    texcoords: &[Vec2],
    normals: &[Vec3],
) -> Result<(), ErrorKind> {
    let v = vert[0] as usize;
    mesh.vertices
        .push(*vertices.get(v).ok_or(ErrorKind::Oob(v, 0))?);
    if !texcoords.is_empty() && vert[1] != u32::MAX {
        let vt = vert[1] as usize;
        mesh.texcoords[0].push(*texcoords.get(vt).ok_or(ErrorKind::Oob(vt, 0))?);
    }
    if !normals.is_empty() && vert[2] != u32::MAX {
        let vn = vert[2] as usize;
        mesh.normals
            .push(*normals.get(vn).ok_or(ErrorKind::Oob(vn, 0))?);
    }
    if !colors.is_empty() {
        let rgb = colors.get(v).ok_or(ErrorKind::Oob(v, 0))?;
        // a is 1 by default: https://github.com/assimp/assimp/blob/v5.3.1/code/AssetLib/Obj/ObjFileImporter.cpp#L233
        mesh.colors[0].push([rgb[0], rgb[1], rgb[2], 1.]);
    }
    Ok(())
}

fn push_mesh(
    meshes: &mut Vec<Mesh>,
    faces: &mut Vec<Face>,
    vertices: &[Vec3],
    texcoords: &[Vec2],
    normals: &[Vec3],
    colors: &[Vec3],
    current_group: &[u8],
    material_index: Option<u32>,
) -> Result<(), ErrorKind> {
    if !faces.is_empty() {
        let mut mesh = Mesh {
            name: from_utf8_lossy(current_group).into_owned(),
            material_index: material_index.unwrap_or(u32::MAX),
            ..Default::default()
        };
        // TODO
        // mesh.faces.reserve(faces.len());
        // mesh.vertices.reserve(faces.len() * 3);
        // if !texcoords.is_empty() {
        //     mesh.texcoords[0].reserve(faces.len() * 3);
        // }
        // if !normals.is_empty() {
        //     mesh.normals.reserve(faces.len() * 3);
        // }
        // if !colors.is_empty() {
        //     mesh.colors[0].reserve(faces.len() * 3);
        // }
        for face in &*faces {
            match face {
                Face::Point(_) | Face::Line(_) => {} // ignored
                Face::Triangle(face) => {
                    #[allow(clippy::cast_possible_truncation)]
                    let vertices_indices = [
                        mesh.vertices.len() as u32,
                        (mesh.vertices.len() + 1) as u32,
                        (mesh.vertices.len() + 2) as u32,
                    ];
                    push_vertex(&mut mesh, face[0], vertices, colors, texcoords, normals)?;
                    push_vertex(&mut mesh, face[1], vertices, colors, texcoords, normals)?;
                    push_vertex(&mut mesh, face[2], vertices, colors, texcoords, normals)?;
                    mesh.faces.push(vertices_indices);
                }
                Face::Polygon(face) => {
                    let a = face[0];
                    let mut b = face[1];
                    for &c in &face[2..] {
                        #[allow(clippy::cast_possible_truncation)]
                        let vertices_indices = [
                            mesh.vertices.len() as u32,
                            (mesh.vertices.len() + 1) as u32,
                            (mesh.vertices.len() + 2) as u32,
                        ];
                        push_vertex(&mut mesh, a, vertices, colors, texcoords, normals)?;
                        push_vertex(&mut mesh, b, vertices, colors, texcoords, normals)?;
                        push_vertex(&mut mesh, c, vertices, colors, texcoords, normals)?;
                        mesh.faces.push(vertices_indices);
                        b = c;
                    }
                }
            }
        }
        if !mesh.colors[0].is_empty() && mesh.vertices.len() != mesh.colors[0].len() {
            // TODO: do not use (0)
            return Err(ErrorKind::InvalidFaceIndex(0));
        }
        if !mesh.texcoords[0].is_empty() && mesh.vertices.len() != mesh.texcoords[0].len() {
            return Err(ErrorKind::InvalidFaceIndex(0));
        }
        if !mesh.normals.is_empty() && mesh.vertices.len() != mesh.normals.len() {
            return Err(ErrorKind::InvalidFaceIndex(0));
        }
        meshes.push(mesh);
        faces.clear();
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// MTL

// Not public API. (Used for fuzzing.)
#[doc(hidden)]
#[allow(clippy::implicit_hasher)] // false positive: doc(hidden) function should be treated as private
pub fn read_mtl(
    bytes: &[u8],
    path: Option<&Path>,
    materials: &mut Vec<common::Material>,
    material_map: &mut HashMap<Vec<u8>, u32>,
) -> io::Result<()> {
    let bytes = &decode_bytes(bytes)?;
    match read_mtl_internal(bytes, path.and_then(Path::parent), materials, material_map) {
        Ok(()) => Ok(()),
        Err(e) => Err(e.into_io_error(bytes, path)),
    }
}

fn read_mtl_internal(
    mut s: &[u8],
    mtl_dir: Option<&Path>,
    materials: &mut Vec<common::Material>,
    material_map: &mut HashMap<Vec<u8>, u32>,
) -> Result<(), ErrorKind> {
    let mut mat: Option<Material<'_>> = None;
    let mut current_name: &[u8] = b"";

    while let Some((&c, s_next)) = s.split_first() {
        match c {
            b'K' | b'k' => {
                s = s_next;
                match s.first() {
                    Some(b'a') => {
                        s = &s[1..];
                        if skip_spaces(&mut s) {
                            let color = read_color(&mut s, "Ka")?;
                            if let Some(mat) = &mut mat {
                                mat.ambient = Some(color);
                            }
                            continue;
                        }
                    }
                    Some(b'd') => {
                        s = &s[1..];
                        if skip_spaces(&mut s) {
                            let color = read_color(&mut s, "Kd")?;
                            if let Some(mat) = &mut mat {
                                mat.diffuse = Some(color);
                            }
                            continue;
                        }
                    }
                    Some(b's') => {
                        s = &s[1..];
                        if skip_spaces(&mut s) {
                            let color = read_color(&mut s, "Ks")?;
                            if let Some(mat) = &mut mat {
                                mat.specular = Some(color);
                            }
                            continue;
                        }
                    }
                    Some(b'e') => {
                        s = &s[1..];
                        if skip_spaces(&mut s) {
                            let color = read_color(&mut s, "Ke")?;
                            if let Some(mat) = &mut mat {
                                mat.emissive = Some(color);
                            }
                            continue;
                        }
                    }
                    _ => {}
                }
            }
            b'T' => {
                s = s_next;
                match s.first() {
                    Some(b'f') => {
                        s = &s[1..];
                        if skip_spaces(&mut s) {
                            let color = read_color(&mut s, "Tf")?;
                            if let Some(mat) = &mut mat {
                                mat.transparent = Some(color);
                            }
                            continue;
                        }
                    }
                    Some(b'r') => {
                        s = &s[1..];
                        if skip_spaces(&mut s) {
                            let f = read_float1(&mut s, "Tr")?;
                            if let Some(mat) = &mut mat {
                                mat.alpha = Some(1. - f);
                            }
                            continue;
                        }
                    }
                    _ => {}
                }
            }
            b'd' => {
                match s.get(1) {
                    Some(b' ' | b'\t') => {
                        s = &s[2..];
                        skip_spaces(&mut s);
                        let f = read_float1(&mut s, "d")?;
                        if let Some(mat) = &mut mat {
                            mat.alpha = Some(f);
                        }
                        continue;
                    }
                    Some(b'i') => {
                        if read_texture(&mut s, &mut mat) {
                            // disp
                            continue;
                        }
                    }
                    _ => {}
                }
                s = s_next;
            }
            b'N' | b'n' => match s.get(1) {
                Some(b's') => {
                    s = &s[2..];
                    if skip_spaces(&mut s) {
                        let f = read_float1(&mut s, "Ns")?;
                        if let Some(mat) = &mut mat {
                            mat.shininess = Some(f);
                        }
                        continue;
                    }
                }
                Some(b'i') => {
                    s = &s[2..];
                    if skip_spaces(&mut s) {
                        let f = read_float1(&mut s, "Ni")?;
                        if let Some(mat) = &mut mat {
                            mat.index_of_refraction = Some(f);
                        }
                        continue;
                    }
                }
                Some(b'e') => {
                    s = &s[2..];
                    if token(&mut s, &b"newmtl"[2..]) {
                        if skip_spaces(&mut s) {
                            let (name, s_next) = name(s);
                            if let Some(mat) = mat.replace(Material::default()) {
                                push_material(materials, material_map, mtl_dir, current_name, &mat);
                            }
                            current_name = name;
                            s = s_next;
                            continue;
                        }
                    }
                }
                Some(b'o') => {
                    if read_texture(&mut s, &mut mat) {
                        // norm
                        continue;
                    }
                }
                _ => {}
            },
            b'P' => {
                s = s_next;
                match s.first() {
                    Some(b'r') => {
                        s = &s[1..];
                        if skip_spaces(&mut s) {
                            let f = read_float1(&mut s, "Pr")?;
                            if let Some(mat) = &mut mat {
                                mat.roughness = Some(f);
                            }
                            continue;
                        }
                    }
                    Some(b'm') => {
                        s = &s[1..];
                        if skip_spaces(&mut s) {
                            let f = read_float1(&mut s, "Pm")?;
                            if let Some(mat) = &mut mat {
                                mat.metallic = Some(f);
                            }
                            continue;
                        }
                    }
                    Some(b's') => {
                        s = &s[1..];
                        if skip_spaces(&mut s) {
                            let color = read_color(&mut s, "Ps")?;
                            if let Some(mat) = &mut mat {
                                mat.sheen = Some(color);
                            }
                            continue;
                        }
                    }
                    Some(b'c') => {
                        s = &s[1..];
                        if s.first() == Some(&b'r') {
                            if skip_spaces(&mut s) {
                                let f = read_float1(&mut s, "Pcr")?;
                                if let Some(mat) = &mut mat {
                                    mat.clearcoat_roughness = Some(f);
                                }
                                continue;
                            }
                        } else if skip_spaces(&mut s) {
                            let f = read_float1(&mut s, "Pc")?;
                            if let Some(mat) = &mut mat {
                                mat.clearcoat_thickness = Some(f);
                            }
                            continue;
                        }
                    }
                    _ => {}
                }
            }
            b'm' | b'b' | b'r' => {
                if read_texture(&mut s, &mut mat) {
                    continue;
                }
            }
            b'i' => {
                s = s_next;
                if token(&mut s, &b"illum"[1..]) {
                    if skip_spaces(&mut s) {
                        match int::parse_partial::<u8>(s) {
                            Some((i, n)) => {
                                s = &s[n..];
                                if !skip_spaces_until_line(&mut s) {
                                    return Err(ErrorKind::ExpectedNewline("illum", s.len()));
                                }
                                if let Some(mat) = &mut mat {
                                    mat.illumination_model = Some(i);
                                }
                            }
                            None => return Err(ErrorKind::Int(s.len())),
                        }
                        continue;
                    }
                }
            }
            b'a' => {
                s = s_next;
                if skip_spaces(&mut s) {
                    let f = read_float1(&mut s, "a")?;
                    if let Some(mat) = &mut mat {
                        mat.anisotropy = Some(f);
                    }
                    continue;
                }
            }
            _ => {}
        }
        // ignore comment or other unknown
        skip_any_until_line(&mut s);
    }

    if let Some(mat) = &mat {
        push_material(materials, material_map, mtl_dir, current_name, mat);
    }

    Ok(())
}

fn read_texture<'a>(s: &mut &'a [u8], mat: &mut Option<Material<'a>>) -> bool {
    // Empty name cases are processed later in texture_path.
    // TODO: handle texture options
    if token(s, b"map_Kd") {
        if skip_spaces(s) {
            let (name, s_next) = name(s);
            if let Some(mat) = mat {
                mat.diffuse_texture = Some(name);
            }
            *s = s_next;
            return true;
        }
    } else if token(s, b"map_Ka") {
        if skip_spaces(s) {
            let (name, s_next) = name(s);
            if let Some(mat) = mat {
                mat.ambient_texture = Some(name);
            }
            *s = s_next;
            return true;
        }
    } else if token(s, b"map_Ks") {
        if skip_spaces(s) {
            let (name, s_next) = name(s);
            if let Some(mat) = mat {
                mat.specular_texture = Some(name);
            }
            *s = s_next;
            return true;
        }
    } else if token(s, b"map_disp") || token(s, b"disp") {
        if skip_spaces(s) {
            let (name, s_next) = name(s);
            if let Some(mat) = mat {
                mat.displacement_texture = Some(name);
            }
            *s = s_next;
            return true;
        }
    } else if token(s, b"map_d") {
        if skip_spaces(s) {
            let (name, s_next) = name(s);
            if let Some(mat) = mat {
                mat.opacity_texture = Some(name);
            }
            *s = s_next;
            return true;
        }
    } else if token(s, b"map_emissive") || token(s, b"map_Ke") {
        if skip_spaces(s) {
            let (name, s_next) = name(s);
            if let Some(mat) = mat {
                mat.emissive_texture = Some(name);
            }
            *s = s_next;
            return true;
        }
    } else if token(s, b"map_Bump") || token(s, b"map_bump") || token(s, b"bump") {
        if skip_spaces(s) {
            let (name, s_next) = name(s);
            if let Some(mat) = mat {
                mat.bump_texture = Some(name);
            }
            *s = s_next;
            return true;
        }
    } else if token(s, b"map_Kn") || token(s, b"norm") {
        if skip_spaces(s) {
            let (name, s_next) = name(s);
            if let Some(mat) = mat {
                mat.normal_texture = Some(name);
            }
            *s = s_next;
            return true;
        }
    } else if token(s, b"refl") {
        if skip_spaces(s) {
            let (_name, s_next) = name(s);
            // ignore https://github.com/assimp/assimp/blob/v5.3.1/code/AssetLib/Obj/ObjFileMtlImporter.cpp#L415
            *s = s_next;
            return true;
        }
    } else if token(s, b"map_Ns") || token(s, b"map_ns") || token(s, b"map_NS") {
        if skip_spaces(s) {
            let (name, s_next) = name(s);
            if let Some(mat) = mat {
                mat.specularity_texture = Some(name);
            }
            *s = s_next;
            return true;
        }
    } else if token(s, b"map_Pr") {
        if skip_spaces(s) {
            let (name, s_next) = name(s);
            if let Some(mat) = mat {
                mat.roughness_texture = Some(name);
            }
            *s = s_next;
            return true;
        }
    } else if token(s, b"map_Pm") {
        if skip_spaces(s) {
            let (name, s_next) = name(s);
            if let Some(mat) = mat {
                mat.metallic_texture = Some(name);
            }
            *s = s_next;
            return true;
        }
    } else if token(s, b"map_Ps") {
        if skip_spaces(s) {
            let (name, s_next) = name(s);
            if let Some(mat) = mat {
                mat.sheen_texture = Some(name);
            }
            *s = s_next;
            return true;
        }
    }
    false
}

fn push_material(
    materials: &mut Vec<common::Material>,
    material_map: &mut HashMap<Vec<u8>, u32>,
    mtl_dir: Option<&Path>,
    current_name: &[u8],
    mat: &Material<'_>,
) {
    fn color4(color3: Option<[f32; 3]>) -> Option<Color4> {
        let rgb = color3?;
        // a is 1 by default: https://github.com/assimp/assimp/blob/v5.3.1/code/AssetLib/Obj/ObjFileImporter.cpp#L233
        Some([rgb[0], rgb[1], rgb[2], 1.])
    }
    fn texture_path(texture: Option<&[u8]>, mtl_dir: Option<&Path>) -> Option<PathBuf> {
        let mut p = texture?;
        if p.is_empty() {
            return None;
        }
        match mtl_dir {
            Some(mtl_dir) => {
                let tmp: Vec<_>;
                if p.contains(&b'\\') {
                    tmp = p
                        .iter()
                        .map(|&b| if b == b'\\' { b'/' } else { b })
                        .collect();
                    p = &*tmp;
                }
                if p.starts_with(b"/..") {
                    p = p.strip_prefix(b"/").unwrap_or(p);
                }
                p = p.strip_prefix(b"./").unwrap_or(p);
                let p = path_from_bytes(p).ok()?;
                let p = mtl_dir.join(p);
                if p.to_str().map_or(false, |s| {
                    s.starts_with("https://") || p.starts_with("http://")
                }) || p.exists()
                {
                    Some(p)
                } else {
                    None
                }
            }
            None => {
                let p = path_from_bytes(p).ok()?.to_owned();
                Some(p)
            }
        }
    }
    #[allow(clippy::cast_possible_truncation)]
    let material_index = materials.len() as u32;
    materials.push(common::Material {
        name: from_utf8_lossy(current_name).into_owned(),
        // Refs: https://github.com/assimp/assimp/blob/v5.3.1/code/AssetLib/Obj/ObjFileImporter.cpp#L591
        shading_model: match mat.illumination_model {
            Some(0) => Some(ShadingModel::NoShading),
            Some(1) => Some(ShadingModel::Gouraud),
            Some(2) => Some(ShadingModel::Phong),
            _ => None,
        },
        shininess: mat.shininess,
        opacity: mat.alpha,
        reflectivity: None,
        index_of_refraction: mat.index_of_refraction,
        // roughness_factor: mat.roughness,
        // metallic_factor: mat.metallic,
        // sheen_color_factor: mat.sheen,
        // clearcoat_factor: mat.clearcoat_thickness,
        // clearcoat_roughness_factor: mat.clearcoat_roughness,
        // anisotropy_factor: mat.anisotropy,
        color: common::Colors {
            ambient: color4(mat.ambient),
            diffuse: color4(mat.diffuse),
            specular: color4(mat.specular),
            emissive: color4(mat.emissive),
            transparent: color4(mat.transparent),
            reflective: None,
        },
        texture: common::Textures {
            diffuse: texture_path(mat.diffuse_texture, mtl_dir),
            ambient: texture_path(mat.ambient_texture, mtl_dir),
            emissive: texture_path(mat.emissive_texture, mtl_dir),
            specular: texture_path(mat.specular_texture, mtl_dir),
            height: texture_path(mat.bump_texture, mtl_dir),
            normal: texture_path(mat.normal_texture, mtl_dir),
            reflection: None, // TODO
            displacement: texture_path(mat.displacement_texture, mtl_dir),
            opacity: texture_path(mat.opacity_texture, mtl_dir),
            shininess: texture_path(mat.specularity_texture, mtl_dir),
            lightmap: None,
        },
    });
    material_map.insert(current_name.to_owned(), material_index);
}

// -----------------------------------------------------------------------------
// Helpers

enum Face {
    Point(#[allow(dead_code)] [[u32; 3]; 1]),
    Line(#[allow(dead_code)] [[u32; 3]; 2]),
    Triangle([[u32; 3]; 3]),
    Polygon(Vec<[u32; 3]>),
}

#[derive(Default)]
struct Material<'a> {
    // Textures
    diffuse_texture: Option<&'a [u8]>,
    specular_texture: Option<&'a [u8]>,
    ambient_texture: Option<&'a [u8]>,
    emissive_texture: Option<&'a [u8]>,
    bump_texture: Option<&'a [u8]>,
    normal_texture: Option<&'a [u8]>,
    // reflection_texture: Option<&'a [u8]>,
    specularity_texture: Option<&'a [u8]>,
    opacity_texture: Option<&'a [u8]>,
    displacement_texture: Option<&'a [u8]>,
    roughness_texture: Option<&'a [u8]>,
    metallic_texture: Option<&'a [u8]>,
    sheen_texture: Option<&'a [u8]>,
    // rma_texture: Option<&'a [u8]>,

    // Colors
    ambient: Option<[f32; 3]>,
    diffuse: Option<[f32; 3]>,
    specular: Option<[f32; 3]>,
    emissive: Option<[f32; 3]>,
    alpha: Option<f32>,
    shininess: Option<f32>,
    illumination_model: Option<u8>,
    index_of_refraction: Option<f32>,
    transparent: Option<[f32; 3]>,

    roughness: Option<f32>,
    metallic: Option<f32>,
    sheen: Option<[f32; 3]>,
    clearcoat_thickness: Option<f32>,
    clearcoat_roughness: Option<f32>,
    anisotropy: Option<f32>,
    // bump_multiplier: Option<f32>,
}

// [\r\n]
const LINE: u8 = 1 << 0;
// [ \t]
const SPACE: u8 = 1 << 1;
// [ \r\n\t]
const WHITESPACE: u8 = 1 << 2;

static TABLE: [u8; 256] = {
    const __: u8 = 0;
    const LN: u8 = WHITESPACE | LINE;
    const NL: u8 = WHITESPACE | SPACE;
    [
        //  _1  _2  _3  _4  _5  _6  _7  _8  _9  _A  _B  _C  _D  _E  _F
        __, __, __, __, __, __, __, __, __, NL, LN, __, __, LN, __, __, // 0_
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 1_
        NL, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 2_
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 3_
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 4_
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 5_
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 6_
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 7_
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 8_
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 9_
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // A_
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // B_
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // C_
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // D_
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // E_
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // F_
    ]
};
#[test]
fn table() {
    for b in u8::MIN..=u8::MAX {
        match b {
            b' ' | b'\t' => {
                assert_eq!(
                    TABLE[b as usize],
                    WHITESPACE | SPACE,
                    "{:?}({b:#X})",
                    b as char
                );
            }
            b'\n' | b'\r' => {
                assert_eq!(
                    TABLE[b as usize],
                    WHITESPACE | LINE,
                    "{:?}({b:#X})",
                    b as char
                );
            }
            _ => assert_eq!(TABLE[b as usize], 0, "{:?}({b:#X})", b as char),
        }
    }
}

#[inline]
fn skip_whitespace_until_byte_or_eof(s: &mut &[u8], byte_mask: u8, whitespace_mask: u8) -> bool {
    while let Some((&b, s_next)) = s.split_first() {
        let t = TABLE[b as usize];
        if t & byte_mask != 0 {
            *s = s_next;
            break;
        }
        if t & whitespace_mask != 0 {
            *s = s_next;
            continue;
        }
        if b == b'\\' && matches!(s_next.first(), Some(b'\n' | b'\r')) {
            if s_next.starts_with(b"\r\n") {
                *s = &s_next[2..];
            } else {
                *s = &s_next[1..];
            }
            continue;
        }
        return false;
    }
    true
}

#[inline]
fn skip_spaces_until_line(s: &mut &[u8]) -> bool {
    skip_whitespace_until_byte_or_eof(s, LINE, SPACE)
}

/// Skips spaces or tabs, and returns `true` if one or more spaces or tabs are
/// present. (not consumes non-{space,tab} characters.
#[inline]
fn skip_spaces(s: &mut &[u8]) -> bool {
    let start = *s;
    while let Some((&b, s_next)) = s.split_first() {
        if TABLE[b as usize] & SPACE != 0 {
            *s = s_next;
            continue;
        }
        if b == b'\\' && matches!(s_next.first(), Some(b'\n' | b'\r')) {
            if s_next.starts_with(b"\r\n") {
                *s = &s_next[2..];
            } else {
                *s = &s_next[1..];
            }
            continue;
        }
        break;
    }
    start.len() != s.len()
}

/// Skips non-line (non-`[\r\n]`) characters. (consumes line character).
#[inline]
fn skip_any_until_line(s: &mut &[u8]) {
    while let Some((&b, s_next)) = s.split_first() {
        if TABLE[b as usize] & LINE != 0 {
            *s = s_next;
            break;
        }
        if b == b'\\' && matches!(s_next.first(), Some(b'\n' | b'\r')) {
            if s_next.starts_with(b"\r\n") {
                *s = &s_next[2..];
            } else {
                *s = &s_next[1..];
            }
            continue;
        }
        *s = s_next;
        continue;
    }
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

fn name(mut s: &[u8]) -> (&[u8], &[u8]) {
    let start = s;
    skip_any_until_line(&mut s);
    let mut name = &start[..start.len() - s.len()];
    // Allow spaces in middle, trim end
    // https://github.com/assimp/assimp/commit/c84a14a7a8ae4329114269a0ffc1921c838eda9e
    while let Some((&b, name_next)) = name.split_last() {
        if TABLE[b as usize] & WHITESPACE != 0 {
            name = name_next;
            continue;
        }
        break;
    }
    (name, s)
}
