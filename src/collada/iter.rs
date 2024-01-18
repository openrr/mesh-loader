use std::{
    iter::{self, FusedIterator},
    ops::Range,
    slice,
};

use crate::{collada as ast, Vec3};

impl ast::Document {
    pub(super) fn meshes(&self) -> Meshes<'_> {
        Meshes {
            iter: self.library_geometries.geometries.values().enumerate(),
            doc: self,
        }
    }
}

pub(super) struct Meshes<'a> {
    pub(super) iter:
        iter::Enumerate<std::collections::btree_map::Values<'a, String, ast::Geometry>>,
    pub(super) doc: &'a ast::Document,
}

impl<'a> Iterator for Meshes<'a> {
    type Item = Mesh<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(_index, xml)| Mesh { doc: self.doc, xml })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl ExactSizeIterator for Meshes<'_> {}

impl FusedIterator for Meshes<'_> {}

#[derive(Clone)]
pub(super) struct Mesh<'a> {
    pub(super) doc: &'a ast::Document,
    pub(super) xml: &'a ast::Geometry,
}

impl<'a> Mesh<'a> {
    pub(super) fn primitives(&self) -> Primitives<'a> {
        Primitives {
            mesh: self.clone(),
            iter: self.xml.mesh.primitives.iter().enumerate(),
        }
    }
}

pub(super) struct Primitives<'a> {
    mesh: Mesh<'a>,
    iter: iter::Enumerate<slice::Iter<'a, ast::Primitive>>,
}

impl<'a> Iterator for Primitives<'a> {
    type Item = Primitive<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_index, xml)| Primitive {
            mesh: self.mesh.clone(),
            xml,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl ExactSizeIterator for Primitives<'_> {}

impl FusedIterator for Primitives<'_> {}

#[derive(Clone)]
pub(super) struct Primitive<'a> {
    pub(super) mesh: Mesh<'a>,
    pub(super) xml: &'a ast::Primitive,
}

impl<'a> Primitive<'a> {
    pub(super) fn positions(&self) -> Positions<'a> {
        let input = match &self.xml.input {
            Some(input) => input,
            None => return Positions(None),
        };
        if self.mesh.xml.mesh.vertices.id == input.vertex.source {
            let position = &self.mesh.xml.mesh.vertices.input.position;
            let acc = &self.mesh.doc[&position.source];
            let data = self.mesh.doc[&acc.source].as_float().unwrap();
            // TODO: check param names are ["X", "Y", "Z"] and type is "float"
            assert!(acc.stride >= 3);
            assert!((acc.count * acc.stride) as usize <= data.len());
            Positions(Some((acc.count, data.chunks(acc.stride as _))))
        } else {
            // TODO: search other mesh's vertices
            todo!()
        }
    }

    pub(super) fn normals(&self) -> Normals<'a> {
        let acc = match &self.xml.input {
            Some(input) => match &input.normal {
                Some(normal) => &self.mesh.doc[&normal.source],
                None => {
                    if self.mesh.xml.mesh.vertices.id == input.vertex.source {
                        match &self.mesh.xml.mesh.vertices.input.normal {
                            Some(normal) => &self.mesh.doc[&normal.source],
                            None => return Positions(None),
                        }
                    } else {
                        // TODO: search other mesh's vertices
                        todo!()
                    }
                }
            },
            None => return Positions(None),
        };
        let data = self.mesh.doc[&acc.source].as_float().unwrap();
        // TODO: check param names are ["X", "Y", "Z"] and type is "float"
        assert!(acc.stride >= 3);
        assert!((acc.count * acc.stride) as usize <= data.len());
        Positions(Some((acc.count, data.chunks(acc.stride as _))))
    }

    pub(super) fn texcoords(&self, set: usize) -> Texcoords<'a> {
        let acc = match &self.xml.input {
            Some(input) => {
                if let Some(texcoord) = input.texcoord.get(set) {
                    &self.mesh.doc[&texcoord.source]
                } else if set == 0 {
                    if self.mesh.xml.mesh.vertices.id == input.vertex.source {
                        match &self.mesh.xml.mesh.vertices.input.texcoord {
                            Some(texcoord) => &self.mesh.doc[&texcoord.source],
                            None => return Texcoords(None),
                        }
                    } else {
                        // TODO: search other mesh's vertices
                        todo!()
                    }
                } else {
                    return Texcoords(None);
                }
            }
            None => return Texcoords(None),
        };
        let data = self.mesh.doc[&acc.source].as_float().unwrap();
        // TODO: check param names are ["S", "T"] and type is "float"
        assert!(acc.stride >= 2);
        assert!((acc.count * acc.stride) as usize <= data.len());
        Texcoords(Some(TexcoordsInner {
            iter: data.chunks(acc.stride as _),
        }))
    }

    fn vertex_indices_inner(&self, offset: u32) -> IndicesInner<'a> {
        match self.xml.ty {
            ast::PrimitiveType::Polylist | ast::PrimitiveType::Polygons => IndicesInner::Polylist {
                offset,
                indices: &self.xml.p,
                stride: self.xml.stride,
                index: 0,
                vcount: self.xml.vcount.iter(),
                range: None,
            },
            ast::PrimitiveType::Triangles => IndicesInner::Triangles {
                offset,
                indices: self.xml.p.chunks(self.xml.stride as _),
            },
            ast::PrimitiveType::TriStrips | ast::PrimitiveType::TriFans => {
                IndicesInner::TriStrips {
                    offset,
                    indices: &self.xml.p,
                    stride: self.xml.stride,
                    index: 0,
                    vcount: self.xml.vcount.iter(),
                    range: None,
                }
            }
            ast::PrimitiveType::Lines => IndicesInner::Lines {
                offset,
                indices: self.xml.p.chunks(self.xml.stride as _),
            },
            ast::PrimitiveType::LineStrips => IndicesInner::LineStrips {
                offset,
                indices: &self.xml.p,
                stride: self.xml.stride,
                index: 0,
                vcount: self.xml.vcount.iter(),
                range: None,
            },
        }
    }

    fn vertex_indices_size(&self, min_face_size: u32) -> u32 {
        debug_assert!((1..=3).contains(&min_face_size));
        match self.xml.ty {
            ast::PrimitiveType::Polylist | ast::PrimitiveType::Polygons => self
                .xml
                .vcount
                .iter()
                .map(|count| {
                    if (min_face_size..=3).contains(count) {
                        1
                    } else if *count > 3 {
                        count - 2
                    } else {
                        0
                    }
                })
                .sum(),
            ast::PrimitiveType::Triangles => self.xml.count,
            ast::PrimitiveType::TriStrips | ast::PrimitiveType::TriFans => {
                self.xml.vcount.iter().map(|&count| count - 2).sum()
            }
            ast::PrimitiveType::Lines => {
                if min_face_size <= 2 {
                    self.xml.count
                } else {
                    0
                }
            }
            ast::PrimitiveType::LineStrips => {
                if min_face_size <= 2 {
                    self.xml.vcount.iter().map(|&count| count - 1).sum()
                } else {
                    0
                }
            }
        }
    }

    pub(super) fn vertex_indices(&self) -> VertexIndices<'a> {
        let offset = match &self.xml.input {
            Some(input) => input.vertex.offset,
            None => return VertexIndices::none(),
        };
        VertexIndices {
            remaining: self.vertex_indices_size(1),
            inner: self.vertex_indices_inner(offset),
        }
    }

    pub(super) fn normal_indices(&self) -> VertexIndices<'a> {
        let offset = match &self.xml.input {
            Some(input) => match &input.normal {
                Some(normal) => normal.offset,
                None => {
                    if self.mesh.xml.mesh.vertices.id == input.vertex.source {
                        if self.mesh.xml.mesh.vertices.input.normal.is_some() {
                            input.vertex.offset
                        } else {
                            return VertexIndices::none();
                        }
                    } else {
                        // TODO: search other mesh's vertices
                        todo!()
                    }
                }
            },
            None => return VertexIndices::none(),
        };
        VertexIndices {
            remaining: self.vertex_indices_size(1),
            inner: self.vertex_indices_inner(offset),
        }
    }

    pub(super) fn texcoord_indices(&self, set: usize) -> VertexIndices<'a> {
        let offset = match &self.xml.input {
            Some(input) => match input.texcoord.get(set) {
                Some(texcoord) => texcoord.offset,
                None => {
                    if self.mesh.xml.mesh.vertices.id == input.vertex.source {
                        if self.mesh.xml.mesh.vertices.input.texcoord.is_some() {
                            input.vertex.offset
                        } else {
                            return VertexIndices::none();
                        }
                    } else {
                        // TODO: search other mesh's vertices
                        todo!()
                    }
                }
            },
            None => return VertexIndices::none(),
        };
        VertexIndices {
            remaining: self.vertex_indices_size(1),
            inner: self.vertex_indices_inner(offset),
        }
    }
}

pub(super) struct Positions<'a>(Option<(u32, slice::Chunks<'a, f32>)>);

impl Iterator for Positions<'_> {
    type Item = Vec3;

    fn next(&mut self) -> Option<Self::Item> {
        let (count, iter) = self.0.as_mut()?;
        let v = iter.next().unwrap();
        *count -= 1;
        if *count == 0 {
            self.0 = None;
        }
        Some([v[0], v[1], v[2]])
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            Some((_, iter)) => iter.size_hint(),
            None => (0, Some(0)),
        }
    }
}

impl ExactSizeIterator for Positions<'_> {}

impl FusedIterator for Positions<'_> {}

pub(super) type Normals<'a> = Positions<'a>;

pub(super) struct Texcoords<'a>(Option<TexcoordsInner<'a>>);

struct TexcoordsInner<'a> {
    iter: slice::Chunks<'a, f32>,
}

impl Iterator for Texcoords<'_> {
    type Item = [f32; 2];

    fn next(&mut self) -> Option<Self::Item> {
        let inner = self.0.as_mut()?;
        match inner.iter.next() {
            Some(v) => Some([v[0], v[1]]),
            None => {
                self.0 = None;
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            Some(inner) => inner.iter.size_hint(),
            None => (0, Some(0)),
        }
    }
}

impl ExactSizeIterator for Texcoords<'_> {}

impl FusedIterator for Texcoords<'_> {}

pub(super) struct VertexIndices<'a> {
    remaining: u32,
    inner: IndicesInner<'a>,
}

enum IndicesInner<'a> {
    Polylist {
        offset: u32,
        indices: &'a [u32],
        stride: u32,
        vcount: slice::Iter<'a, u32>,
        index: usize,
        range: Option<Range<u32>>,
    },
    Triangles {
        offset: u32,
        indices: slice::Chunks<'a, u32>,
    },
    TriStrips {
        offset: u32,
        indices: &'a [u32],
        stride: u32,
        vcount: slice::Iter<'a, u32>,
        index: usize,
        range: Option<Range<u32>>,
    },
    Lines {
        offset: u32,
        indices: slice::Chunks<'a, u32>,
    },
    LineStrips {
        offset: u32,
        indices: &'a [u32],
        stride: u32,
        vcount: slice::Iter<'a, u32>,
        index: usize,
        range: Option<Range<u32>>,
    },
    None,
}

impl VertexIndices<'_> {
    const fn none() -> Self {
        Self {
            remaining: 0,
            inner: IndicesInner::None,
        }
    }
}

impl Iterator for VertexIndices<'_> {
    type Item = Face;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;

        match &mut self.inner {
            IndicesInner::Polylist {
                offset,
                indices,
                stride,
                index,
                vcount,
                range,
            }
            | IndicesInner::TriStrips {
                offset,
                indices,
                stride,
                index,
                vcount,
                range,
            } => {
                let offset = *offset as usize;
                let stride = *stride as usize;
                if let Some(r) = range {
                    if let Some(k) = r.next() {
                        let x = *index + offset;
                        let y = *index + offset + stride * k as usize;
                        let z = *index + offset + stride * (k as usize + 1);
                        let value = Face::Triangle([indices[x], indices[y], indices[z]]);
                        // NOTE: Do *not* increment index until range ends.
                        return Some(value);
                    }
                    let vc = r.end + 1;
                    *index += stride * vc as usize;
                    *range = None;
                }
                let vc = *vcount.next()?;
                match vc {
                    3 => {
                        let x = *index + offset;
                        let y = *index + offset + stride;
                        let z = *index + offset + stride * 2;
                        let value = Face::Triangle([indices[x], indices[y], indices[z]]);
                        *index += stride * vc as usize;
                        Some(value)
                    }
                    2 => {
                        let x = *index + offset;
                        let y = *index + offset + stride;
                        let value = Face::Line([indices[x], indices[y]]);
                        *index += stride * vc as usize;
                        Some(value)
                    }
                    1 => {
                        let x = *index + offset;
                        let value = Face::Point([indices[x]]);
                        *index += stride * vc as usize;
                        Some(value)
                    }
                    0 => unreachable!(),
                    _ => {
                        let mut ri = 1..vc - 1;
                        let k = ri.next().unwrap();
                        let x = *index + offset;
                        let y = *index + offset + stride * k as usize;
                        let z = *index + offset + stride * (k as usize + 1);
                        let value = Face::Triangle([indices[x], indices[y], indices[z]]);
                        // Set range for next call.
                        // NOTE: Do *not* increment index until range ends.
                        *range = Some(ri);
                        Some(value)
                    }
                }
            }
            IndicesInner::Triangles { offset, indices } => {
                let indices1 = indices.next().unwrap();
                let indices2 = indices.next().unwrap();
                let indices3 = indices.next().unwrap();
                Some(Face::Triangle([
                    indices1[*offset as usize],
                    indices2[*offset as usize],
                    indices3[*offset as usize],
                ]))
            }
            IndicesInner::Lines { offset, indices } => {
                let indices1 = indices.next().unwrap();
                let indices2 = indices.next().unwrap();
                Some(Face::Line([
                    indices1[*offset as usize],
                    indices2[*offset as usize],
                ]))
            }
            IndicesInner::LineStrips {
                offset,
                indices,
                stride,
                index,
                vcount,
                range,
            } => {
                let offset = *offset as usize;
                let stride = *stride as usize;
                if let Some(r) = range {
                    if let Some(k) = r.next() {
                        let x = *index + offset;
                        let y = *index + offset + stride * k as usize;
                        let value = Face::Line([indices[x], indices[y]]);
                        // NOTE: Do *not* increment index until range ends.
                        return Some(value);
                    }
                    let vc = r.end;
                    *index += stride * vc as usize;
                    *range = None;
                }
                let vc = *vcount.next()?;
                match vc {
                    2 => {
                        let x = *index + offset;
                        let y = *index + offset + stride;
                        let value = Face::Line([indices[x], indices[y]]);
                        *index += stride * vc as usize;
                        Some(value)
                    }
                    0..=2 => unreachable!(),
                    _ => {
                        let mut r = 1..vc;
                        let k = r.next().unwrap();
                        let x = *index + offset;
                        let y = *index + offset + stride * k as usize;
                        let value = Face::Line([indices[x], indices[y]]);
                        // Set range for next call.
                        // NOTE: Do *not* increment index until range ends.
                        *range = Some(r);
                        Some(value)
                    }
                }
            }
            IndicesInner::None => unreachable!(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining as usize, Some(self.remaining as usize))
    }
}

impl ExactSizeIterator for VertexIndices<'_> {}

impl FusedIterator for VertexIndices<'_> {}

#[derive(Clone)]
pub(super) enum Face {
    Point(#[allow(dead_code)] [u32; 1]),
    Line(#[allow(dead_code)] [u32; 2]),
    Triangle([u32; 3]),
}
