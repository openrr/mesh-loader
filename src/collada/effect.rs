use super::*;

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=277
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct LibraryEffects {
    /// The unique identifier of this element.
    pub id: Option<String>,
    /// The name of this element.
    pub name: Option<String>,

    pub effects: IndexMap<String, Effect>,
}

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=265
// TODO: remove clone
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Effect {
    /// The unique identifier of this element.
    pub id: String,
    /// The name of this element.
    pub name: Option<String>,

    pub profile: ProfileCommon,
}

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=301
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ProfileCommon {
    /// The unique identifier of this element.
    pub id: Option<String>,

    pub surfaces: HashMap<String, Surface>,
    pub samplers: HashMap<String, Sampler>,
    pub technique: Technique,
}

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=332
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Surface {
    pub init_from: String,
}

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=312
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Sampler {
    // An xs:NCName, which is the sid of a <surface>. A
    // <sampler*> is a definition of how a shader will resolve a
    // color out of a <surface>. <source> identifies the
    // <surface> to read.
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShadeType {
    Constant,
    Lambert,
    Phong,
    Blinn,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ColorOrTexture {
    pub color: Color4,
    pub texture: Texture,
}

impl ColorOrTexture {
    fn new(color: Color4) -> Self {
        Self { color, texture: Texture { texture: String::new(), texcoord: String::new() } }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Texture {
    pub texture: String,
    pub texcoord: String,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Transparent {
    pub opaque: Option<Opaque>,
    pub color: Color4,
    pub texture: Texture,
}

// =============================================================================
// Parsing

pub(crate) fn parse_library_effects(cx: &mut Context, node: xml::Node<'_, '_>) -> Result<()> {
    debug_assert_eq!(node.tag_name().name(), "library_effects");
    cx.library_effects.id = node.attribute("id").map(Into::into);
    cx.library_effects.name = node.attribute("name").map(Into::into);

    for child in node.element_children() {
        match child.tag_name().name() {
            "effect" => {
                let effect = parse_effect(cx, child)?;
                cx.library_effects.effects.insert(effect.id.clone(), effect);
            }
            "asset" | "extra" => { /* skip */ }
            _ => error::unexpected_child_elem(child)?,
        }
    }

    // The specification says <library_effects> has 1 or more <effect> elements,
    // but some exporters write empty <library_effects/> tags.

    Ok(())
}

/*
The `<effect>` element

Attributes:
- `id` (xs:ID, Required)
- `name` (xs:token, Optional)

Child elements must appear in the following order if present:
- `<asset>` (0 or 1)
- `<annotate>` (0 or more)
- `<newparam>` (0 or more)
- profile (1 or more)
    At least one profile must appear, but any number of any of
    the following profiles can be included:
    - <profile_BRIDGE>
    - <profile_CG>
    - <profile_GLES>
    - <profile_GLES2>
    - <profile_GLSL>
    - <profile_COMMON>
- `<extra>` (0 or more)
*/
fn parse_effect(cx: &mut Context, node: xml::Node<'_, '_>) -> Result<Effect> {
    debug_assert_eq!(node.tag_name().name(), "effect");
    let id = node.required_attribute("id")?;
    let mut profile = None;

    for child in node.element_children() {
        if child.tag_name().name() == "profile_COMMON" {
            profile = Some(parse_profile_common(cx, child)?);
        }
    }

    let profile = match profile {
        Some(profile) => profile,
        None => error::exactly_one_elem(node, "profile_COMMON")?,
    };

    Ok(Effect { id: id.into(), name: node.attribute("name").map(Into::into), profile })
}

/*
The `<profile_COMMON>` element

Attributes:
- `id` (xs:ID, Optional)

Child elements must appear in the following order if present:
- `<asset>` (0 or 1)
- `<newparam>` (0 or more)
- `<technique>` (1)
- `<extra>` (0 or more)

Child Elements for `<profile_COMMON>` / `<technique>`
Child elements must appear in the following order if present:
- `<asset>` (0 or 1)
- shader_element (0 or more)
    One of `<constant>` (FX), `<lambert>`, `<phong>`, or `<blinn>`.
- `<extra>` (0 or more)
*/
fn parse_profile_common(cx: &mut Context, node: xml::Node<'_, '_>) -> Result<ProfileCommon> {
    debug_assert_eq!(node.tag_name().name(), "profile_COMMON");
    let mut surfaces = HashMap::new();
    let mut samplers = HashMap::new();
    let mut technique = None;

    for child in node.element_children() {
        match child.tag_name().name() {
            "newparam" => {
                parse_newparam(cx, child, &mut surfaces, &mut samplers)?;
            }
            "technique" => {
                for t in child.element_children() {
                    let name = t.tag_name().name();
                    match name {
                        "constant" | "lambert" | "phong" | "blinn" => {
                            technique = Some(parse_technique(t, name.parse().unwrap())?)
                        }
                        "asset" | "extra" => { /* skip */ }
                        _ => {}
                    }
                }
            }
            "asset" | "extra" => { /* skip */ }
            _ => error::unexpected_child_elem(child)?,
        }
    }

    let technique = match technique {
        Some(technique) => technique,
        // TODO: technique maybe flatten?
        None => error::exactly_one_elem(node, "technique")?,
    };

    Ok(ProfileCommon { id: node.attribute("id").map(Into::into), surfaces, samplers, technique })
}

fn parse_newparam(
    _cx: &mut Context,
    node: xml::Node<'_, '_>,
    surfaces: &mut HashMap<String, Surface>,
    samplers: &mut HashMap<String, Sampler>,
) -> Result<()> {
    debug_assert_eq!(node.tag_name().name(), "newparam");
    let sid = node.required_attribute("sid")?;

    for child in node.element_children() {
        match child.tag_name().name() {
            "surface" => {
                // image ID given inside <init_from> tags
                if let Some(init) = child.child("init_from") {
                    surfaces.insert(sid.to_owned(), Surface {
                        init_from: init.text().unwrap_or_default().trim().to_owned(),
                    });
                }
            }
            "sampler2D" => {
                // surface ID is given inside <source> tags
                if let Some(source) = child.child("source") {
                    samplers.insert(sid.to_owned(), Sampler {
                        source: source.text().unwrap_or_default().trim().to_owned(),
                    });
                }
            }
            _ => error::unexpected_child_elem(child)?,
        }
    }

    Ok(())
}

impl FromStr for ShadeType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "constant" => Self::Constant,
            "lambert" => Self::Lambert,
            "phong" => Self::Phong,
            "blinn" => Self::Blinn,
            _ => bail!("unknown shade type {:?}", s),
        })
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Technique {
    pub ty: ShadeType,

    // Colors/Textures
    pub emission: ColorOrTexture,
    pub ambient: ColorOrTexture,
    pub diffuse: ColorOrTexture,
    pub specular: ColorOrTexture,
    pub reflective: ColorOrTexture,
    pub transparent: Transparent,

    /// Scalar factory
    pub shininess: f32,
    pub index_of_refraction: f32,
    pub reflectivity: f32,
    pub transparency: f32,
    pub has_transparency: bool,
    pub rgb_transparency: bool,
    pub invert_transparency: bool,

    // MAX3D extensions
    pub double_sided: bool,
    pub wireframe: bool,
    pub faceted: bool,
}

/*
Child elements must appear in the following order if present:
- <emission> (0 or 1, fx_common_color_or_texture_type)
- <ambient> (FX) (0 or 1, fx_common_color_or_texture_type)
- <diffuse> (0 or 1, fx_common_color_or_texture_type)
- <specular> (0 or 1, fx_common_color_or_texture_type)
- <shininess> (0 or 1, fx_common_float_or_param_type)
- <reflective> (0 or 1, fx_common_color_or_texture_type)
- <reflectivity> (0 or 1, fx_common_float_or_param_type 0.0 ..= 1.0)
- <transparent> (0 or 1, fx_common_color_or_texture_type)
- <transparency> (0 or 1, fx_common_float_or_param_type 0.0 ..= 1.0)
- <index_of_refraction> (0 or 1, fx_common_float_or_param_type)
*/
fn parse_technique(node: xml::Node<'_, '_>, ty: ShadeType) -> Result<Technique> {
    debug_assert_eq!(node.tag_name().name().parse::<ShadeType>().unwrap(), ty);
    let mut effect = Technique::new();

    for child in node.element_children() {
        let name = child.tag_name().name();
        match name {
            // fx_common_color_or_texture_type
            "emission" => {
                parse_effect_color(
                    child,
                    &mut effect.emission.color,
                    &mut effect.emission.texture,
                )?;
            }
            "ambient" => {
                parse_effect_color(child, &mut effect.ambient.color, &mut effect.ambient.texture)?;
            }
            "diffuse" => {
                parse_effect_color(child, &mut effect.diffuse.color, &mut effect.diffuse.texture)?;
            }
            "specular" => {
                parse_effect_color(
                    child,
                    &mut effect.specular.color,
                    &mut effect.specular.texture,
                )?;
            }
            "reflective" => {
                parse_effect_color(
                    child,
                    &mut effect.reflective.color,
                    &mut effect.reflective.texture,
                )?;
            }
            "transparent" => {
                effect.transparent.opaque = child.parse_attribute("opaque")?;
                parse_effect_color(
                    child,
                    &mut effect.transparent.color,
                    &mut effect.transparent.texture,
                )?;
            }

            // fx_common_float_or_param_type
            "shininess" => {
                if let Some(n) = parse_effect_float(child)? {
                    effect.shininess = n;
                }
            }
            "reflectivity" => {
                if let Some(n) = parse_effect_float(child)? {
                    effect.reflectivity = n;
                }
            }
            "transparency" => {
                if let Some(n) = parse_effect_float(child)? {
                    effect.transparency = n;
                }
            }
            "index_of_refraction" => {
                if let Some(n) = parse_effect_float(child)? {
                    effect.index_of_refraction = n;
                }
            }

            // GOOGLEEARTH/OKINO extensions
            "double_sided" => {
                effect.double_sided = node.parse_required_attribute(name)?;
            }

            // MAX3D extensions
            "wireframe" => {
                effect.wireframe = node.parse_required_attribute(name)?;
            }
            "faceted" => {
                effect.faceted = node.parse_required_attribute(name)?;
            }

            _ => {}
        }
    }

    Ok(effect)
}

impl Technique {
    fn new() -> Self {
        Self {
            ty: ShadeType::Phong,
            emission: ColorOrTexture::new([0.0, 0.0, 0.0, 1.0]),
            ambient: ColorOrTexture::new([0.1, 0.1, 0.1, 1.0]),
            diffuse: ColorOrTexture::new([0.6, 0.6, 0.6, 1.0]),
            specular: ColorOrTexture::new([0.4, 0.4, 0.4, 1.0]),
            transparent: Transparent {
                opaque: None,
                // refs: https://www.khronos.org/files/collada_spec_1_5.pdf#page=250
                color: [1.0, 1.0, 1.0, 1.0],
                texture: Texture { texture: String::new(), texcoord: String::new() },
            },
            reflective: ColorOrTexture::new([0.0, 0.0, 0.0, 1.0]),
            shininess: 10.0,
            index_of_refraction: 1.0,
            reflectivity: 0.0,
            // refs: https://www.khronos.org/files/collada_spec_1_5.pdf#page=250
            transparency: 1.0,
            has_transparency: false,
            rgb_transparency: false,
            invert_transparency: false,
            double_sided: false,
            wireframe: false,
            faceted: false,
        }
    }
}

// #[derive(Debug, Clone)]
// #[non_exhaustive]
// pub enum ColorOrTexture {
//     Color(Color4),
//     Texture { texture: String, texcoord: String },
// }

// impl ColorOrTexture {
//     pub fn as_color(&self) -> Option<Color4> {
//         match self {
//             Self::Color(c) => Some(*c),
//             Self::Texture { .. } => None,
//         }
//     }

//     pub fn as_texture(&self) -> Option<(&str, &str)> {
//         match self {
//             Self::Color(..) => None,
//             Self::Texture { texture, texcoord } => Some((texture, texcoord)),
//         }
//     }
// }

// Attributes:
// Only <transparent> has an attribute
// - `opaque` (Enumeration, Optional)
//
// Child Elements:
// Note: Exactly one of the child elements `<color>`, `<param>`, or
// `<texture>` must appear. They are mutually exclusive.
// - `<color>`
// - `<param>` (reference)
// - `<texture>`
//
// See also fx_common_color_or_texture_type in specification.
fn parse_effect_color(
    node: xml::Node<'_, '_>,
    color: &mut Color4,
    texture: &mut Texture,
) -> Result<()> {
    for child in node.element_children() {
        match child.tag_name().name() {
            "color" => {
                // TODO: https://stackoverflow.com/questions/4325363/converting-a-number-with-comma-as-decimal-point-to-float
                let content = child.text().unwrap_or_default().trim().replace(',', ".");
                let mut iter = float::parse_array_exact(&content, 4);

                let r = iter.next().unwrap()?;
                let g = iter.next().unwrap()?;
                let b = iter.next().unwrap()?;
                let a = iter.next().unwrap()?;
                *color = [r, g, b, a];
            }
            "texture" => {
                *texture = Texture {
                    texture: child.required_attribute("texture")?.into(),
                    texcoord: child.required_attribute("texcoord")?.into(),
                };
            }
            "param" => warn::unsupported_child_elem(child),
            _ => {}
        }
    }
    Ok(())
}

fn parse_effect_float(node: xml::Node<'_, '_>) -> Result<Option<f32>> {
    let mut float = None;

    for child in node.element_children() {
        match child.tag_name().name() {
            "float" => {
                // TODO: https://stackoverflow.com/questions/4325363/converting-a-number-with-comma-as-decimal-point-to-float
                let content = child.text().unwrap_or_default().trim().replace(',', ".");
                float = Some(fast_float::parse(&content)?);
            }
            "param" => warn::unsupported_child_elem(child),
            _ => error::unexpected_child_elem(child)?,
        }
    }

    Ok(float)
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Opaque {
    A_ZERO,
    A_ONE,
    RGB_ZERO,
    RGB_ONE,
}

impl Opaque {
    pub fn rgb_transparency(self) -> bool {
        matches!(self, Self::RGB_ZERO | Self::RGB_ONE)
    }

    pub fn invert_transparency(self) -> bool {
        matches!(self, Self::RGB_ZERO | Self::A_ZERO)
    }
}

impl FromStr for Opaque {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "A_ZERO" => Self::A_ZERO,
            "A_ONE" => Self::A_ONE,
            "RGB_ZERO" => Self::RGB_ZERO,
            "RGB_ONE" => Self::RGB_ONE,
            _ => bail!("unknown shade type {:?}", s),
        })
    }
}

/*

#[derive(Debug)]
#[non_exhaustive]
pub struct ProfileCommon {
    // Optional
    pub(crate) id: Option<String>,

    /// Shading mode
    pub(crate) shade_type: ShadeType,

    /// Scalar factory
    pub(crate) shininess: f32,
    pub(crate) refract_index: f32,
    pub(crate) reflectivity: f32,
    pub(crate) transparency: f32,
    pub(crate) has_transparency: bool,
    pub(crate) rgb_transparency: bool,
    pub(crate) invert_transparency: bool,

    /// local params referring to each other by their SID
    pub(crate) params: HashMap<String, EffectParam>,

}
*/
/*
#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) struct Sampler {
    /// Name of image reference
    pub(crate) name: Option<String>,

    /// Wrap U?
    pub(crate) wrap_u: bool,

    /// Wrap V?
    pub(crate) wrap_v: bool,

    /// Mirror U?
    pub(crate) mirror_u: bool,

    /// Mirror V?
    pub(crate) mirror_v: bool,

    // /// Blend mode
    // pub(crate) op: aiTextureOp,

    // /// UV transformation
    // pub(crate) transform: aiUVTransform,
    /// Name of source UV channel
    pub(crate) uv_channel: Option<String>,

    /// Resolved UV channel index or UINT_MAX if not known
    pub(crate) uv_id: u32,

    // OKINO/MAX3D extensions from here
    // -------------------------------------------------------
    /// Weighting factor
    pub(crate) weighting: f32,

    /// Mixing factor from OKINO
    pub(crate) mix_with_previous: f32,
}

impl Default for Sampler {
    fn default() -> Self {
        Self {
            name: None,
            wrap_u: true,
            wrap_v: true,
            mirror_u: false,
            mirror_v: false,
            // Op(aiTextureOp_Multiply),
            uv_channel: None,
            uv_id: u32::MAX,
            weighting: 1.0,
            mix_with_previous: 1.0,
        }
    }
}
*/
