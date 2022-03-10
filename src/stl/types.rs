#![allow(missing_docs)] // TODO

use std::io;

use crate::fxhash::FxHashMap;

pub trait FromStl: Sized {
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
    fn reserve(cx: &mut Self::Context, num_triangles: u32) {
        let _ = (cx, num_triangles);
    }

    /// Sets the name.
    fn set_name<S>(cx: &mut Self::Context, name: S)
    where
        S: Into<String>,
    {
        let _ = (cx, name);
    }

    /*
    /// Sets the default vertex color.
    fn set_default_vertex_color(cx: &mut Self::Context, color: Color) {
        let _ = (cx, color);
    }

    /// Sets the material color.
    fn set_material_color(cx: &mut Self::Context, color: Color) {
        let _ = (cx, color);
    }
    */
}

pub type Vector3D = [f32; 3];

/*
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[non_exhaustive]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        debug_assert!((0.0..=1.0).contains(&r));
        debug_assert!((0.0..=1.0).contains(&g));
        debug_assert!((0.0..=1.0).contains(&b));
        debug_assert!((0.0..=1.0).contains(&a));
        Self { r, g, b, a: 1.0 }
    }

    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        debug_assert!((0.0..=1.0).contains(&r));
        debug_assert!((0.0..=1.0).contains(&g));
        debug_assert!((0.0..=1.0).contains(&b));
        Self { r, g, b, a }
    }
}

impl From<[f32; 3]> for Color {
    fn from(rgba: [f32; 3]) -> Self {
        Self::rgb(rgba[0], rgba[1], rgba[2])
    }
}

impl From<[f32; 4]> for Color {
    fn from(rgba: [f32; 4]) -> Self {
        Self::rgba(rgba[0], rgba[1], rgba[2], rgba[3])
    }
}

impl From<Color> for [f32; 3] {
    fn from(color: Color) -> Self {
        [color.r, color.g, color.b]
    }
}

impl From<Color> for [f32; 4] {
    fn from(color: Color) -> Self {
        [color.r, color.g, color.b, color.a]
    }
}
*/

#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct Mesh {
    pub name: String,
    pub triangles: Vec<Triangle>,
}

impl Mesh {
    /// Creates a new `Mesh`.
    pub fn new(triangles: Vec<Triangle>) -> Self {
        Self {
            name: String::new(),
            triangles,
        }
    }

    pub fn dedup(&self) -> IndexMesh {
        let mut cx = IndexMesh::start();
        IndexMesh::reserve(&mut cx, self.triangles.len() as _);
        for &triangle in &self.triangles {
            IndexMesh::push_triangle(&mut cx, triangle);
        }
        IndexMesh::set_name(&mut cx, &self.name);
        IndexMesh::end(cx)
    }
}

impl FromStl for Mesh {
    type Context = Self;

    fn start() -> Self::Context {
        Self::default()
    }

    fn end(mut cx: Self::Context) -> Self {
        cx.triangles.shrink_to_fit();
        cx
    }

    #[inline]
    fn push_triangle(cx: &mut Self::Context, triangle: Triangle) {
        cx.triangles.push(triangle);
    }

    fn reserve(cx: &mut Self::Context, num_triangles: u32) {
        // Use reserve_exact because binary stl has information on the exact number of triangles.
        cx.triangles.reserve_exact(num_triangles as _);
    }

    fn set_name<S>(cx: &mut Self::Context, name: S)
    where
        S: Into<String>,
    {
        cx.name = name.into();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Triangle {
    pub normal: Vector3D,
    pub vertices: [Vector3D; 3],
    /*
    pub color: Option<Color>,
    */
}

impl Triangle {
    /// Creates a new `Triangle`.
    #[inline]
    pub fn new(normal: Vector3D, vertices: [Vector3D; 3]) -> Self {
        Self { normal, vertices }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct IndexMesh {
    pub name: String,
    pub triangles: Vec<IndexTriangle>,
    pub vertices: Vec<Vector3D>,
    /*
    pub colors: Vec<Option<Color>>,
    pub material_color: Option<Color>,
    */
}

impl IndexMesh {
    /// Creates a new `IndexMesh`.
    pub fn new(triangles: Vec<IndexTriangle>, vertices: Vec<Vector3D>) -> Self {
        Self {
            name: String::new(),
            triangles,
            vertices,
        }
    }

    // pub fn from_reader_nodedup<R>(reader: R) -> io::Result<Self>
    // where
    //     R: Read + Seek,
    // {
    //     Ok(super::from_reader::<_, IndexMeshNodedup>(reader)?.mesh)
    // }

    // pub fn from_buf_reader_nodedup<R>(reader: R) -> io::Result<Self>
    // where
    //     R: BufRead + Seek,
    // {
    //     Ok(super::from_buf_reader::<_, IndexMeshNodedup>(reader)?.mesh)
    // }

    pub fn from_slice_nodedup(bytes: &[u8]) -> io::Result<Self> {
        Ok(super::from_slice::<IndexMeshNodedup>(bytes)?.mesh)
    }
}

// Not public API.
#[doc(hidden)]
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct IndexMeshReadContext {
    mesh: IndexMesh,
    vertices_to_indices: FxHashMap<[u32; 3], usize>,
    vertices_indices: [usize; 3],
    /*
    default_vertex_color: Option<Color>,
    */
}

impl FromStl for IndexMesh {
    type Context = IndexMeshReadContext;

    fn start() -> Self::Context {
        IndexMeshReadContext::default()
    }

    fn end(mut cx: Self::Context) -> Self {
        cx.mesh.vertices.shrink_to_fit();
        cx.mesh.triangles.shrink_to_fit();
        /*
        cx.mesh.colors.shrink_to_fit();
        */
        cx.mesh
    }

    #[inline]
    fn push_triangle(cx: &mut Self::Context, triangle: Triangle) {
        /*
        let mut has_color = !cx.mesh.colors.is_empty();
        if triangle.color.is_some() && !has_color {
            has_color = true;
            cx.mesh.colors.reserve_exact(cx.mesh.vertices.capacity());
            while cx.mesh.vertices.len() > cx.mesh.colors.len() {
                cx.mesh.colors.push(cx.default_vertex_color);
            }
        }
        */

        for (i, vertex) in triangle.vertices.iter().enumerate() {
            let bits = [
                vertex[0].to_bits(),
                vertex[1].to_bits(),
                vertex[2].to_bits(),
            ];

            if let Some(&index) = cx.vertices_to_indices.get(&bits) {
                cx.vertices_indices[i] = index;
                /*
                if has_color {
                    cx.mesh.colors[index] = triangle.color;
                }
                */
            } else {
                let index = cx.mesh.vertices.len();
                cx.vertices_to_indices.insert(bits, index);
                cx.vertices_indices[i] = index;
                cx.mesh.vertices.push(*vertex);
                /*
                if has_color {
                    cx.mesh.colors.push(triangle.color);
                }
                */
            }
        }

        cx.mesh.triangles.push(IndexTriangle {
            normal: triangle.normal,
            vertices_indices: cx.vertices_indices,
        });
    }

    fn reserve(cx: &mut Self::Context, num_triangles: u32) {
        // Use reserve_exact because binary stl has information on the exact number of triangles.
        cx.mesh.triangles.reserve_exact(num_triangles as _);
        // The number of vertices can be up to three times the number of triangles,
        // but is usually less than the number of triangles because of deduplication.
        // `num_triangles / 1.6` is a heuristic based on the results of benchmarks.
        let cap = (num_triangles as f64 / 1.6) as usize;
        cx.mesh.vertices.reserve(cap);
        cx.vertices_to_indices.reserve(cap);
    }

    fn set_name<S>(cx: &mut Self::Context, name: S)
    where
        S: Into<String>,
    {
        cx.mesh.name = name.into();
    }

    /*
    fn set_default_vertex_color(cx: &mut Self::Context, color: Color) {
        cx.default_vertex_color = Some(color);
    }

    fn set_material_color(cx: &mut Self::Context, color: Color) {
        cx.mesh.material_color = Some(color);
    }
    */
}

#[derive(Default)]
struct IndexMeshNodedup {
    mesh: IndexMesh,
    /*
    default_vertex_color: Option<Color>,
    */
}

impl FromStl for IndexMeshNodedup {
    type Context = Self;

    fn start() -> Self::Context {
        Self::default()
    }

    fn end(mut cx: Self::Context) -> Self {
        cx.mesh.vertices.shrink_to_fit();
        cx.mesh.triangles.shrink_to_fit();
        /*
        cx.mesh.colors.shrink_to_fit();
        */
        cx
    }

    #[inline]
    fn push_triangle(cx: &mut Self::Context, triangle: Triangle) {
        /*
        let mut has_color = !cx.mesh.colors.is_empty();
        if triangle.color.is_some() && !has_color {
            has_color = true;
            cx.mesh.colors.reserve_exact(cx.mesh.vertices.capacity());
            while cx.mesh.vertices.len() > cx.mesh.colors.len() {
                cx.mesh.colors.push(cx.default_vertex_color);
            }
        }
        */

        let vertices_indices = [
            cx.mesh.vertices.len(),
            cx.mesh.vertices.len() + 1,
            cx.mesh.vertices.len() + 2,
        ];
        // for vertex in triangle.vertices {
        cx.mesh.vertices.extend_from_slice(&triangle.vertices);
        /*
        if has_color {
            cx.mesh.colors.push(triangle.color);
        }
        */
        // }

        cx.mesh.triangles.push(IndexTriangle {
            normal: triangle.normal,
            vertices_indices,
        });
    }

    fn reserve(cx: &mut Self::Context, num_triangles: u32) {
        // Use reserve_exact because binary stl has information on the exact number of triangles.
        cx.mesh.triangles.reserve_exact(num_triangles as usize);
        cx.mesh.vertices.reserve_exact(num_triangles as usize * 3);
    }

    fn set_name<S>(cx: &mut Self::Context, name: S)
    where
        S: Into<String>,
    {
        cx.mesh.name = name.into();
    }

    /*
    fn set_default_vertex_color(cx: &mut Self::Context, color: Color) {
        cx.default_vertex_color = Some(color);
    }

    fn set_material_color(cx: &mut Self::Context, color: Color) {
        cx.mesh.material_color = Some(color);
    }
    */
}

impl From<Mesh> for IndexMesh {
    fn from(mesh: Mesh) -> Self {
        let mut cx = Self::start();
        Self::reserve(&mut cx, mesh.triangles.len() as _);
        for triangle in mesh.triangles {
            Self::push_triangle(&mut cx, triangle);
        }
        Self::set_name(&mut cx, mesh.name);
        Self::end(cx)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct IndexTriangle {
    pub normal: Vector3D,
    pub vertices_indices: [usize; 3],
}

impl IndexTriangle {
    /// Creates a new `IndexTriangle`.
    #[inline]
    pub fn new(normal: Vector3D, vertices_indices: [usize; 3]) -> Self {
        Self {
            normal,
            vertices_indices,
        }
    }
}
