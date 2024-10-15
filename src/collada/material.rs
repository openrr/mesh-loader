use super::*;

/// The `<library_materials>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=279
#[derive(Default)]
pub(super) struct LibraryMaterials<'a> {
    // /// The unique identifier of this element.
    // pub(super) id: Option<&'a str>,
    // /// The name of this element.
    // pub(super) name: Option<&'a str>,
    pub(super) materials: HashMap<&'a str, Material<'a>>,
}

/// The `<material>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=280
pub(super) struct Material<'a> {
    /// The unique identifier of this element.
    pub(super) id: &'a str,
    /// The name of this element.
    pub(super) name: Option<&'a str>,
    pub(super) instance_effect: InstanceEffect<'a>,
}

/// The `<instance_effect>` element.
///
/// See the [specification][1.4] for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=271
pub(super) struct InstanceEffect<'a> {
    // /// The scoped identifier of this element.
    // pub(super) sid: Option<&'a str>,
    // /// The name of this element.
    // pub(super) name: Option<&'a str>,
    /// The URI of the location of the [`Effect`] to instantiate.
    pub(super) url: Uri<'a, Effect<'a>>,
}

// -----------------------------------------------------------------------------
// Parsing

pub(super) fn parse_library_materials<'a>(
    cx: &mut Context<'a>,
    node: xml::Node<'a, '_>,
) -> io::Result<()> {
    debug_assert_eq!(node.tag_name().name(), "library_materials");
    // cx.library_materials.id = node.attribute("id");
    // cx.library_materials.name = node.attribute("name");

    for node in node.element_children() {
        match node.tag_name().name() {
            "material" => {
                let material = parse_material(node)?;
                cx.library_materials.materials.insert(material.id, material);
            }
            "asset" | "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(node)),
        }
    }

    // The specification says <library_materials> has 1 or more <material> elements,
    // but some exporters write empty <library_materials/> tags.

    Ok(())
}

fn parse_material<'a>(node: xml::Node<'a, '_>) -> io::Result<Material<'a>> {
    debug_assert_eq!(node.tag_name().name(), "material");
    // The specification say it is optional, but it is actually required.
    let id = node.required_attribute("id")?;
    let mut instance_effect = None;

    for node in node.element_children() {
        match node.tag_name().name() {
            "instance_effect" => {
                instance_effect = Some(parse_instance_effect(node)?);
            }
            "asset" | "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(node)),
        }
    }

    let instance_effect = match instance_effect {
        Some(instance_effect) => instance_effect,
        None => return Err(error::one_or_more_elems(node, "instance_effect")),
    };

    Ok(Material {
        id,
        name: node.attribute("name"),
        instance_effect,
    })
}

fn parse_instance_effect<'a>(node: xml::Node<'a, '_>) -> io::Result<InstanceEffect<'a>> {
    debug_assert_eq!(node.tag_name().name(), "instance_effect");
    let url = node.parse_url("url")?;
    Ok(InstanceEffect {
        // sid: node.attribute("sid"),
        // name: node.attribute("name"),
        url,
    })
}
