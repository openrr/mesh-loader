use super::{Color4, Vec2, Vec3, MAX_NUMBER_OF_COLOR_SETS, MAX_NUMBER_OF_TEXCOORDS};

/// Triangle mesh
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct TriMesh {
    pub name: String,
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub texcoords: [Vec<Vec2>; MAX_NUMBER_OF_TEXCOORDS],
    pub faces: Vec<Face>,
    pub colors: [Vec<Color4>; MAX_NUMBER_OF_COLOR_SETS],
}

pub type Face = [u32; 3];
