use std::{
    iter::{self, FusedIterator},
    ops::Range,
    slice,
};

use crate::{collada as ast, Face, Vec2, Vec3};

#[derive(Clone)]
pub(super) struct Mesh<'a> {
    pub(super) doc: &'a ast::Document<'a>,
    pub(super) xml: &'a ast::Geometry<'a>,
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
    iter: iter::Enumerate<slice::Iter<'a, ast::Primitive<'a>>>,
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
    pub(super) xml: &'a ast::Primitive<'a>,
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
            // ["X", "Y", "Z"]
            if acc.stride < 3 || acc.params.len() < 3 || acc.params.iter().any(|p| p.ty != "float")
            {
                // TODO: error?
                return Positions(None);
            }
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
        // ["X", "Y", "Z"]
        if acc.stride < 3 || acc.params.len() < 3 || acc.params.iter().any(|p| p.ty != "float") {
            // TODO: error?
            return Positions(None);
        }
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
        // ["S", "T"] or ["S", "T", "P"]
        if acc.stride < 2 || acc.params.len() < 2 || acc.params.iter().any(|p| p.ty != "float") {
            // TODO: error?
            return Texcoords(None);
        }
        assert!((acc.count * acc.stride) as usize <= data.len());
        Texcoords(Some(TexcoordsInner {
            iter: data.chunks(acc.stride as _),
        }))
    }

    pub(super) fn colors(&self) -> Colors<'a> {
        let acc = match &self.xml.input {
            Some(input) => match &input.color {
                Some(color) => &self.mesh.doc[&color.source],
                None => {
                    if self.mesh.xml.mesh.vertices.id == input.vertex.source {
                        match &self.mesh.xml.mesh.vertices.input.color {
                            Some(color) => &self.mesh.doc[&color.source],
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
        // ["R", "G", "B"] or ["R", "G", "B", "A"]
        if acc.stride < 3 || acc.params.len() < 3 || acc.params.iter().any(|p| p.ty != "float") {
            // TODO: error?
            return Positions(None);
        }
        assert!((acc.count * acc.stride) as usize <= data.len());
        Positions(Some((acc.count, data.chunks(acc.stride as _))))
    }

    fn vertex_indices_inner(&self, offset: u32) -> IndicesInner<'a> {
        match self.xml.ty {
            ast::PrimitiveType::Lines | ast::PrimitiveType::LineStrips => IndicesInner::Skip,
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
        }
    }

    fn vertex_indices_size(&self) -> u32 {
        match self.xml.ty {
            ast::PrimitiveType::Polylist | ast::PrimitiveType::Polygons => self
                .xml
                .vcount
                .iter()
                .map(|count| if *count >= 3 { count - 2 } else { 0 })
                .sum(),
            ast::PrimitiveType::Triangles => self.xml.count,
            ast::PrimitiveType::TriStrips | ast::PrimitiveType::TriFans => {
                self.xml.vcount.iter().map(|&count| count - 2).sum()
            }
            ast::PrimitiveType::Lines => 0,
            ast::PrimitiveType::LineStrips => 0,
        }
    }

    pub(super) fn vertex_indices(&self) -> VertexIndices<'a> {
        let offset = match &self.xml.input {
            Some(input) => input.vertex.offset,
            None => return VertexIndices::none(),
        };
        VertexIndices {
            remaining: self.vertex_indices_size(),
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
            remaining: self.vertex_indices_size(),
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
            remaining: self.vertex_indices_size(),
            inner: self.vertex_indices_inner(offset),
        }
    }

    pub(super) fn color_indices(&self) -> VertexIndices<'a> {
        let offset = match &self.xml.input {
            Some(input) => match &input.color {
                Some(color) => color.offset,
                None => {
                    if self.mesh.xml.mesh.vertices.id == input.vertex.source {
                        if self.mesh.xml.mesh.vertices.input.color.is_some() {
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
            remaining: self.vertex_indices_size(),
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
pub(super) type Colors<'a> = Positions<'a>;

pub(super) struct Texcoords<'a>(Option<TexcoordsInner<'a>>);

struct TexcoordsInner<'a> {
    iter: slice::Chunks<'a, f32>,
}

impl Iterator for Texcoords<'_> {
    type Item = Vec2;

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
    Skip,
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
                        let value = [indices[x], indices[y], indices[z]];
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
                        let value = [indices[x], indices[y], indices[z]];
                        *index += stride * vc as usize;
                        Some(value)
                    }
                    1..=2 => self.next(),
                    0 => unreachable!(),
                    _ => {
                        let mut ri = 1..vc - 1;
                        let k = ri.next().unwrap();
                        let x = *index + offset;
                        let y = *index + offset + stride * k as usize;
                        let z = *index + offset + stride * (k as usize + 1);
                        let value = [indices[x], indices[y], indices[z]];
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
                Some([
                    indices1[*offset as usize],
                    indices2[*offset as usize],
                    indices3[*offset as usize],
                ])
            }
            IndicesInner::Skip => self.next(),
            IndicesInner::None => unreachable!(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining as usize, Some(self.remaining as usize))
    }
}

impl ExactSizeIterator for VertexIndices<'_> {}
impl FusedIterator for VertexIndices<'_> {}
