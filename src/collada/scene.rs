use super::*;

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=141
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Scene {
    pub instance_visual_scene: Option<InstanceVisualScene>,
}

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=91
#[derive(Debug)]
#[non_exhaustive]
pub struct InstanceVisualScene {
    /// The scoped identifier of this element.
    pub sid: Option<String>,
    /// The name of this element.
    pub name: Option<String>,
    /// The URI of the location of the [`VisualScene`] to instantiate.
    pub url: Uri<Node>,
}

new_key_type! {
    pub struct NodeIndex;
}

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=102
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct LibraryVisualScenes {
    /// The unique identifier of this element.
    pub id: Option<String>,
    /// The name of this element.
    pub name: Option<String>,

    // TODO: if we make this field read-only, we can change this to IndexMap.
    pub nodes: ArenaMap<String, Node, NodeIndex>,
}

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=119
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Node {
    /// The unique identifier of this element.
    pub id: Option<String>,
    /// The name of this element.
    pub name: Option<String>,
    /// The scoped identifier of this element.
    pub sid: Option<String>,
    /// The type of this element.
    pub ty: NodeType,

    pub parent: Option<NodeIndex>,
    pub children: Vec<NodeIndex>,

    // pub transforms: Vec<Transform<'a>>,
    // pub instance_camera: Vec<InstanceCamera>,
    // pub instance_controller: Vec<InstanceController>,
    pub instance_geometry: Vec<InstanceGeometry>,
    // pub instance_light: Vec<InstanceLight>,
    // pub instance_node: Vec<InstanceNode>,
}

/// The type of the [`Node`].
#[derive(Debug)]
#[non_exhaustive]
pub enum NodeType {
    Joint,
    Node,
}

impl Default for NodeType {
    fn default() -> Self {
        Self::Node
    }
}

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=85
#[derive(Debug)]
#[non_exhaustive]
pub struct InstanceGeometry {
    /// The scoped identifier of this element.
    pub sid: Option<String>,
    /// The name of this element.
    pub name: Option<String>,
    /// The URI of the location of the [`Geometry`] to instantiate.
    pub url: Uri<Geometry>,

    pub materials: IndexMap<String, SemanticMappingTable>,
}

/*
/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=82
#[derive(Debug)]
#[non_exhaustive]
pub struct InstanceController {
    /// The scoped identifier of this element.
    pub sid: Option<String>,
    /// The name of this element.
    pub name: Option<String>,
    /// The URI of the location of the [`Controller`] to instantiate.
    pub url: Uri<Controller>,

    pub materials: IndexMap<String, SemanticMappingTable>,
}
*/

#[derive(Debug)]
#[non_exhaustive]
pub struct SemanticMappingTable {
    // Required
    pub target: Uri<Material>,
    // Required
    pub symbol: String,

    pub map: HashMap<String, InputSemanticMapEntry>,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct InputSemanticMapEntry {
    pub input_semantic: InputSemantic,
    pub input_set: u32,
}

/*
/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=80
#[derive(Debug)]
#[non_exhaustive]
pub struct InstanceCamera {
    /// The scoped identifier of this element.
    pub sid: Option<String>,
    /// The name of this element.
    pub name: Option<String>,
    /// The URI of the location of the [`Camera`] to instantiate.
    pub url: Uri<Camera>,
}
*/

/*
/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=87
#[derive(Debug)]
#[non_exhaustive]
pub struct InstanceLight {
    /// The scoped identifier of this element.
    pub sid: Option<String>,
    /// The name of this element.
    pub name: Option<String>,
    /// The URI of the location of the [`Light`] to instantiate.
    pub url: Uri<Light>,
}
*/

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=89
#[derive(Debug)]
#[non_exhaustive]
pub struct InstanceNode {
    /// The scoped identifier of this element.
    pub sid: Option<String>,
    /// The name of this element.
    pub name: Option<String>,
    /// The URI of the location of the [`Node`] to instantiate.
    pub url: Uri<Node>,
}

// =============================================================================
// Parsing

pub(crate) fn parse_scene(_cx: &mut Context, node: xml::Node<'_, '_>) -> Result<Scene> {
    debug_assert_eq!(node.tag_name().name(), "scene");
    let mut instance_visual_scene = None;

    for child in node.element_children() {
        match child.tag_name().name() {
            "instance_visual_scene" => {
                instance_visual_scene = Some(parse_instance_visual_scene(child)?);
            }
            "instance_physics_scene" | "instance_kinematics_scene" => {
                warn::unsupported_child_elem(child)
            }
            "extra" => { /* skip */ }
            _ => error::unexpected_child_elem(child)?,
        }
    }

    Ok(Scene { instance_visual_scene })
}

fn parse_instance_visual_scene(node: xml::Node<'_, '_>) -> Result<InstanceVisualScene> {
    debug_assert_eq!(node.tag_name().name(), "instance_visual_scene");
    let url = node.parse_url("url")?;
    Ok(InstanceVisualScene {
        sid: node.attribute("sid").map(Into::into),
        name: node.attribute("name").map(Into::into),
        url,
    })
}

pub(crate) fn parse_library_visual_scenes(cx: &mut Context, node: xml::Node<'_, '_>) -> Result<()> {
    debug_assert_eq!(node.tag_name().name(), "library_visual_scenes");
    cx.library_visual_scenes.id = node.attribute("id").map(Into::into);
    cx.library_visual_scenes.name = node.attribute("name").map(Into::into);

    for child in node.element_children() {
        match child.tag_name().name() {
            "visual_scene" => {
                parse_visual_scene(child, &mut cx.library_visual_scenes.nodes)?;
            }
            "asset" | "extra" => { /* skip */ }
            _ => error::unexpected_child_elem(child)?,
        }
    }

    // if visual_scenes.is_empty() {
    //     error::one_or_more_elems(node, "visual_scene")?;
    // }

    Ok(())
}

fn parse_visual_scene(
    node: xml::Node<'_, '_>,
    nodes: &mut ArenaMap<String, Node, NodeIndex>,
) -> Result<()> {
    debug_assert_eq!(node.tag_name().name(), "visual_scene");
    // The specification say it is optional, but it is actually required.
    let id = node.required_attribute("id")?;
    let mut scene_nodes = vec![];
    let this = Node {
        id: Some(id.to_owned()),
        name: node.attribute("name").map(Into::into),
        ..Node::default()
    };
    let this = nodes.insert(id.to_owned(), this);

    for child in node.element_children() {
        match child.tag_name().name() {
            "node" => {
                scene_nodes.push(parse_node(child, nodes, this)?);
            }
            "evaluate_scene" => warn::unsupported_child_elem(child),
            "asset" | "extra" => { /* skip */ }
            _ => error::unexpected_child_elem(child)?,
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
fn parse_node(
    node: xml::Node<'_, '_>,
    nodes: &mut ArenaMap<String, Node, NodeIndex>,
    parent: NodeIndex,
) -> Result<NodeIndex> {
    debug_assert_eq!(node.tag_name().name(), "node");
    let ty = node.parse_attribute("type")?.unwrap_or_default();
    let this = Node {
        id: node.attribute("id").map(Into::into),
        name: node.attribute("name").map(Into::into),
        sid: node.attribute("sid").map(Into::into),
        ty,
        parent: Some(parent),
        ..Node::default()
    };
    let this = nodes.alloc(this);

    for child in node.element_children() {
        match child.tag_name().name() {
            "node" => {
                let c = parse_node(child, nodes, this)?;
                nodes[this].children.push(c);
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
                nodes[this].instance_geometry.push(parse_instance_geometry(child)?);
            }
            "instance_light" => {}
            "instance_node" => {}

            _ => {}
        }
    }
    // TODO

    Ok(this)
}

impl FromStr for NodeType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "NODE" => Self::Node,
            "JOINT" => Self::Joint,
            _ => bail!("unknown note type {:?}", s),
        })
    }
}

fn parse_instance_geometry(node: xml::Node<'_, '_>) -> Result<InstanceGeometry> {
    debug_assert_eq!(node.tag_name().name(), "instance_geometry");
    let url = node.parse_url("url")?;
    let mut materials = IndexMap::new();

    for child in node.element_children() {
        match child.tag_name().name() {
            "bind_material" => {
                parse_bind_material(child, &mut materials)?;
            }
            "extra" => { /* skip */ }
            _ => error::unexpected_child_elem(child)?,
        }
    }

    Ok(InstanceGeometry {
        sid: node.attribute("sid").map(Into::into),
        name: node.attribute("name").map(Into::into),
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
fn parse_bind_material(
    node: xml::Node<'_, '_>,
    materials: &mut IndexMap<String, SemanticMappingTable>,
) -> Result<()> {
    debug_assert_eq!(node.tag_name().name(), "bind_material");
    for child in node.element_children() {
        match child.tag_name().name() {
            "technique_common" => {
                for instance_mat_node in child.element_children() {
                    match instance_mat_node.tag_name().name() {
                        "instance_material" => {
                            let table = parse_instance_material(instance_mat_node)?;
                            materials.insert(table.symbol.clone(), table);
                        }
                        _ => error::unexpected_child_elem(instance_mat_node)?,
                    }
                }
            }
            "param" | "technique" | "extra" => { /* skip */ }
            _ => error::unexpected_child_elem(child)?,
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
fn parse_instance_material(node: xml::Node<'_, '_>) -> Result<SemanticMappingTable> {
    debug_assert_eq!(node.tag_name().name(), "instance_material");
    let target = node.parse_url("target")?;
    let symbol = node.required_attribute("symbol")?;
    let mut map = HashMap::new();

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

                let semantic = child.required_attribute("semantic")?;
                let input_semantic = child.parse_required_attribute("input_semantic")?;
                let input_set = child.parse_attribute("input_set")?.unwrap_or(0);

                map.insert(semantic.to_owned(), InputSemanticMapEntry {
                    input_semantic,
                    input_set,
                });
            }
            "bind" => warn::unsupported_child_elem(child),
            "extra" => { /* skip */ }
            _ => error::unexpected_child_elem(child)?,
        }
    }

    Ok(SemanticMappingTable { target, symbol: symbol.into(), map })
}
