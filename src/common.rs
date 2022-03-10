pub(crate) type Vec3 = [f32; 3];
pub(crate) type Face = [u32; 3];

/// Triangle mesh
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub faces: Vec<Face>,
}
