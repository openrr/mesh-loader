use super::*;

/// The `<library_images>` element.
///
/// See the specifications ([1.4], [1.5]) for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=278
/// [1.5]: https://www.khronos.org/files/collada_spec_1_5.pdf#page=327
#[derive(Default)]
pub(super) struct LibraryImages<'a> {
    // /// The unique identifier of this element.
    // pub(super) id: Option<&'a str>,
    // /// The name of this element.
    // pub(super) name: Option<&'a str>,
    pub(super) images: HashMap<&'a str, Image<'a>>,
}

/// The `<image>` element.
///
/// See the specifications ([1.4], [1.5]) for details.
///
/// [1.4]: https://www.khronos.org/files/collada_spec_1_4.pdf#page=268
/// [1.5]: https://www.khronos.org/files/collada_spec_1_5.pdf#page=310
pub(super) struct Image<'a> {
    /// The unique identifier of this element.
    pub(super) id: &'a str,
    // /// The name of this element.
    // pub(super) name: Option<&'a str>,
    // /// The image format.
    // pub(super) format: Option<&'a str>,
    // /// The height of the image in pixels.
    // pub(super) height: Option<u32>,
    // /// The width of the image in pixels.
    // pub(super) width: Option<u32>,
    // /// The depth of the image in pixels. A 2-D image has a depth of 1, which is the default.
    // pub(super) depth: u32,
    /// An embedded image data or an external image file.
    pub(super) source: ImageSource<'a>,
}

/// An embedded image data or an external image file.
pub(super) enum ImageSource<'a> {
    /// An embedded image data.
    Data(Vec<u8>),
    /// An external image file.
    InitFrom(&'a str),
    Skip,
}

// -----------------------------------------------------------------------------
// Parsing

pub(super) fn parse_library_images<'a>(
    cx: &mut Context<'a>,
    node: xml::Node<'a, '_>,
) -> io::Result<()> {
    debug_assert_eq!(node.tag_name().name(), "library_images");
    // cx.library_images.id = node.attribute("id");
    // cx.library_images.name = node.attribute("name");

    for node in node.element_children() {
        match node.tag_name().name() {
            "image" => {
                let image = parse_image(cx, node)?;
                cx.library_images.images.insert(image.id, image);
            }
            "asset" | "extra" => { /* skip */ }
            _ => return Err(error::unexpected_child_elem(node)),
        }
    }

    // The specification says <library_images> has 1 or more <image> elements,
    // but some exporters write empty <library_images/> tags.

    Ok(())
}

fn parse_image<'a>(cx: &Context<'a>, node: xml::Node<'a, '_>) -> io::Result<Image<'a>> {
    debug_assert_eq!(node.tag_name().name(), "image");
    let id = node.required_attribute("id")?;
    // let name = node.attribute("name");
    let is_1_4 = cx.version.is_1_4();
    if is_1_4 {
        // let mut format = node.attribute("format");
        let _height: Option<u32> = node.parse_attribute("height")?;
        let _width: Option<u32> = node.parse_attribute("width")?;
        let _depth: u32 = node.parse_attribute("depth")?.unwrap_or(1);
    } else {
        // let sid = node.attribute("sid");
    }
    let mut source = None;

    for node in node.element_children() {
        let tag_name = node.tag_name().name();
        match tag_name {
            "init_from" => {
                if is_1_4 {
                    source = Some(ImageSource::InitFrom(node.trimmed_text()));
                    continue;
                }
                for node in node.element_children() {
                    match node.tag_name().name() {
                        "ref" => {
                            source = Some(ImageSource::InitFrom(node.trimmed_text()));
                        }
                        "hex" => {
                            // format = node.attribute("format");
                            let data = hex::decode(node.trimmed_text().as_bytes())?;
                            source = Some(ImageSource::Data(data));
                        }
                        _ => {}
                    }
                }
            }
            "data" if is_1_4 => {
                let data = hex::decode(node.trimmed_text().as_bytes())?;
                source = Some(ImageSource::Data(data));
            }
            "asset" | "extra" => { /* skip */ }
            _ if is_1_4 => return Err(error::unexpected_child_elem(node)),
            _ => {}
        }
    }

    let source = match source {
        Some(source) => source,
        None => {
            if is_1_4 {
                bail!(
                    "<{}> element must be contain <data> or <init_from> element ({})",
                    node.tag_name().name(),
                    node.node_location()
                )
            }
            // 1.5 has <create_*> elements, but many applications ignore them.
            ImageSource::Skip
        }
    };

    Ok(Image {
        id,
        // name,
        // format,
        // height,
        // width,
        // depth,
        source,
    })
}
