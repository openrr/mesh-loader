mod material;
mod mesh;
mod scene;
mod texture;
mod types;

pub use self::{material::*, mesh::*, scene::*, texture::*, types::*};

// TODO: assimp uses 8 here
pub const MAX_NUMBER_OF_TEXCOORDS: usize = 2;
// TODO: assimp uses 8 here
pub const MAX_NUMBER_OF_COLOR_SETS: usize = 2;
