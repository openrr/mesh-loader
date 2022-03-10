mod mesh;
mod types;

pub use self::{mesh::*, types::*};

// TODO: assimp uses 8 here
pub const MAX_NUMBER_OF_TEXCOORDS: usize = 2;
// TODO: assimp uses 8 here
pub const MAX_NUMBER_OF_COLOR_SETS: usize = 2;
