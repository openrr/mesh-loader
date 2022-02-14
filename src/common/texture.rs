#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum TextureType {
    None,
    Diffuse,
    Specular,
    Ambient,
    Emissive,
    Height,
    Normals,
    Shininess,
    Opacity,
    Displacement,
    LightMap,
    Reflection,
    BaseColor,
    NormalCamera,
    EmissionColor,
    Metalness,
    Roughness,
    AmbientOcclusion,
    Unknown,
    Force32bit,
}

#[derive(Debug, Clone)]
pub struct Texel {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

#[derive(Debug, Clone, Default)]
pub struct Texture {
    pub path: String,
    pub texture_mapping: u32,
    pub uv_index: u32,
    pub blend: f32,
    pub op: u32,
    pub map_mode: Vec<u32>,
    pub flags: u32,
    pub height: u32,
    pub width: u32,
    pub ach_format_hint: String,
    pub data: Option<DataContent>,
}

#[derive(Debug, Clone)]
pub enum DataContent {
    Texel(Vec<Texel>),
    Bytes(Vec<u8>),
}

#[allow(dead_code)]
struct TextureComponent {
    path: String,
    texture_mapping: u32,
    uv_index: u32,
    blend: f32,
    op: u32,
    map_mode: Vec<u32>,
    flags: u32,
}

#[allow(dead_code)]
impl TextureComponent {
    fn new(
        path: String,
        texture_mapping: u32,
        uv_index: u32,
        blend: f32,
        op: u32,
        map_mode: Vec<u32>,
        flags: u32,
    ) -> TextureComponent {
        Self { path, texture_mapping, uv_index, blend, op, map_mode, flags }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum TextureMapMode {
    Clamp,
    Decal,
    Mirror,
    Wrap,
}
