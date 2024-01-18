use super::*;

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=99
#[derive(Default)]
pub(super) struct LibraryGeometries {
    /// The unique identifier of this element.
    pub(super) id: Option<String>,
    /// The name of this element.
    pub(super) name: Option<String>,

    pub(super) geometries: BTreeMap<String, Geometry>,

    pub(super) accessors: HashMap<String, Accessor>,
    pub(super) array_data: HashMap<String, ArrayData>,
}

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=68
pub(super) struct Geometry {
    /// The unique identifier of this element.
    pub(super) id: String,
    /// The name of this element.
    #[allow(dead_code)] // TODO
    pub(super) name: Option<String>,

    pub(super) mesh: Mesh,
}

pub(super) struct Mesh {
    pub(super) vertices: Vertices,
    pub(super) primitives: Vec<Primitive>,
}

pub(super) struct VerticesInputs {
    pub(super) position: UnsharedInput,
    pub(super) normal: Option<UnsharedInput>,
    pub(super) texcoord: Option<UnsharedInput>,
}

pub(super) struct Vertices {
    /// The unique identifier of this element.
    pub(super) id: String,
    /// The name of this element.
    #[allow(dead_code)] // TODO
    pub(super) name: Option<String>,

    pub(super) input: VerticesInputs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum PrimitiveType {
    /// The `<lines>` element.
    Lines,
    /// The `<linestrips>` element.
    LineStrips,
    /// The `<polygons>` element.
    Polygons,
    /// The `<polylist>` element.
    Polylist,
    /// The `<triangles>` element.
    Triangles,
    /// The `<trifans>` element.
    TriFans,
    /// The `<tristrips>` element.
    TriStrips,
}

impl PrimitiveType {
    pub(super) fn face_size(self) -> Option<u32> {
        match self {
            PrimitiveType::Lines | PrimitiveType::LineStrips => Some(2),
            PrimitiveType::Triangles | PrimitiveType::TriFans | PrimitiveType::TriStrips => Some(3),
            PrimitiveType::Polygons | PrimitiveType::Polylist => None,
        }
    }

    pub(super) fn min_face_size(self) -> u32 {
        self.face_size().unwrap_or(1)
    }
}

pub(super) struct PrimitiveInputs {
    pub(super) vertex: SharedInput<Vertices>,
    pub(super) normal: Option<SharedInput>,
    #[allow(dead_code)] // TODO(material)
    pub(super) color: Option<SharedInput>,
    pub(super) texcoord: Vec<SharedInput>,
}

pub(super) struct Primitive {
    /// The type of this element.
    pub(super) ty: PrimitiveType,

    /// The name of this element.
    #[allow(dead_code)] // TODO
    pub(super) name: Option<String>,
    /// The number of primitives.
    pub(super) count: u32,
    /// A symbol for a material.
    #[allow(dead_code)] // TODO(material)
    pub(super) material: Option<String>,

    /// Declares the input semantics of a data source and connects a consumer to that source.
    pub(super) input: Option<PrimitiveInputs>,
    /// The number of vertices for one polygon.
    ///
    /// Only [polylist] actually have a vcount element, but we use this field to
    /// represent the number of primitives other than [lines] and [triangles].
    ///
    /// The values included in this list are:
    ///
    /// - For [polylist] and [polygons]: `1 <= n`, and contains one polygon.
    /// - For [linestrips]: `2 <= n`, and contains `n - 1` lines.
    /// - For [tristrips] and [trifans]: `3 <= n`, and contains `n - 2` triangles.
    ///
    /// For [lines] and [triangles]: Since we know vcount of [lines] is always `vec![2; count]` and vcount of
    /// [triangles] is always `vec![3; count]`, this field is not used and is empty.
    ///
    /// [lines]: PrimitiveType::Lines
    /// [linestrips]: PrimitiveType::LineStrips
    /// [polylist]: PrimitiveType::Polylist
    /// [polygons]: PrimitiveType::Polygons
    /// [triangles]: PrimitiveType::Triangles
    /// [trifans]: PrimitiveType::TriFans
    /// [tristrips]: PrimitiveType::TriStrips
    pub(super) vcount: Vec<u32>,
    /// The vertex attributes (indices) for an individual primitive.
    pub(super) p: Vec<u32>,

    pub(super) stride: u32,
}

// -----------------------------------------------------------------------------
// Parsing

pub(super) fn parse_library_geometries(
    cx: &mut Context,
    node: xml::Node<'_, '_>,
) -> io::Result<()> {
    debug_assert_eq!(node.tag_name().name(), "library_geometries");
    cx.library_geometries.id = node.attribute("id").map(Into::into);
    cx.library_geometries.name = node.attribute("name").map(Into::into);

    for node in node.element_children() {
        match node.tag_name().name() {
            "geometry" => {
                if let Some(geometry) = parse_geometry(cx, node)? {
                    cx.library_geometries
                        .geometries
                        .insert(geometry.id.clone(), geometry);
                }
            }
            "asset" | "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(node)),
        }
    }

    if cx.library_geometries.geometries.is_empty() {
        return Err(error::one_or_more_elems(node, "geometry"));
    }

    Ok(())
}

fn parse_geometry(cx: &mut Context, node: xml::Node<'_, '_>) -> io::Result<Option<Geometry>> {
    debug_assert_eq!(node.tag_name().name(), "geometry");
    // The specification say it is optional, but it is actually required.
    let id = node.required_attribute("id")?;
    let mut mesh = None;

    for node in node.element_children() {
        match node.tag_name().name() {
            "mesh" => {
                mesh = Some(parse_mesh(cx, node)?);
            }
            "convex_mesh" | "spline" | "brep" => {
                warn::unsupported_child_elem(node);
                return Ok(None);
            }
            "asset" | "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(node)),
        }
    }

    let mesh = match mesh {
        Some(mesh) => mesh,
        None => return Err(error::one_or_more_elems(node, "mesh")),
    };

    Ok(Some(Geometry {
        id: id.into(),
        name: node.attribute("name").map(Into::into),
        mesh,
    }))
}

fn parse_mesh(cx: &mut Context, node: xml::Node<'_, '_>) -> io::Result<Mesh> {
    debug_assert_eq!(node.tag_name().name(), "mesh");
    let mut primitives = vec![];
    let mut has_source = false;
    let mut vertices = None;

    for node in node.element_children() {
        let name = node.tag_name().name();
        match name {
            "source" => {
                has_source = true;
                let s = Source::parse(node)?;
                if let Some(acc) = s.accessor {
                    cx.library_geometries.accessors.insert(s.id, acc);
                }
                if let Some(data) = s.array_element {
                    cx.library_geometries.array_data.insert(data.id, data.data);
                }
            }
            "vertices" => {
                vertices = Some(parse_vertices(node)?);
            }
            "lines" | "linestrips" | "polygons" | "polylist" | "triangles" | "trifans"
            | "tristrips" => {
                primitives.push(parse_primitive(node, name.parse().unwrap())?);
            }
            "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(node)),
        }
    }

    if !has_source {
        return Err(error::one_or_more_elems(node, "source"));
    }
    let vertices = match vertices {
        Some(vertices) => vertices,
        None => return Err(error::exactly_one_elem(node, "vertices")),
    };

    Ok(Mesh {
        vertices,
        primitives,
    })
}

fn parse_vertices(node: xml::Node<'_, '_>) -> io::Result<Vertices> {
    debug_assert_eq!(node.tag_name().name(), "vertices");
    let id = node.required_attribute("id")?;

    let mut input_position = None;
    let mut input_normal = None;
    let mut input_texcoord = None;

    for node in node.element_children() {
        match node.tag_name().name() {
            "input" => {
                let i = UnsharedInput::parse(node)?;
                match i.semantic {
                    InputSemantic::POSITION => input_position = Some(i),
                    InputSemantic::NORMAL => input_normal = Some(i),
                    InputSemantic::TEXCOORD => input_texcoord = Some(i),
                    _semantic => {
                        // warn!(
                        //     "unsupported semantic {:?} in <input> ({})",
                        //     semantic,
                        //     node.node_location(),
                        // );
                    }
                }
            }
            "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(node)),
        }
    }

    // One input must specify semantic="POSITION".
    let input_position = match input_position {
        Some(input_position) => input_position,
        None => return Err(error::one_or_more_elems(node, "input")),
    };

    Ok(Vertices {
        id: id.into(),
        name: node.attribute("name").map(Into::into),
        input: VerticesInputs {
            position: input_position,
            normal: input_normal,
            texcoord: input_texcoord,
        },
    })
}

impl FromStr for PrimitiveType {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "lines" => Self::Lines,
            "linestrips" => Self::LineStrips,
            "polygons" => Self::Polygons,
            "polylist" => Self::Polylist,
            "triangles" => Self::Triangles,
            "trifans" => Self::TriFans,
            "tristrips" => Self::TriStrips,
            _ => bail!("unknown primitive type {:?}", s),
        })
    }
}

fn parse_primitive(node: xml::Node<'_, '_>, ty: PrimitiveType) -> io::Result<Primitive> {
    debug_assert_eq!(node.tag_name().name().parse::<PrimitiveType>().unwrap(), ty);
    let count: u32 = node.parse_required_attribute("count")?;
    let mut vcount = vec![];
    let mut p = vec![];
    let mut stride = 0;

    let mut input_vertex = None;
    let mut input_normal = None;
    let mut input_color = None;
    let mut input_texcoord = vec![];

    for node in node.element_children() {
        match node.tag_name().name() {
            "input" => {
                let i = SharedInput::parse(node)?;
                stride = cmp::max(stride, i.offset + 1);
                match i.semantic {
                    InputSemantic::VERTEX => {
                        // ignore all position streams except 0 - there can be only one position
                        if i.set == 0 {
                            input_vertex = Some(i);
                        }
                    }
                    InputSemantic::NORMAL => {
                        // ignore all position streams except 0 - there can be only one position
                        if i.set == 0 {
                            input_normal = Some(i);
                        }
                    }
                    InputSemantic::COLOR => input_color = Some(i),
                    InputSemantic::TEXCOORD => input_texcoord.push(i),
                    _semantic => {
                        // warn!(
                        //     "unsupported semantic {:?} in <input> ({})",
                        //     semantic,
                        //     node.node_location(),
                        // );
                    }
                }
            }
            "vcount" => {
                // Only <polylist> has <vcount>.
                if ty != PrimitiveType::Polylist {
                    return Err(error::unexpected_child_elem(node));
                }
                if !vcount.is_empty() {
                    return Err(error::multiple_elems(node));
                }
                // It is possible to not contain any indices.
                if count == 0 {
                    continue;
                }

                vcount.reserve(count as _);

                let content = node.text().unwrap_or_default();
                let mut iter = xml::parse_int_array::<u32>(content);
                for _ in 0..count {
                    let value = iter.next().ok_or_else(|| {
                        format_err!(
                            "expected more values while reading <{}> \
                                 contents at {}",
                            node.tag_name().name(),
                            node.node_location()
                        )
                    })??;
                    if value >= 1 {
                        vcount.push(value);
                    } else {
                        bail!(
                            "incorrect number of indices in <p> element ({})",
                            node.node_location()
                        );
                    }
                }
            }
            "p" => {
                // It is possible to not contain any indices.
                if count == 0 {
                    continue;
                }

                if matches!(
                    ty,
                    PrimitiveType::Lines | PrimitiveType::Polylist | PrimitiveType::Triangles
                ) {
                    // For primitives with at most one <p> element,
                    // the length of indices can be pre-calculated.

                    if !p.is_empty() {
                        return Err(error::multiple_elems(node));
                    }

                    let mut expected_count = 0;
                    match ty {
                        PrimitiveType::Polylist => {
                            for &i in &vcount {
                                expected_count += i as usize;
                            }
                        }
                        PrimitiveType::Lines => {
                            expected_count = count as usize * 2;
                        }
                        PrimitiveType::Triangles => {
                            expected_count = count as usize * 3;
                        }
                        _ => unreachable!(),
                    }

                    p.reserve(expected_count * stride as usize);

                    // TODO: It seems some exporters put negative indices sometimes.
                    for value in xml::parse_int_array(node.text().unwrap_or_default()) {
                        p.push(value?);
                    }

                    if p.len() != expected_count * stride as usize {
                        // TODO: It seems SketchUp 15.3.331 writes the wrong 'count' for 'lines'.
                        bail!(
                            "incorrect index count in <p> element, expected {} but found {} ({})",
                            expected_count * stride as usize,
                            p.len(),
                            node.node_location()
                        );
                    }
                } else {
                    // For primitives that can have multiple <p> elements,
                    // One <p> element corresponds to one polygon.
                    // Therefore, we represent them in the same way as polylist.
                    // See the description of the `Primitive::vcount` field for more information.

                    if vcount.capacity() == 0 {
                        vcount.reserve(count as _);
                    }

                    let prev_len = p.len();

                    // TODO: It seems some exporters put negative indices sometimes.
                    for value in xml::parse_int_array(node.text().unwrap_or_default()) {
                        p.push(value?);
                    }

                    #[allow(clippy::cast_possible_truncation)]
                    let added = (p.len() - prev_len) as u32;
                    if added % stride != 0 {
                        bail!(
                            "incorrect index count in <p> element, expected multiple of {}, but found {} ({})",
                            stride,
                            p.len(),
                            node.node_location()
                        );
                    }
                    let vc = added / stride;
                    if vc >= ty.min_face_size() {
                        vcount.push(vc);
                    } else {
                        bail!(
                            "incorrect number of indices in <p> element ({})",
                            node.node_location()
                        );
                    }
                }
            }
            "ph" => warn::unsupported_child_elem(node),
            "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(node)),
        }
    }

    // When at least one input is present, one input must specify semantic="VERTEX".
    if input_vertex.is_none()
        && (input_normal.is_some() || input_color.is_some() || !input_texcoord.is_empty())
    {
        bail!(
            "one <input> in <{}> element must specify semantic=\"VERTEX\" ({})",
            node.tag_name().name(),
            node.node_location()
        );
    }
    // Attempt to respect the specified set.
    if !input_texcoord.is_empty() {
        input_texcoord.sort_by_key(|i| i.set);
    }

    Ok(Primitive {
        ty,
        name: node.attribute("name").map(Into::into),
        count,
        material: node.attribute("material").map(Into::into),
        input: input_vertex.map(|vertex| PrimitiveInputs {
            vertex: vertex.cast(),
            normal: input_normal,
            color: input_color,
            texcoord: input_texcoord,
        }),
        vcount,
        p,
        stride,
    })
}
