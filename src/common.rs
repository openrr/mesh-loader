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

impl Mesh {
    #[inline]
    pub fn merge(mut meshes: Vec<Self>) -> Self {
        if meshes.len() <= 1 {
            return meshes.pop().unwrap_or_default();
        }
        let mut vertices = Vec::with_capacity(meshes.iter().map(|m| m.vertices.len()).sum());
        for m in &meshes {
            vertices.extend_from_slice(&m.vertices);
        }
        let mut faces = Vec::with_capacity(meshes.iter().map(|m| m.faces.len()).sum());
        let mut last = 0;
        for m in &meshes {
            if m.faces.is_empty() {
                continue;
            }
            faces.extend(
                m.faces
                    .iter()
                    .map(|f| [f[0] + last, f[1] + last, f[2] + last]),
            );
            last = m.faces.last().unwrap()[2] + 1;
        }
        Self {
            name: String::new(),
            vertices,
            faces,
            ..Default::default() // TODO
        }
    }
}
