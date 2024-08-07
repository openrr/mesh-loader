use super::*;

/// The `<library_effects>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=277
#[derive(Default)]
pub(super) struct LibraryEffects<'a> {
    // /// The unique identifier of this element.
    // pub(super) id: Option<&'a str>,
    // /// The name of this element.
    // pub(super) name: Option<&'a str>,
    pub(super) effects: HashMap<&'a str, Effect<'a>>,
}

/// The `<effect>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=265
pub(super) struct Effect<'a> {
    /// The unique identifier of this element.
    pub(super) id: &'a str,
    // /// The name of this element.
    // pub(super) name: Option<&'a str>,
    pub(super) profile: ProfileCommon<'a>,
}

/// The `<profile_COMMON>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=301
pub(super) struct ProfileCommon<'a> {
    // /// The unique identifier of this element.
    // pub(super) id: Option<&'a str>,
    pub(super) surfaces: HashMap<&'a str, Surface<'a>>,
    pub(super) samplers: HashMap<&'a str, Sampler<'a>>,

    pub(super) technique: Technique<'a>,
}

pub(super) struct Technique<'a> {
    #[allow(dead_code)] // TODO
    pub(super) ty: ShadeType,

    // Colors/Textures
    pub(super) emission: ColorAndTexture<'a>,
    pub(super) ambient: ColorAndTexture<'a>,
    pub(super) diffuse: ColorAndTexture<'a>,
    pub(super) specular: ColorAndTexture<'a>,
    pub(super) reflective: ColorAndTexture<'a>,
    pub(super) transparent: ColorAndTexture<'a>,
    pub(super) has_transparency: bool,
    pub(super) rgb_transparency: bool,
    pub(super) invert_transparency: bool,

    pub(super) shininess: f32,
    pub(super) reflectivity: f32,
    pub(super) transparency: f32,
    pub(super) index_of_refraction: f32,

    // GOOGLEEARTH/OKINO extensions
    pub(super) double_sided: bool,

    // FCOLLADA extensions
    pub(super) bump: Texture<'a>,

    // MAX3D extensions
    pub(super) wireframe: bool,
    pub(super) faceted: bool,
}

/// The `<surface>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=332
pub(super) struct Surface<'a> {
    pub(super) init_from: Uri<'a, Image<'a>>,
}

/// The `<sampler2D>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4] https://www.khronos.org/files/collada_spec_1_4.pdf#page=312
pub(super) struct Sampler<'a> {
    // An xs:NCName, which is the sid of a <surface>. A
    // <sampler*> is a definition of how a shader will resolve a
    // color out of a <surface>. <source> identifies the
    // <surface> to read.
    pub(super) source: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum ShadeType {
    Constant,
    Lambert,
    Phong,
    Blinn,
}

pub(super) struct ColorAndTexture<'a> {
    pub(super) color: Color4,
    pub(super) texture: Texture<'a>,
}

impl ColorAndTexture<'_> {
    fn new(color: Color4) -> Self {
        Self {
            color,
            texture: Texture {
                texture: "",
                // texcoord: "",
            },
        }
    }
}

pub(super) struct Texture<'a> {
    pub(super) texture: &'a str,
    // pub(super) texcoord: &'a str,
}

// -----------------------------------------------------------------------------
// Parsing

pub(super) fn parse_library_effects<'a>(
    cx: &mut Context<'a>,
    node: xml::Node<'a, '_>,
) -> io::Result<()> {
    debug_assert_eq!(node.tag_name().name(), "library_effects");
    // cx.library_effects.id = node.attribute("id");
    // cx.library_effects.name = node.attribute("name");

    for child in node.element_children() {
        match child.tag_name().name() {
            "effect" => {
                let effect = parse_effect(cx, child)?;
                cx.library_effects.effects.insert(effect.id, effect);
            }
            "asset" | "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(child)),
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
fn parse_effect<'a>(cx: &mut Context<'a>, node: xml::Node<'a, '_>) -> io::Result<Effect<'a>> {
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
        None => return Err(error::exactly_one_elem(node, "profile_COMMON")),
    };

    Ok(Effect {
        id,
        // name: node.attribute("name"),
        profile,
    })
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
fn parse_profile_common<'a>(
    cx: &mut Context<'a>,
    node: xml::Node<'a, '_>,
) -> io::Result<ProfileCommon<'a>> {
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
                            technique = Some(parse_technique(t, name.parse().unwrap())?);
                        }
                        "asset" | "extra" => { /* skip */ }
                        _ => {}
                    }
                }
            }
            "asset" | "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(child)),
        }
    }

    let technique = match technique {
        Some(technique) => technique,
        // TODO: technique maybe flatten?
        None => return Err(error::exactly_one_elem(node, "technique")),
    };

    Ok(ProfileCommon {
        // id: node.attribute("id"),
        surfaces,
        samplers,
        technique,
    })
}

fn parse_newparam<'a>(
    _cx: &mut Context<'a>,
    node: xml::Node<'a, '_>,
    surfaces: &mut HashMap<&'a str, Surface<'a>>,
    samplers: &mut HashMap<&'a str, Sampler<'a>>,
) -> io::Result<()> {
    debug_assert_eq!(node.tag_name().name(), "newparam");
    let sid = node.required_attribute("sid")?;

    for child in node.element_children() {
        match child.tag_name().name() {
            "surface" => {
                // image ID given inside <init_from> tags
                if let Some(init) = child.child("init_from") {
                    surfaces.insert(
                        sid,
                        Surface {
                            init_from: Uri::from_id(init.trimmed_text()),
                        },
                    );
                }
            }
            "sampler2D" => {
                // surface ID is given inside <source> tags
                if let Some(source) = child.child("source") {
                    samplers.insert(
                        sid,
                        Sampler {
                            source: source.trimmed_text(),
                        },
                    );
                }
            }
            _ => return Err(error::unexpected_child_elem(child)),
        }
    }

    Ok(())
}

impl FromStr for ShadeType {
    type Err = io::Error;

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
fn parse_technique<'a>(node: xml::Node<'a, '_>, ty: ShadeType) -> io::Result<Technique<'a>> {
    debug_assert_eq!(node.tag_name().name().parse::<ShadeType>().unwrap(), ty);
    let mut effect = Technique::new(ty);

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
                parse_effect_color(
                    child,
                    &mut effect.ambient.color,
                    &mut effect.ambient.texture,
                )?;
            }
            "diffuse" => {
                parse_effect_color(
                    child,
                    &mut effect.diffuse.color,
                    &mut effect.diffuse.texture,
                )?;
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
                effect.has_transparency = true;
                if let Some(opaque) = child.parse_attribute::<Opaque>("opaque")? {
                    effect.rgb_transparency = opaque.rgb_transparency();
                    effect.invert_transparency = opaque.invert_transparency();
                }
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

            // FCOLLADA extensions
            "bump" => {
                let mut dummy = [0.; 4];
                parse_effect_color(child, &mut dummy, &mut effect.bump)?;
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

impl Technique<'_> {
    fn new(ty: ShadeType) -> Self {
        Self {
            ty,
            emission: ColorAndTexture::new([0.0, 0.0, 0.0, 1.0]),
            ambient: ColorAndTexture::new([0.1, 0.1, 0.1, 1.0]),
            diffuse: ColorAndTexture::new([0.6, 0.6, 0.6, 1.0]),
            specular: ColorAndTexture::new([0.4, 0.4, 0.4, 1.0]),
            // refs: https://www.khronos.org/files/collada_spec_1_5.pdf#page=250
            transparent: ColorAndTexture::new([1.0, 1.0, 1.0, 1.0]),
            reflective: ColorAndTexture::new([0.0, 0.0, 0.0, 1.0]),
            shininess: 10.0,
            index_of_refraction: 1.0,
            reflectivity: 0.0,
            // refs: https://www.khronos.org/files/collada_spec_1_5.pdf#page=250
            transparency: 1.0,
            has_transparency: false,
            rgb_transparency: false,
            invert_transparency: false,
            double_sided: false,
            bump: Texture {
                texture: "",
                // texcoord: "",
            },
            wireframe: false,
            faceted: false,
        }
    }
}

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
fn parse_effect_color<'a>(
    node: xml::Node<'a, '_>,
    color: &mut Color4,
    texture: &mut Texture<'a>,
) -> io::Result<()> {
    for child in node.element_children() {
        match child.tag_name().name() {
            "color" => {
                let content = xml::comma_to_period(child.trimmed_text());
                let mut iter = xml::parse_float_array_exact(&content, 4);
                // TODO: include in parse_float_array_exact?
                let map_err = |e| {
                    format_err!(
                        "{e} in <{}> element ({})",
                        child.tag_name().name(),
                        child.text_location(),
                    )
                };
                let r = iter.next().unwrap().map_err(map_err)?;
                let g = iter.next().unwrap().map_err(map_err)?;
                let b = iter.next().unwrap().map_err(map_err)?;
                let a = iter.next().unwrap().map_err(map_err)?;
                *color = [r, g, b, a];
            }
            "texture" => {
                let _texcoord = child.required_attribute("texcoord")?;
                *texture = Texture {
                    texture: child.required_attribute("texture")?,
                    // texcoord,
                };
            }
            "param" => {
                // warn!(
                //     "<{}> child element in <{}> element is unsupported ({})",
                //     child.tag_name().name(),
                //     child.parent_element().unwrap().tag_name().name(),
                //     child.node_location()
                // );
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_effect_float(node: xml::Node<'_, '_>) -> io::Result<Option<f32>> {
    let mut float = None;

    for child in node.element_children() {
        match child.tag_name().name() {
            "float" => {
                let content = xml::comma_to_period(child.trimmed_text());
                float = Some(
                    float::parse(content.as_bytes())
                        .ok_or_else(|| format_err!("error while parsing a float"))?,
                );
            }
            "param" => {
                // warn!(
                //     "<{}> child element in <{}> element is unsupported ({})",
                //     child.tag_name().name(),
                //     child.parent_element().unwrap().tag_name().name(),
                //     child.node_location()
                // );
            }
            _ => return Err(error::unexpected_child_elem(child)),
        }
    }

    Ok(float)
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Opaque {
    A_ZERO,
    A_ONE,
    RGB_ZERO,
    RGB_ONE,
}

impl Opaque {
    fn rgb_transparency(self) -> bool {
        matches!(self, Self::RGB_ZERO | Self::RGB_ONE)
    }
    fn invert_transparency(self) -> bool {
        matches!(self, Self::RGB_ZERO | Self::A_ZERO)
    }
}

impl FromStr for Opaque {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "A_ZERO" => Self::A_ZERO,
            "A_ONE" => Self::A_ONE,
            "RGB_ZERO" => Self::RGB_ZERO,
            "RGB_ONE" => Self::RGB_ONE,
            _ => bail!("unknown opaque type {:?}", s),
        })
    }
}
