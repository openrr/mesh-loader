use super::*;

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=279
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct LibraryMaterials {
    /// The unique identifier of this element.
    pub id: Option<String>,
    /// The name of this element.
    pub name: Option<String>,

    pub materials: IndexMap<String, Material>,
}

/// See the [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=280
#[derive(Debug)]
#[non_exhaustive]
pub struct Material {
    /// The unique identifier of this element.
    pub id: String,
    /// The name of this element.
    pub name: Option<String>,

    pub instance_effect: InstanceEffect,
}

/// See the [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=271
#[derive(Debug)]
#[non_exhaustive]
pub struct InstanceEffect {
    /// The scoped identifier of this element.
    pub sid: Option<String>,
    /// The name of this element.
    pub name: Option<String>,
    /// The URI of the location of the [`Effect`] to instantiate.
    pub url: Uri<Effect>,
}

// =============================================================================
// Parsing

pub(crate) fn parse_library_materials(cx: &mut Context, node: xml::Node<'_, '_>) -> Result<()> {
    debug_assert_eq!(node.tag_name().name(), "library_materials");
    cx.library_materials.id = node.attribute("id").map(Into::into);
    cx.library_materials.name = node.attribute("name").map(Into::into);

    for node in node.element_children() {
        match node.tag_name().name() {
            "material" => {
                let material = parse_material(node)?;
                cx.library_materials.materials.insert(material.id.clone(), material);
            }
            "asset" | "extra" => { /* skip */ }
            _ => error::unexpected_child_elem(node)?,
        }
    }

    // The specification says <library_materials> has 1 or more <material> elements,
    // but some exporters write empty <library_materials/> tags.

    Ok(())
}

fn parse_material(node: xml::Node<'_, '_>) -> Result<Material> {
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
            _ => error::unexpected_child_elem(node)?,
        }
    }

    let instance_effect = match instance_effect {
        Some(instance_effect) => instance_effect,
        None => error::one_or_more_elems(node, "instance_effect")?,
    };

    Ok(Material { id: id.into(), name: node.attribute("name").map(Into::into), instance_effect })
}

fn parse_instance_effect(node: xml::Node<'_, '_>) -> Result<InstanceEffect> {
    debug_assert_eq!(node.tag_name().name(), "instance_effect");
    let url = node.parse_url("url")?;
    Ok(InstanceEffect {
        sid: node.attribute("sid").map(Into::into),
        name: node.attribute("name").map(Into::into),
        url,
    })
}
