use super::*;

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=278
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct LibraryImages {
    /// The unique identifier of this element.
    pub id: Option<String>,
    /// The name of this element.
    pub name: Option<String>,

    pub images: IndexMap<String, Image>,
}

/// See [specification][spec] for details.
///
/// [spec]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=268
#[derive(Debug)]
#[non_exhaustive]
pub struct Image {
    /// The unique identifier of this element.
    pub id: String,
    /// The name of this element.
    pub name: Option<String>,
    /// The image format.
    pub format: Option<String>,
    /// The height of the image in pixels.
    pub height: Option<u32>,
    /// The width of the image in pixels.
    pub width: Option<u32>,
    /// The depth of the image in pixels. A 2-D image has a depth of 1, which is the default.
    pub depth: u32,

    /// An embedded image data or an external image file.
    pub source: ImageSource,
}

impl Image {
    pub fn new(id: String, source: ImageSource) -> Self {
        Self { id, name: None, format: None, height: None, width: None, depth: 1, source }
    }
}

/// An embedded image data or an external image file.
#[derive(Debug)]
#[non_exhaustive]
pub enum ImageSource {
    /// An embedded image data.
    Data(Vec<u8>),
    /// An external image file.
    InitFrom(String),
}

// =============================================================================
// Parsing

pub(crate) fn parse_library_images(cx: &mut Context, node: xml::Node<'_, '_>) -> Result<()> {
    debug_assert_eq!(node.tag_name().name(), "library_images");
    cx.library_images.id = node.attribute("id").map(Into::into);
    cx.library_images.name = node.attribute("name").map(Into::into);

    for node in node.element_children() {
        match node.tag_name().name() {
            "image" => {
                let image = parse_image(cx, node)?;
                cx.library_images.images.insert(image.id.clone(), image);
            }
            "asset" | "extra" => { /* skip */ }
            _ => error::unexpected_child_elem(node)?,
        }
    }

    // The specification says <library_images> has 1 or more <image> elements,
    // but some exporters write empty <library_images/> tags.

    Ok(())
}

fn parse_image(_cx: &mut Context, node: xml::Node<'_, '_>) -> Result<Image> {
    debug_assert_eq!(node.tag_name().name(), "image");
    let id = node.required_attribute("id")?.into();
    let name = node.attribute("name").map(Into::into);
    let format = node.attribute("format").map(Into::into);
    let height = node.parse_attribute("height")?;
    let width = node.parse_attribute("width")?;
    let depth = node.parse_attribute("depth")?.unwrap_or(1);
    let mut source = None;

    for node in node.element_children() {
        match node.tag_name().name() {
            "data" => {
                let data = hex::decode(node.text().unwrap_or_default().trim().as_bytes())?;
                source = Some(ImageSource::Data(data));
            }
            "init_from" => {
                source =
                    Some(ImageSource::InitFrom(node.text().unwrap_or_default().trim().to_owned()));
            }
            "asset" | "extra" => { /* skip */ }
            _ => error::unexpected_child_elem(node)?,
        }
    }

    let source = match source {
        Some(source) => source,
        None => bail!(
            "<{}> element must be contain <data> or <init_from> element ({})",
            node.tag_name().name(),
            node.node_location()
        ),
    };

    Ok(Image { id, name, format, height, width, depth, source })
}
