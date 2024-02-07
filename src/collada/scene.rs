use super::*;

/// The `<scene>` element.
///
/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=141
#[derive(Default)]
pub(super) struct Scene<'a> {
    pub(super) instance_visual_scene: Option<InstanceVisualScene<'a>>,
}

/// The `<instance_visual_scene>` element.
///
/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=91
pub(super) struct InstanceVisualScene<'a> {
    // /// The scoped identifier of this element.
    // pub(super) sid: Option<&'a str>,
    // /// The name of this element.
    // pub(super) name: Option<&'a str>,
    /// The URI of the location of the [`VisualScene`] to instantiate.
    pub(super) url: Uri<'a, Node<'a>>,
}

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
    /// The unique identifier of this element.
    pub(super) id: Option<&'a str>,
    // /// The name of this element.
    // pub(super) name: Option<&'a str>,
    // /// The scoped identifier of this element.
    // pub(super) sid: Option<&'a str>,
    // /// The type of this element.
    // pub(super) ty: NodeType,
    pub(super) parent: Option<usize>,
    // pub(super) children: Vec<usize>,
    // pub(super) transforms: Vec<Transform>,
    pub(super) transform: Matrix4x4,
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

pub(super) enum Transform {
    Lookat([f32; 9]),
    Rotate([f32; 4]),
    Translate([f32; 3]),
    Scale([f32; 3]),
    Skew(#[allow(dead_code)] [f32; 7]),
    Matrix([f32; 16]),
}

impl Transform {
    // Based on https://github.com/assimp/assimp/blob/v5.3.1/code/AssetLib/Collada/ColladaParser.cpp#L2318
    fn calculate_transform(transforms: &[Self]) -> Matrix4x4 {
        fn sub(mut a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
            a[0] -= b[0];
            a[1] -= b[1];
            a[2] -= b[2];
            a
        }
        fn cross_product(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
            let mut r = [0.; 3];
            r[0] = a[1] * b[2] - a[2] * b[1];
            r[0] = a[2] * b[0] - a[0] * b[2];
            r[0] = a[0] * b[1] - a[1] * b[0];
            r
        }
        fn normalize(mut v: [f32; 3]) -> [f32; 3] {
            let square_len = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
            let len = square_len.sqrt();
            if len == 0. {
                return v;
            }
            let inv_len = 1. / len;
            v[0] /= inv_len;
            v[1] /= inv_len;
            v[2] /= inv_len;
            v
        }

        let mut out = None;
        for transform in transforms {
            match transform {
                Self::Lookat(f) => {
                    let pos = [f[0], f[1], f[2]];
                    let dst_pos = [f[3], f[4], f[5]];
                    let up = normalize([f[6], f[7], f[8]]);
                    let dir = normalize(sub(dst_pos, pos));
                    let right = normalize(cross_product(dir, up));
                    let m = Matrix4x4::new(
                        right[0], up[0], -dir[0], pos[0], right[1], up[1], -dir[1], pos[1],
                        right[2], up[2], -dir[2], pos[2], 0., 0., 0., 1.,
                    );
                    match &mut out {
                        Some(out) => *out *= m,
                        _ => out = Some(m),
                    }
                }
                Self::Rotate(f) => {
                    let angle = f[3] * std::f32::consts::PI / 180.;
                    let axis = [f[0], f[1], f[2]];
                    let m = Matrix4x4::rotation(angle, axis);
                    match &mut out {
                        Some(out) => *out *= m,
                        _ => out = Some(m),
                    }
                }
                Self::Translate(f) => {
                    let m = Matrix4x4::translation(*f);
                    match &mut out {
                        Some(out) => *out *= m,
                        _ => out = Some(m),
                    }
                }
                Self::Scale(f) => {
                    let m = Matrix4x4::new(
                        f[0], 0., 0., 0., 0., f[1], 0., 0., 0., 0., f[2], 0., 0., 0., 0., 1.,
                    );
                    match &mut out {
                        Some(out) => *out *= m,
                        _ => out = Some(m),
                    }
                }
                Self::Skew(_f) => {
                    // TODO
                }
                Self::Matrix(f) => {
                    let m = Matrix4x4::new(
                        f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8], f[9], f[10], f[11],
                        f[12], f[13], f[14], f[15],
                    );
                    match &mut out {
                        Some(out) => *out *= m,
                        _ => out = Some(m),
                    }
                }
            }
        }
        out.unwrap_or_default()
    }
}

// Based on https://github.com/assimp/assimp/blob/v5.3.1/include/assimp/matrix4x4.inl
#[derive(Clone, Copy)]
pub(super) struct Matrix4x4 {
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    c1: f32,
    c2: f32,
    c3: f32,
    c4: f32,
    d1: f32,
    d2: f32,
    d3: f32,
    d4: f32,
}
impl Matrix4x4 {
    pub(super) const fn new(
        a1: f32,
        a2: f32,
        a3: f32,
        a4: f32,
        b1: f32,
        b2: f32,
        b3: f32,
        b4: f32,
        c1: f32,
        c2: f32,
        c3: f32,
        c4: f32,
        d1: f32,
        d2: f32,
        d3: f32,
        d4: f32,
    ) -> Self {
        Self {
            a1,
            a2,
            a3,
            a4,
            b1,
            b2,
            b3,
            b4,
            c1,
            c2,
            c3,
            c4,
            d1,
            d2,
            d3,
            d4,
        }
    }
    fn rotation(a: f32, axis: [f32; 3]) -> Self {
        let c = a.cos();
        let s = a.sin();
        let t = 1. - c;
        let [x, y, z] = axis;
        Self::new(
            t * x * x + c,
            t * x * y - s * z,
            t * x * z + s * y,
            0.,
            t * x * y + s * z,
            t * y * y + c,
            t * y * z - s * x,
            0.,
            t * x * z - s * y,
            t * y * z + s * x,
            t * z * z + c,
            0.,
            0.,
            0.,
            0.,
            1.,
        )
    }
    fn translation(v: [f32; 3]) -> Self {
        Self {
            a4: v[0],
            b4: v[1],
            c4: v[2],
            ..Default::default()
        }
    }
    pub(super) fn is_identity(&self) -> bool {
        // TODO: use f32::EPSILON?
        const EPSILON: f32 = 10e-3;
        self.a2 <= EPSILON
            && self.a2 >= -EPSILON
            && self.a3 <= EPSILON
            && self.a3 >= -EPSILON
            && self.a4 <= EPSILON
            && self.a4 >= -EPSILON
            && self.b1 <= EPSILON
            && self.b1 >= -EPSILON
            && self.b3 <= EPSILON
            && self.b3 >= -EPSILON
            && self.b4 <= EPSILON
            && self.b4 >= -EPSILON
            && self.c1 <= EPSILON
            && self.c1 >= -EPSILON
            && self.c2 <= EPSILON
            && self.c2 >= -EPSILON
            && self.c4 <= EPSILON
            && self.c4 >= -EPSILON
            && self.d1 <= EPSILON
            && self.d1 >= -EPSILON
            && self.d2 <= EPSILON
            && self.d2 >= -EPSILON
            && self.d3 <= EPSILON
            && self.d3 >= -EPSILON
            && self.a1 <= 1. + EPSILON
            && self.a1 >= 1. - EPSILON
            && self.b2 <= 1. + EPSILON
            && self.b2 >= 1. - EPSILON
            && self.c3 <= 1. + EPSILON
            && self.c3 >= 1. - EPSILON
            && self.d4 <= 1. + EPSILON
            && self.d4 >= 1. - EPSILON
    }
}
impl Default for Matrix4x4 {
    fn default() -> Self {
        Self::new(
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        )
    }
}
impl ops::MulAssign<Matrix4x4> for [f32; 3] {
    fn mul_assign(&mut self, m: Matrix4x4) {
        let [x, y, z] = *self;
        self[0] = m.a1 * x + m.a2 * y + m.a3 * z;
        self[1] = m.b1 * x + m.b2 * y + m.b3 * z;
        self[2] = m.c1 * x + m.c2 * y + m.c3 * z;
    }
}
impl ops::MulAssign for Matrix4x4 {
    fn mul_assign(&mut self, m: Self) {
        let t = *self;
        self.a1 = m.a1 * t.a1 + m.b1 * t.a2 + m.c1 * t.a3 + m.d1 * t.a4;
        self.a2 = m.a2 * t.a1 + m.b2 * t.a2 + m.c2 * t.a3 + m.d2 * t.a4;
        self.a3 = m.a3 * t.a1 + m.b3 * t.a2 + m.c3 * t.a3 + m.d3 * t.a4;
        self.a4 = m.a4 * t.a1 + m.b4 * t.a2 + m.c4 * t.a3 + m.d4 * t.a4;
        self.b1 = m.a1 * t.b1 + m.b1 * t.b2 + m.c1 * t.b3 + m.d1 * t.b4;
        self.b2 = m.a2 * t.b1 + m.b2 * t.b2 + m.c2 * t.b3 + m.d2 * t.b4;
        self.b3 = m.a3 * t.b1 + m.b3 * t.b2 + m.c3 * t.b3 + m.d3 * t.b4;
        self.b4 = m.a4 * t.b1 + m.b4 * t.b2 + m.c4 * t.b3 + m.d4 * t.b4;
        self.c1 = m.a1 * t.c1 + m.b1 * t.c2 + m.c1 * t.c3 + m.d1 * t.c4;
        self.c2 = m.a2 * t.c1 + m.b2 * t.c2 + m.c2 * t.c3 + m.d2 * t.c4;
        self.c3 = m.a3 * t.c1 + m.b3 * t.c2 + m.c3 * t.c3 + m.d3 * t.c4;
        self.c4 = m.a4 * t.c1 + m.b4 * t.c2 + m.c4 * t.c3 + m.d4 * t.c4;
        self.d1 = m.a1 * t.d1 + m.b1 * t.d2 + m.c1 * t.d3 + m.d1 * t.d4;
        self.d2 = m.a2 * t.d1 + m.b2 * t.d2 + m.c2 * t.d3 + m.d2 * t.d4;
        self.d3 = m.a3 * t.d1 + m.b3 * t.d2 + m.c3 * t.d3 + m.d3 * t.d4;
        self.d4 = m.a4 * t.d1 + m.b4 * t.d2 + m.c4 * t.d3 + m.d4 * t.d4;
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

pub(super) fn parse_scene<'a>(
    _cx: &mut Context<'a>,
    node: xml::Node<'a, '_>,
) -> io::Result<Scene<'a>> {
    debug_assert_eq!(node.tag_name().name(), "scene");
    let mut instance_visual_scene = None;

    for child in node.element_children() {
        match child.tag_name().name() {
            "instance_visual_scene" => {
                instance_visual_scene = Some(parse_instance_visual_scene(child)?);
            }
            "instance_physics_scene" | "instance_kinematics_scene" => {
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

    Ok(Scene {
        instance_visual_scene,
    })
}

fn parse_instance_visual_scene<'a>(node: xml::Node<'a, '_>) -> io::Result<InstanceVisualScene<'a>> {
    debug_assert_eq!(node.tag_name().name(), "instance_visual_scene");
    let url = node.parse_url("url")?;
    Ok(InstanceVisualScene {
        // sid: node.attribute("sid"),
        // name: node.attribute("name"),
        url,
    })
}

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
    let id = node.attribute("id");
    let mut scene_nodes = vec![];
    let this = Node {
        id,
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
    parent: usize,
) -> io::Result<usize> {
    debug_assert_eq!(node.tag_name().name(), "node");
    let _ty: NodeType = node.parse_attribute("type")?.unwrap_or_default();
    let this = Node {
        // id: node.attribute("id"),
        // name: node.attribute("name"),
        // sid: node.attribute("sid"),
        // ty,
        parent: Some(parent),
        ..Default::default()
    };
    let this_index = nodes.len();
    nodes.push(this);
    let mut transforms = vec![];

    for child in node.element_children() {
        match child.tag_name().name() {
            "node" => {
                let _c = parse_node(child, nodes, this_index)?;
                // nodes[this_index].children.push(c);
            }

            // transformation
            "lookat" => {
                let content = xml::comma_to_period(child.trimmed_text());
                let mut iter = xml::parse_float_array_exact(&content, 9);
                let t = [
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                ];
                transforms.push(Transform::Lookat(t));
            }
            "matrix" => {
                let content = xml::comma_to_period(child.trimmed_text());
                let mut iter = xml::parse_float_array_exact(&content, 16);
                let t = [
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                ];
                transforms.push(Transform::Matrix(t));
            }
            "rotate" => {
                let content = xml::comma_to_period(child.trimmed_text());
                let mut iter = xml::parse_float_array_exact(&content, 4);
                let t = [
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                ];
                transforms.push(Transform::Rotate(t));
            }
            "scale" => {
                let content = xml::comma_to_period(child.trimmed_text());
                let mut iter = xml::parse_float_array_exact(&content, 3);
                let t = [
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                ];
                transforms.push(Transform::Scale(t));
            }
            "skew" => {
                let content = xml::comma_to_period(child.trimmed_text());
                let mut iter = xml::parse_float_array_exact(&content, 7);
                let t = [
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                ];
                transforms.push(Transform::Skew(t));
            }
            "translate" => {
                let content = xml::comma_to_period(child.trimmed_text());
                let mut iter = xml::parse_float_array_exact(&content, 3);
                let t = [
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                    iter.next().unwrap()?,
                ];
                transforms.push(Transform::Translate(t));
            }

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

    if !transforms.is_empty() {
        nodes[this_index].transform = Transform::calculate_transform(&transforms);
    }

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
