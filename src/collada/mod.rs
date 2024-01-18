//! [COLLADA] (.dae) parser.
//!
//! [COLLADA]: https://en.wikipedia.org/wiki/COLLADA

mod geometry;
mod instance;
mod iter;

use std::{
    cmp, collections::BTreeMap, collections::HashMap, fmt, io, marker::PhantomData, ops, str,
    str::FromStr,
};

use self::geometry::*;
use crate::{
    utils::xml::{self, XmlNodeExt},
    Scene,
};

/// Parses meshes from bytes of COLLADA text.
#[inline]
pub fn from_slice(bytes: &[u8]) -> io::Result<Scene> {
    from_str(str::from_utf8(bytes).map_err(crate::error::invalid_data)?)
}

/// Parses meshes from a string of COLLADA text.
#[inline]
pub fn from_str(s: &str) -> io::Result<Scene> {
    let xml = xml::Document::parse(s).map_err(crate::error::invalid_data)?;
    let collada = Document::parse(&xml)?;
    Ok(Scene {
        meshes: instance::build_meshes(&collada),
    })
}

// Inspired by gltf-json's `Get` trait.
/// Helper trait for retrieving top-level objects by a universal identifier.
trait Get<T> {
    type Target;

    fn get(&self, uri: &T) -> Option<&Self::Target>;
}

macro_rules! impl_get_by_uri {
    ($ty:ty, $($field:ident).*) => {
        impl Get<Uri<$ty>> for Document {
            type Target = $ty;

            fn get(&self, index: &Uri<$ty>) -> Option<&Self::Target> {
                self.$($field).*.get(&index.0)
            }
        }
    };
}

impl_get_by_uri!(Accessor, library_geometries.accessors);
impl_get_by_uri!(ArrayData, library_geometries.array_data);
impl_get_by_uri!(Geometry, library_geometries.geometries);

struct Uri<T>(String, PhantomData<fn() -> T>);

impl<T> Uri<T> {
    fn parse(url: &str) -> io::Result<Self> {
        // skipping the leading #, hopefully the remaining text is the accessor ID only
        if let Some(url) = url.strip_prefix('#') {
            Ok(Self(url.to_owned(), PhantomData))
        } else {
            Err(format_err!("unknown reference format {:?}", url))
        }
    }

    fn cast<U>(self) -> Uri<U> {
        Uri(self.0, PhantomData)
    }

    #[allow(dead_code)] // TODO(material)
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T> PartialEq for Uri<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Uri<T> {}

impl<T, S> PartialEq<S> for Uri<T>
where
    S: ?Sized + AsRef<str>,
{
    fn eq(&self, other: &S) -> bool {
        self.0 == other.as_ref()
    }
}

impl<T> PartialEq<Uri<T>> for str {
    #[inline]
    fn eq(&self, other: &Uri<T>) -> bool {
        self == other.0
    }
}

impl<T> PartialEq<Uri<T>> for String {
    #[inline]
    fn eq(&self, other: &Uri<T>) -> bool {
        *self == other.0
    }
}

trait ColladaXmlNodeExt<'a, 'input> {
    fn parse_url<T>(&self, name: &str) -> io::Result<Uri<T>>;
    fn parse_url_opt<T>(&self, name: &str) -> io::Result<Option<Uri<T>>>;
}

impl<'a, 'input> ColladaXmlNodeExt<'a, 'input> for xml::Node<'a, 'input> {
    fn parse_url<T>(&self, name: &str) -> io::Result<Uri<T>> {
        let url = self.required_attribute(name)?;
        Uri::parse(url).map_err(|e| {
            format_err!(
                "{} in {} attribute of <{}> element at {}",
                e,
                name,
                self.tag_name().name(),
                self.attr_location(name),
            )
        })
    }

    fn parse_url_opt<T>(&self, name: &str) -> io::Result<Option<Uri<T>>> {
        if let Some(url) = self.attribute(name) {
            Uri::parse(url).map(Some).map_err(|e| {
                format_err!(
                    "{} in {} attribute of <{}> element at {}",
                    e,
                    name,
                    self.tag_name().name(),
                    self.attr_location(name),
                )
            })
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    minor: u32,
    patch: u32,
}

impl Version {
    const MIN: Self = Self { minor: 4, patch: 0 };
}

impl FromStr for Version {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        (|| {
            let mut digits = s.splitn(3, '.');
            let major = digits.next()?;
            if major != "1" {
                return None;
            }
            let minor = digits.next()?.parse().ok()?;
            let patch = digits.next()?.parse().ok()?;
            Some(Self { minor, patch })
        })()
        .ok_or_else(|| format_err!("unrecognized version format {:?}", s))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "1.{}.{}", self.minor, self.patch)
    }
}

struct Context {
    library_geometries: LibraryGeometries,
}

struct Document {
    library_geometries: LibraryGeometries,
}

impl Document {
    /*
    The `<COLLADA>` element

    Attributes:
    - `version` (Required)
    - `xmlns` (xs:anyURI)
    - `base` (xs:anyURI)

    Child elements must appear in the following order if present:
    - `<asset>` (1)
    - library_element (0 or more)
        Any quantity and combination of any library elements can appear in any order:
        - `<library_animation_clips>`
        - `<library_animations>`
        - `<library_articulated_systems>` (in Kinematics)
        - `<library_cameras>`
        - `<library_controllers>`
        - `<library_effects>` (in FX)
        - `<library_force_fields>` (in Physics)
        - `<library_formulas>`
        - `<library_geometries>`
        - `<library_images>` (in FX)
        - `<library_joints>` (in Kinematics)
        - `<library_kinematics_models>` (in Kinematics)
        - `<library_kinematics_scenes>` (in Kinematics)
        - `<library_lights>`
        - `<library_materials>` (in FX)
        - `<library_nodes>`
        - `<library_physics_materials>` (in Physics)
        - `<library_physics_models>` (in Physics)
        - `<library_physics_scenes>` (in Physics)
        - `<library_visual_scenes>`
    - `<scene>` (0 or 1)
    - `<extra>` (0 or more)
    */
    fn parse(doc: &xml::Document<'_>) -> io::Result<Self> {
        let node = doc.root_element();
        if node.tag_name().name() != "COLLADA" {
            bail!("root element is not <COLLADA>");
        }

        let version: Version = node.required_attribute("version")?.parse()?;
        if version < Version::MIN {
            bail!("collada schema version {} is not supported", version);
        };
        // debug!("collada schema version is {}", version);

        let mut cx = Context {
            library_geometries: LibraryGeometries::default(),
        };

        for node in node.element_children() {
            match node.tag_name().name() {
                "library_geometries" => {
                    parse_library_geometries(&mut cx, node)?;
                }
                _name => {
                    // debug!("ignored <{}> element", name);
                }
            }
        }

        Ok(Self {
            library_geometries: cx.library_geometries,
        })
    }

    fn get<T>(&self, url: &T) -> Option<&<Self as Get<T>>::Target>
    where
        Self: Get<T>,
    {
        <Self as Get<T>>::get(self, url)
    }
}

impl<T> ops::Index<&T> for Document
where
    Self: Get<T>,
{
    type Output = <Self as Get<T>>::Target;

    #[track_caller]
    fn index(&self, url: &T) -> &Self::Output {
        self.get(url).expect("no entry found for key")
    }
}

struct Source {
    // Required
    id: String,
    // Optional
    #[allow(dead_code)]
    name: Option<String>,

    // 0 or 1
    array_element: Option<ArrayElement>,
    // 0 or 1
    accessor: Option<Accessor>,
}

impl Source {
    /*
    The `<source>` element (core)

    Attributes:
    - `id` (xs:ID, Required)
    - `name` (xs:token, Optional)

    Child elements must appear in the following order if present:
    - `<asset>` (0 or 1)
    - array_element (0 or 1)
        Can be one of:
        - `<bool_array>`
        - `<float_array>`
        - `<IDREF_array>`
        - `<int_array>`
        - `<Name_array>`
        - `<SIDREF_array>`
        - `<token_array>`
    - `<technique_common>` (0 or 1)
    - `<technique>` (core) (0 or more)
    */
    fn parse(node: xml::Node<'_, '_>) -> io::Result<Self> {
        debug_assert_eq!(node.tag_name().name(), "source");
        let id = node.required_attribute("id")?;
        let mut array_element = None;
        let mut accessor = None;

        for child in node.element_children() {
            match child.tag_name().name() {
                "float_array" | "IDREF_array" | "Name_array" => {
                    array_element = Some(parse_array_element(child)?);
                }
                "technique_common" => {
                    for technique in child.element_children() {
                        match technique.tag_name().name() {
                            "accessor" => {
                                accessor = Some(Accessor::parse(technique)?);
                            }
                            _ => return Err(error::unexpected_child_elem(technique)),
                        }
                    }
                }
                "bool_array" | "int_array" | "SIDREF_array" | "token_array" => {
                    // warn!(
                    //     "ignored array element {} ({})",
                    //     child.tag_name().name(),
                    //     child.node_location()
                    // );
                }
                "asset" | "technique" => { /* skip */ }
                _ => return Err(error::unexpected_child_elem(child)),
            }
        }

        Ok(Self {
            id: id.into(),
            name: node.attribute("name").map(Into::into),
            array_element,
            accessor,
        })
    }
}

struct ArrayElement {
    // Required
    id: String,
    // Required
    #[allow(dead_code)]
    count: u32,

    data: ArrayData,
}

fn parse_array_element(node: xml::Node<'_, '_>) -> io::Result<ArrayElement> {
    let name = node.tag_name().name();
    let is_string_array = name == "IDREF_array" || name == "Name_array";

    let id = node.required_attribute("id")?;
    let count = node.parse_required_attribute("count")?;
    let mut content = xml::trim(node.text().unwrap_or_default());

    // some exporters write empty data arrays, but we need to conserve them anyways because others might reference them
    if content.is_empty() {
        let data = if is_string_array {
            ArrayData::String(vec![])
        } else {
            ArrayData::Float(vec![])
        };
        return Ok(ArrayElement {
            id: id.into(),
            count,
            data,
        });
    }

    if is_string_array {
        // TODO: check large count
        let mut values = Vec::with_capacity(count as _);
        for _ in 0..count {
            if content.is_empty() {
                bail!(
                    "expected more values while reading <{}> contents at {}",
                    node.tag_name().name(),
                    node.node_location()
                );
            }

            let mut n = 0;
            while content
                .as_bytes()
                .first()
                .map_or(false, |&b| !xml::is_whitespace(b as char))
            {
                n += 1;
            }
            values.push(content[..n].into());

            content = xml::trim_start(content.get(n..).unwrap_or_default());
        }

        Ok(ArrayElement {
            id: id.into(),
            count,
            data: ArrayData::String(values),
        })
    } else {
        // TODO: check large count
        let mut values = Vec::with_capacity(count as _);
        // TODO: https://stackoverflow.com/questions/4325363/converting-a-number-with-comma-as-decimal-point-to-float
        let content = content.replace(',', ".");
        for res in xml::parse_float_array_exact(&content, count as _) {
            let value = res.map_err(|e| {
                format_err!(
                    "{} in <{}> element ({})",
                    e,
                    node.tag_name().name(),
                    node.node_location(),
                )
            })?;
            values.push(value);
        }

        Ok(ArrayElement {
            id: id.into(),
            count,
            data: ArrayData::Float(values),
        })
    }
}

/// Data source array.
enum ArrayData {
    /// <float_array>
    Float(Vec<f32>),
    /// <IDREF_array> or <Name_array>
    String(Vec<String>),
    // TODO(material)
    // /// <int_array>
    // Int(Vec<i32>),
    // /// <bool_array>
    // Bool(Vec<bool>),
}

#[allow(dead_code)] // TODO(material)
impl ArrayData {
    fn is_float(&self) -> bool {
        matches!(self, Self::Float(..))
    }

    fn is_string(&self) -> bool {
        matches!(self, Self::String(..))
    }

    fn as_float(&self) -> Option<&[f32]> {
        match self {
            Self::Float(v) => Some(v),
            Self::String(..) => None,
        }
    }

    fn as_string(&self) -> Option<&[String]> {
        match self {
            Self::Float(..) => None,
            Self::String(v) => Some(v),
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Float(v) => v.len(),
            Self::String(v) => v.len(),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::Float(v) => v.is_empty(),
            Self::String(v) => v.is_empty(),
        }
    }
}

struct Accessor {
    // Required
    count: u32,
    // Optional
    #[allow(dead_code)] // TODO(material)
    offset: u32,
    // Required
    source: Uri<ArrayData>,
    // Optional
    stride: u32,

    // 0 or more
    #[allow(dead_code)] // TODO(material)
    params: Vec<Param>,
}

impl Accessor {
    /*
    The `<accessor>` element

    Attributes:
    - `count` (uint_type, Required)
    - `offset` (uint_type, Optional, default: 0)
    - `source` (xs:anyURI, Required)
    - `stride` (uint_type, Optional, default: 1)

    Child elements:
    - `<param>` (0 or more)
    */
    fn parse(node: xml::Node<'_, '_>) -> io::Result<Self> {
        debug_assert_eq!(node.tag_name().name(), "accessor");
        let count = node.parse_required_attribute("count")?;
        let source = node.parse_url("source")?;
        let offset = node.parse_attribute("offset")?.unwrap_or(0);
        let stride = node.parse_attribute("stride")?.unwrap_or(1);
        let mut params = vec![];

        for child in node.element_children() {
            match child.tag_name().name() {
                "param" => {
                    params.push(Param::parse(child)?);
                }
                _ => return Err(error::unexpected_child_elem(child)),
            }
        }

        Ok(Self {
            count,
            offset,
            source,
            stride,
            params,
        })
    }
}

#[allow(dead_code)] // TODO(material)
struct Param {
    /// The name of this element.
    name: Option<String>,
    /// The scoped identifier of this element.
    sid: Option<String>,
    // Required
    ty: String,
    // Optional
    semantic: Option<String>,
}

impl Param {
    /*
    The `<param>` element (data flow)

    Attributes:
    - `name` (xs:token, Optional)
    - `sid` (sid_type, Optional)
    - `type` (xs:NMTOKEN, Required)
    - `semantic` (xs:NMTOKEN, Optional)
    */
    fn parse(node: xml::Node<'_, '_>) -> io::Result<Self> {
        let ty = node.required_attribute("type")?;
        let name = node.attribute("name");
        let sid = node.attribute("sid");
        let semantic = node.attribute("semantic");
        Ok(Self {
            name: name.map(Into::into),
            sid: sid.map(Into::into),
            ty: ty.into(),
            semantic: semantic.map(Into::into),
        })
    }
}

struct SharedInput<T = Accessor> {
    // Required
    offset: u32,
    // Required
    semantic: InputSemantic,
    // Required
    source: Uri<T>,
    // Optional
    set: u32,
}

impl<T> SharedInput<T> {
    /*
    The `<input>` element (shared)

    Attributes:
    - `offset` (uint_type, Required)
    - `semantic` (xs:NMTOKEN, Required)
    - `source` (uri_fragment_type, Required)
    - `set` (uint_type, Optional)
    */
    fn parse(node: xml::Node<'_, '_>) -> io::Result<Self> {
        debug_assert_eq!(node.tag_name().name(), "input");
        let semantic = node.parse_required_attribute("semantic")?;
        let source = node.parse_url("source")?;
        let offset = node.parse_required_attribute("offset")?;
        let set = node.parse_attribute("set")?.unwrap_or(0);
        Ok(Self {
            offset,
            semantic,
            source,
            set,
        })
    }

    fn cast<U>(self) -> SharedInput<U> {
        SharedInput {
            offset: self.offset,
            semantic: self.semantic,
            source: self.source.cast(),
            set: self.set,
        }
    }
}

struct UnsharedInput {
    // Required
    semantic: InputSemantic,
    // Required
    source: Uri<Accessor>,
}

impl UnsharedInput {
    /*
    The `<input>` element (unshared)

    Attributes:
    - `semantic` (xs:NMTOKEN, Required)
    - `source` (uri_fragment_type, Required)
    */
    fn parse(node: xml::Node<'_, '_>) -> io::Result<Self> {
        debug_assert_eq!(node.tag_name().name(), "input");
        let semantic = node.parse_required_attribute("semantic")?;
        let source = node.parse_url("source")?;
        Ok(Self { semantic, source })
    }
}

// refs: https://www.khronos.org/files/collada_spec_1_4.pdf#page=74
// refs: https://www.khronos.org/files/collada_spec_1_5.pdf#page=88
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum InputSemantic {
    /// Geometric binormal (bitangent) vector.
    BINORMAL,
    /// Color coordinate vector. Color inputs are RGB (float3_type).
    COLOR,
    /// Continuity constraint at the control vertex (CV).
    CONTINUITY,
    /// Raster or MIP-level input.
    IMAGE,
    /// Sampler input.
    INPUT,
    /// Tangent vector for preceding control point.
    IN_TANGENT,
    /// Sampler interpolation type.
    INTERPOLATION,
    /// Inverse of local-to-world matrix.
    INV_BIND_MATRIX,
    /// Skin influence identifier.
    JOINT,
    /// Number of piece-wise linear approximation steps to use for the spline segment that follows this CV.
    LINEAR_STEPS,
    /// Morph targets for mesh morphing.
    MORPH_TARGET,
    /// Weights for mesh morphing
    MORPH_WEIGHT,
    /// Normal vector
    NORMAL,
    /// Sampler output.
    OUTPUT,
    /// Tangent vector for succeeding control point.
    OUT_TANGENT,
    /// Geometric coordinate vector.
    POSITION,
    /// Geometric tangent vector.
    TANGENT,
    /// Texture binormal (bitangent) vector.
    TEXBINORMAL,
    /// Texture coordinate vector.
    TEXCOORD,
    /// Texture tangent vector.
    TEXTANGENT,
    /// Generic parameter vector.
    UV,
    /// Mesh vertex.
    VERTEX,
    /// Skin influence weighting value.
    WEIGHT,
}

impl FromStr for InputSemantic {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "BINORMAL" => Self::BINORMAL,
            "COLOR" => Self::COLOR,
            "CONTINUITY" => Self::CONTINUITY,
            "IMAGE" => Self::IMAGE,
            "INPUT" => Self::INPUT,
            "IN_TANGENT" => Self::IN_TANGENT,
            "INTERPOLATION" => Self::INTERPOLATION,
            "INV_BIND_MATRIX" => Self::INV_BIND_MATRIX,
            "JOINT" => Self::JOINT,
            "LINEAR_STEPS" => Self::LINEAR_STEPS,
            "MORPH_TARGET" => Self::MORPH_TARGET,
            "MORPH_WEIGHT" => Self::MORPH_WEIGHT,
            "NORMAL" => Self::NORMAL,
            "OUTPUT" => Self::OUTPUT,
            "OUT_TANGENT" => Self::OUT_TANGENT,
            "POSITION" => Self::POSITION,
            "TANGENT" => Self::TANGENT,
            "TEXBINORMAL" => Self::TEXBINORMAL,
            "TEXCOORD" => Self::TEXCOORD,
            "TEXTANGENT" => Self::TEXTANGENT,
            "UV" => Self::UV,
            "VERTEX" => Self::VERTEX,
            "WEIGHT" => Self::WEIGHT,
            _ => bail!("unknown input semantic {:?}", s),
        })
    }
}

mod error {
    use super::*;

    #[cold]
    pub(super) fn one_or_more_elems(node: xml::Node<'_, '_>, name: &str) -> io::Error {
        format_err!(
            "<{}> element must be contain one or more <{}> elements ({})",
            node.tag_name().name(),
            name,
            node.node_location()
        )
    }

    #[cold]
    pub(super) fn exactly_one_elem(node: xml::Node<'_, '_>, name: &str) -> io::Error {
        format_err!(
            "<{}> element must be contain exactly one <{}> element ({})",
            node.tag_name().name(),
            name,
            node.node_location()
        )
    }

    #[cold]
    pub(super) fn multiple_elems(node: xml::Node<'_, '_>) -> io::Error {
        format_err!(
            "multiple <{}> elements ({})",
            node.tag_name().name(),
            node.node_location()
        )
    }

    #[cold]
    pub(super) fn unexpected_child_elem(child: xml::Node<'_, '_>) -> io::Error {
        format_err!(
            "unexpected child element <{}> in <{}> element ({})",
            child.tag_name().name(),
            child.parent_element().unwrap().tag_name().name(),
            child.node_location()
        )
    }
}

mod warn {
    use super::*;

    #[cold]
    pub(super) fn unsupported_child_elem(_child: xml::Node<'_, '_>) {
        // warn!(
        //     "<{}> child element in <{}> element is unsupported ({})",
        //     child.tag_name().name(),
        //     child.parent_element().unwrap().tag_name().name(),
        //     child.node_location()
        // );
    }
}
