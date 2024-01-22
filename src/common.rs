use std::{fmt, path::PathBuf};

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
    pub materials: Vec<Material>,
    pub meshes: Vec<Mesh>,
}

/// Triangle mesh
#[derive(Default)]
#[non_exhaustive]
pub struct Mesh {
    pub name: String,
    pub vertices: Vec<Vec3>,
    pub texcoords: [Vec<Vec2>; MAX_NUMBER_OF_TEXCOORDS],
    pub normals: Vec<Vec3>,
    pub faces: Vec<Face>,
    pub colors: [Vec<Color4>; MAX_NUMBER_OF_COLOR_SETS],
    #[cfg(feature = "obj")]
    pub(crate) material_index: u32,
}

impl Mesh {
    #[inline]
    #[must_use]
    pub fn merge(mut meshes: Vec<Self>) -> Self {
        if meshes.len() <= 1 {
            return meshes.pop().unwrap_or_default();
        }

        let num_vertices = meshes.iter().map(|m| m.vertices.len()).sum();
        let mut vertices = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);
        // TODO: fill with default if one or more meshes has colors
        let has_colors0 = num_vertices == meshes.iter().map(|m| m.colors[0].len()).sum();
        let mut colors0 = Vec::with_capacity(if has_colors0 { num_vertices } else { 0 });
        let has_colors1 = num_vertices == meshes.iter().map(|m| m.colors[1].len()).sum();
        let mut colors1 = Vec::with_capacity(if has_colors1 { num_vertices } else { 0 });
        for m in &meshes {
            vertices.extend_from_slice(&m.vertices);
            normals.extend_from_slice(&m.normals);
            if has_colors0 {
                colors0.extend_from_slice(&m.colors[0]);
            }
            if has_colors1 {
                colors1.extend_from_slice(&m.colors[1]);
            }
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
            texcoords: Default::default(), // TODO
            normals,
            faces,
            colors: [colors0, colors1],
            #[cfg(feature = "obj")]
            material_index: u32::MAX,
        }
    }
}

impl fmt::Debug for Mesh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mesh")
            .field("name", &self.name)
            .field("num_vertices", &self.vertices.len())
            .field("num_texcoords0", &self.texcoords[0].len())
            .field("num_texcoords1", &self.texcoords[1].len())
            .field("num_normals", &self.normals.len())
            .field("num_faces", &self.faces.len())
            .field("num_colors0", &self.colors[0].len())
            .field("num_colors1", &self.colors[1].len())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Material {
    pub name: String,
    pub color: Colors,
    pub texture: Textures,
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Colors {
    pub ambient: Option<Color4>,
    pub diffuse: Option<Color4>,
    pub specular: Option<Color4>,
    pub emissive: Option<Color4>,
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Textures {
    pub ambient: Option<PathBuf>,
    pub diffuse: Option<PathBuf>,
    pub specular: Option<PathBuf>,
    pub emissive: Option<PathBuf>,
    pub normal: Option<PathBuf>,
}
