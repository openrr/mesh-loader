use std::{collections::HashMap, convert::TryFrom};

use anyhow::bail;

use super::texture::{Texture, TextureType};
use crate::{Color4, TODO};

#[derive(Debug, Default)]
pub struct Material {
    pub properties: Properties,
    // TODO
    pub textures: HashMap<TextureType, Vec<Texture>>,
}

impl Material {
    // pub fn add_property<P, S>(&mut self, input: P, key: S)
    // where
    //     P: Into<PropertyTypeInfo>,
    //     S: Into<String>,
    // {
    //     self.properties.insert(key.into(), MaterialProperty {
    //         data: input.into(),
    //         index: self.properties.len(),
    //         semantic: TextureType::Unknown,
    //     });
    // }
}

#[derive(Debug, Default)]
pub struct Properties {
    pub name: Option<String>,
    pub two_sided: bool,
    pub shading_model: Option<ShadingMode>,
    pub wireframe: bool,
    pub blend: Option<TODO>,
    pub opacity: Option<f32>,
    pub transparency_factor: Option<TODO>,
    pub bump_scaling: Option<TODO>,
    pub shininess: Option<f32>,
    pub reflectivity: Option<f32>,
    pub shininess_strength: Option<TODO>,
    pub refracti: Option<f32>,
    pub color_diffuse: Option<Color4>,
    pub color_ambient: Option<Color4>,
    pub color_specular: Option<Color4>,
    pub color_emissive: Option<Color4>,
    pub color_transparent: Option<Color4>,
    pub color_reflective: Option<Color4>,
    pub global_background_image: Option<TODO>,
    pub global_shaderlang: Option<TODO>,
    pub shader_vertex: Option<TODO>,
    pub shader_fragment: Option<TODO>,
    pub shader_geo: Option<TODO>,
    pub shader_tesselation: Option<TODO>,
    pub shader_primitive: Option<TODO>,
    pub shader_compute: Option<TODO>,

    // PBR material support
    // Properties defining PBR rendering techniques
    pub use_color_map: Option<TODO>,

    // Metallic/Roughness Workflow
    pub base_color: Option<TODO>,
    pub base_color_texture: Option<TODO>,
    pub use_metallic_map: Option<TODO>,
    pub metallic_factor: Option<TODO>,
    pub metallic_texture: Option<TODO>,
    pub use_roughness_map: Option<TODO>,
    pub roughness_factor: Option<TODO>,
    pub roughness_texture: Option<TODO>,

    // Specular/Glossiness Workflow
    pub specular_factor: Option<TODO>,
    pub glossiness_factor: Option<TODO>,

    // Sheen
    pub sheen_color_factor: Option<TODO>,
    pub sheen_roughness_factor: Option<TODO>,
    pub sheen_color_texture: Option<TODO>,
    pub sheen_roughness_texture: Option<TODO>,

    // Clearcoat
    pub clearcoat_factor: Option<TODO>,
    pub clearcoat_roughness_factor: Option<TODO>,
    pub clearcoat_texture: Option<TODO>,
    pub clearcoat_roughness_texture: Option<TODO>,
    pub clearcoat_normal_texture: Option<TODO>,

    // Transmission
    pub transmission_factor: Option<TODO>,
    pub transmission_texture: Option<TODO>,

    // Emissive
    pub use_emissive_map: Option<TODO>,
    pub emissive_intensity: Option<TODO>,
    pub use_ao_map: Option<TODO>,

    // Pure key names for all texture-related properties
    pub texture_base: Option<TODO>,
    pub uvwsrc_base: Option<TODO>,
    pub texop_base: Option<TODO>,
    pub mapping_base: Option<TODO>,
    pub texblend_base: Option<TODO>,
    pub mappingmode_u_base: Option<TODO>,
    pub mappingmode_v_base: Option<TODO>,
    pub texmap_axis_base: Option<TODO>,
    pub uvtransform_base: Option<TODO>,
    pub texflags_base: Option<TODO>,
}

#[derive(Debug)]
pub struct MaterialProperty {
    pub data: PropertyTypeInfo,
    pub index: usize,
    // TODO
    // pub semantic: TextureType,
}

#[derive(Debug, PartialEq)]
#[repr(u32)]
pub enum PropertyTypeInfo {
    // Force32Bit, aiPropertyTypeInfo__aiPTI_Force32Bit Not sure how to handle this
    Buffer(Vec<u8>),
    IntegerArray(Vec<i32>),
    FloatArray(Vec<f32>),
    String(String),
}

impl From<Vec<u8>> for PropertyTypeInfo {
    fn from(v: Vec<u8>) -> Self {
        Self::Buffer(v)
    }
}

impl From<&[u8]> for PropertyTypeInfo {
    fn from(v: &[u8]) -> Self {
        Self::Buffer(v.to_vec())
    }
}

impl From<Vec<i32>> for PropertyTypeInfo {
    fn from(v: Vec<i32>) -> Self {
        Self::IntegerArray(v)
    }
}

impl From<&[i32]> for PropertyTypeInfo {
    fn from(v: &[i32]) -> Self {
        Self::IntegerArray(v.to_vec())
    }
}

impl From<i32> for PropertyTypeInfo {
    fn from(v: i32) -> Self {
        Self::IntegerArray(vec![v])
    }
}

impl From<Vec<f32>> for PropertyTypeInfo {
    fn from(v: Vec<f32>) -> Self {
        Self::FloatArray(v)
    }
}

impl From<f32> for PropertyTypeInfo {
    fn from(v: f32) -> Self {
        Self::FloatArray(vec![v])
    }
}

impl From<&[f32]> for PropertyTypeInfo {
    fn from(v: &[f32]) -> Self {
        Self::FloatArray(v.to_vec())
    }
}

impl<const N: usize> From<[f32; N]> for PropertyTypeInfo {
    fn from(v: [f32; N]) -> Self {
        Self::FloatArray(v.to_vec())
    }
}

impl From<String> for PropertyTypeInfo {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for PropertyTypeInfo {
    fn from(v: &str) -> Self {
        Self::String(v.to_owned())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadingMode {
    /// Flat shading. Shading is done on per-face base,
    /// diffuse only. Also known as 'faceted shading'.
    Flat = 0x1,
    /// Simple Gouraud shading.
    Gouraud = 0x2,
    /// Phong-Shading
    Phong = 0x3,
    /// Phong-Blinn-Shading
    Blinn = 0x4,
    /// Toon-Shading per pixel
    ///
    /// Also known as 'comic' shader.
    Toon = 0x5,
    /// OrenNayar
    OrenNayar = 0x6,
    /// Minnaert-Shading per pixel
    ///
    /// Extension to standard Lambertian shading, taking the
    /// "darkness" of the material into account
    Minnaert = 0x7,
    /// CookTorrance-Shading per pixel
    ///
    /// Special shader for metallic surfaces.
    CookTorrance = 0x8,
    /// No shading at all. Constant light influence of 1.0.
    /// Also known as "Unlit"
    NoShading = 0x9,
    /// Fresnel shading
    Fresnel = 0xa,
    /// Physically-Based Rendering (PBR) shading using
    /// Bidirectional scattering/reflectance distribution function (BSDF/BRDF)
    /// There are multiple methods under this banner, and model files may provide
    /// data for more than one PBR-BRDF method.
    /// Applications should use the set of provided properties to determine which
    /// of their preferred PBR rendering methods are likely to be available
    /// eg:
    /// - If METALLIC_FACTOR is set, then a Metallic/Roughness is available
    /// - If GLOSSINESS_FACTOR is set, then a Specular/Glossiness is available
    /// Note that some PBR methods allow layering of techniques
    PbrBrdF = 0xb,
}

impl TryFrom<u8> for ShadingMode {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0x1 => Self::Flat,
            0x2 => Self::Gouraud,
            0x3 => Self::Phong,
            0x4 => Self::Blinn,
            0x5 => Self::Toon,
            0x6 => Self::OrenNayar,
            0x7 => Self::Minnaert,
            0x8 => Self::CookTorrance,
            0x9 => Self::NoShading,
            0xa => Self::Fresnel,
            0xb => Self::PbrBrdF,
            _ => bail!("invalid shading mode"),
        })
    }
}
