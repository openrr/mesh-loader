pub(crate) type Vec2 = [f32; 2];
pub(crate) type Vec3 = [f32; 3];
pub(crate) type Face = [u32; 3];
pub(crate) type Color4 = [f32; 4];

// TODO: assimp uses 8 here
pub(crate) const MAX_NUMBER_OF_TEXCOORDS: usize = 2;
// TODO: assimp uses 8 here
pub(crate) const MAX_NUMBER_OF_COLOR_SETS: usize = 2;

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Scene {
    pub meshes: Vec<Mesh>,
}

/// Triangle mesh
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Mesh {
    pub name: String,
    pub vertices: Vec<Vec3>,
    pub texcoords: [Vec<Vec2>; MAX_NUMBER_OF_TEXCOORDS],
    pub normals: Vec<Vec3>,
    pub faces: Vec<Face>,
    pub colors: [Vec<Color4>; MAX_NUMBER_OF_COLOR_SETS],
}
