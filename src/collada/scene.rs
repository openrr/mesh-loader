use super::*;

/// The `<library_visual_scenes>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=102
#[derive(Default)]
pub(super) struct LibraryVisualScenes<'a> {
    // /// The unique identifier of this element.
    // pub(super) id: Option<&'a str>,
    // /// The name of this element.
    // pub(super) name: Option<&'a str>,
    pub(super) nodes: Vec<Node<'a>>,
}

/// The `<node>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=119
#[derive(Default)]
pub(super) struct Node<'a> {
    // /// The unique identifier of this element.
    // pub(super) id: Option<&'a str>,
    // /// The name of this element.
    // pub(super) name: Option<&'a str>,
    // /// The scoped identifier of this element.
    // pub(super) sid: Option<&'a str>,
    // /// The type of this element.
    // pub(super) ty: NodeType,

    // pub(super) parent: Option<usize>,
    // pub(super) children: Vec<usize>,

    // pub(super) transforms: Vec<Transform<'a>>,
    // pub(super) instance_camera: Vec<InstanceCamera>,
    // pub(super) instance_controller: Vec<InstanceController>,
    pub(super) instance_geometry: Vec<InstanceGeometry<'a>>,
    // pub(super) instance_light: Vec<InstanceLight>,
    // pub(super) instance_node: Vec<InstanceNode>,
}

/// The type of the [`Node`].
#[derive(Debug)]
pub(super) enum NodeType {
    Joint,
    Node,
}

impl Default for NodeType {
    fn default() -> Self {
        Self::Node
    }
}

/// The `<instance_geometry>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=85
pub(super) struct InstanceGeometry<'a> {
    // /// The scoped identifier of this element.
    // pub(super) sid: Option<&'a str>,
    // /// The name of this element.
    // pub(super) name: Option<&'a str>,
    /// The URI of the location of the [`Geometry`] to instantiate.
    pub(super) url: Uri<'a, Geometry<'a>>,

    pub(super) materials: BTreeMap<&'a str, SemanticMappingTable<'a>>,
}

/*
/// The `<instance_controller>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=82
pub(super) struct InstanceController<'a> {
    /// The scoped identifier of this element.
    pub(super) sid: Option<&'a str>,
    /// The name of this element.
    pub(super) name: Option<&'a str>,
    /// The URI of the location of the [`Controller`] to instantiate.
    pub(super) url: Uri<Controller>,

    pub(super) materials: IndexMap<&'a str, SemanticMappingTable>,
}
*/

pub(super) struct SemanticMappingTable<'a> {
    // Required
    pub(super) target: Uri<'a, Material<'a>>,
    // Required
    pub(super) symbol: &'a str,
    // pub(super) map: HashMap<&'a str, InputSemanticMapEntry>,
}

// pub(super) struct InputSemanticMapEntry {
//     pub(super) input_semantic: InputSemantic,
//     pub(super) input_set: u32,
// }

/*
/// The `<instance_camera>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=80
#[derive(Debug)]
#[non_exhaustive]
pub struct InstanceCamera<'a> {
    /// The scoped identifier of this element.
    pub sid: Option<&'a str>,
    /// The name of this element.
    pub name: Option<&'a str>,
    /// The URI of the location of the [`Camera`] to instantiate.
    pub url: Uri<Camera<'a>>,
}

/// The `<instance_light>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=87
#[derive(Debug)]
#[non_exhaustive]
pub struct InstanceLight<'a> {
    /// The scoped identifier of this element.
    pub sid: Option<&'a str>,
    /// The name of this element.
    pub name: Option<&'a str>,
    /// The URI of the location of the [`Light`] to instantiate.
    pub url: Uri<Light<'a>>,
}

/// The `<instance_node>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=89
pub(super) struct InstanceNode<'a> {
    /// The scoped identifier of this element.
    pub(super) sid: Option<&'a str>,
    /// The name of this element.
    pub(super) name: Option<&'a str>,
    /// The URI of the location of the [`Node`] to instantiate.
    pub(super) url: Uri<'a, Node<'a>>,
}
*/

// =============================================================================
// Parsing

pub(super) fn parse_library_visual_scenes<'a>(
    cx: &mut Context<'a>,
    node: xml::Node<'a, '_>,
) -> io::Result<()> {
    debug_assert_eq!(node.tag_name().name(), "library_visual_scenes");
    // cx.library_visual_scenes.id = node.attribute("id");
    // cx.library_visual_scenes.name = node.attribute("name");

    for child in node.element_children() {
        match child.tag_name().name() {
            "visual_scene" => {
                parse_visual_scene(child, &mut cx.library_visual_scenes.nodes)?;
            }
            "asset" | "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(child)),
        }
    }

    // if visual_scenes.is_empty() {
    //     error::one_or_more_elems(node, "visual_scene")?;
    // }

    Ok(())
}

fn parse_visual_scene<'a>(node: xml::Node<'a, '_>, nodes: &mut Vec<Node<'a>>) -> io::Result<()> {
    debug_assert_eq!(node.tag_name().name(), "visual_scene");
    // The specification say it is optional, but it is actually required.
    let _id = node.required_attribute("id")?;
    let mut scene_nodes = vec![];
    let this = Node {
        // id: Some(id),
        // name: node.attribute("name"),
        ..Default::default()
    };
    let this_index = nodes.len();
    nodes.push(this);

    for child in node.element_children() {
        match child.tag_name().name() {
            "node" => {
                scene_nodes.push(parse_node(child, nodes, this_index)?);
            }
            "evaluate_scene" => {
                // warn!(
                //     "<{}> child element in <{}> element is unsupported ({})",
                //     child.tag_name().name(),
                //     child.parent_element().unwrap().tag_name().name(),
                //     child.node_location()
                // );
            }
            "asset" | "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(child)),
        }
    }

    Ok(())
}

/*
The `<node>` element

Attributes:
- `id` (xs:ID, Optional)
- `name` (xs:token, Optional)
- `sid` (sid_type, Optional)
- `type` (Enumeration, Optional)
    The type of the <node> element. Valid values are JOINT or NODE.
    The default is NODE.
- `layer` (list_of_names_type, Optional)

Child elements must appear in the following order if present:
- `<asset>` (0 or 1)
- transformation_elements (0 or more )
    Any combination of the following transformation elements:
    - `<lookat>`
    - `<matrix>`
    - `<rotate>`
    - `<scale>`
    - `<skew>`
    - `<translate>`
- `<instance_camera>` (0 or more)
- `<instance_controller>` (0 or more)
- `<instance_geometry>` (0 or more)
- `<instance_light>` (0 or more)
- `<instance_node>` (0 or more)
- `<node>` (0 or more)
- `<extra>` (0 or more)
*/
fn parse_node<'a>(
    node: xml::Node<'a, '_>,
    nodes: &mut Vec<Node<'a>>,
    _parent: usize,
) -> io::Result<usize> {
    debug_assert_eq!(node.tag_name().name(), "node");
    let _ty: NodeType = node.parse_attribute("type")?.unwrap_or_default();
    let this = Node {
        // id: node.attribute("id"),
        // name: node.attribute("name"),
        // sid: node.attribute("sid"),
        // ty,
        // parent: Some(parent),
        ..Default::default()
    };
    let this_index = nodes.len();
    nodes.push(this);

    for child in node.element_children() {
        match child.tag_name().name() {
            "node" => {
                let _c = parse_node(child, nodes, this_index)?;
                // nodes[this_index].children.push(c);
            }

            // transformation
            "lookat" => {}
            "matrix" => {}
            "rotate" => {}
            "scale" => {}
            "skew" => {}
            "translate" => {}

            // instances
            "instance_camera" => {}
            "instance_controller" => {}
            "instance_geometry" => {
                nodes[this_index]
                    .instance_geometry
                    .push(parse_instance_geometry(child)?);
            }
            "instance_light" => {}
            "instance_node" => {}

            _ => {}
        }
    }
    // TODO

    Ok(this_index)
}

impl FromStr for NodeType {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "NODE" => Self::Node,
            "JOINT" => Self::Joint,
            _ => bail!("unknown note type {:?}", s),
        })
    }
}

fn parse_instance_geometry<'a>(node: xml::Node<'a, '_>) -> io::Result<InstanceGeometry<'a>> {
    debug_assert_eq!(node.tag_name().name(), "instance_geometry");
    let url = node.parse_url("url")?;
    let mut materials = BTreeMap::new();

    for child in node.element_children() {
        match child.tag_name().name() {
            "bind_material" => {
                parse_bind_material(child, &mut materials)?;
            }
            "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(child)),
        }
    }

    Ok(InstanceGeometry {
        // sid: node.attribute("sid"),
        // name: node.attribute("name"),
        url,
        materials,
    })
}

/*
The <bind_material> element

Child elements must appear in the following order if present:
- `<param>` (core) (0 or more)
- `<technique_common>` (1)
- `<technique>` (core) (0 or more)
- `<extra>` (0 or more)

Child Elements for <bind_material> / <technique_common>
- `<instance_material>` (geometry) (1 or more)
*/
fn parse_bind_material<'a>(
    node: xml::Node<'a, '_>,
    materials: &mut BTreeMap<&'a str, SemanticMappingTable<'a>>,
) -> io::Result<()> {
    debug_assert_eq!(node.tag_name().name(), "bind_material");
    for child in node.element_children() {
        match child.tag_name().name() {
            "technique_common" => {
                for instance_mat_node in child.element_children() {
                    match instance_mat_node.tag_name().name() {
                        "instance_material" => {
                            let table = parse_instance_material(instance_mat_node)?;
                            materials.insert(table.symbol, table);
                        }
                        _ => return Err(error::unexpected_child_elem(instance_mat_node)),
                    }
                }
            }
            "param" | "technique" | "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(child)),
        }
        // TODO
    }
    Ok(())
}

/*
The <instance_material> element (geometry)

Attributes:
- `sid` (sid_type, Optional)
- `name` (xs:token, Optional)
- `target` (xs:anyURI, Required)
- `symbol` (xs:NCName, Required)

Child elements must appear in the following order if present:
- `<bind>` (FX) (0 or more)
- `<bind_vertex_input>` (0 or more)
- `<extra>` (0 or more)
*/
fn parse_instance_material<'a>(node: xml::Node<'a, '_>) -> io::Result<SemanticMappingTable<'a>> {
    debug_assert_eq!(node.tag_name().name(), "instance_material");
    let target = node.parse_url("target")?;
    let symbol = node.required_attribute("symbol")?;
    // let mut map = HashMap::new();

    for child in node.element_children() {
        match child.tag_name().name() {
            "bind_vertex_input" => {
                /*
                The <bind_vertex_input> element

                Attributes:
                - `semantic` (xs:NCName, Required)
                - `input_semantic` (xs:NCName, Required)
                - `input_set` (uint_type, Optional)
                */

                let _semantic = child.required_attribute("semantic")?;
                let _input_semantic: InputSemantic =
                    child.parse_required_attribute("input_semantic")?;
                let _input_set: u32 = child.parse_attribute("input_set")?.unwrap_or(0);

                // map.insert(
                //     semantic,
                //     InputSemanticMapEntry {
                //         input_semantic,
                //         input_set,
                //     },
                // );
            }
            "bind" => {
                // warn!(
                //     "<{}> child element in <{}> element is unsupported ({})",
                //     child.tag_name().name(),
                //     child.parent_element().unwrap().tag_name().name(),
                //     child.node_location()
                // );
            }
            "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(child)),
        }
    }

    Ok(SemanticMappingTable {
        target,
        symbol,
        // map,
    })
}
