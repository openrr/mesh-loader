use super::{material::Material, mesh::TriMesh as Mesh, texture::Texture};

#[derive(Debug, Default)]
pub struct Scene {
    pub materials: Vec<Material>,
    pub textures: Vec<Texture>,
    pub meshes: Vec<Mesh>,
    // pub metadata: Option<MetaData>,
    // pub animations: Vec<Animation>,
    // pub cameras: Vec<Camera>,
    // pub lights: Vec<Light>,
    // pub nodes: SlotMap<NodeIndex, Node>,
    // pub root: Option<NodeIndex>,
    // pub flags: u32,
}
