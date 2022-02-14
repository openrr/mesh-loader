use super::{material::Material, mesh::TriMesh as Mesh, texture::Texture};

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Scene {
    pub materials: Vec<Material>,
    pub textures: Vec<Texture>,
    pub meshes: Vec<Mesh>,
}
