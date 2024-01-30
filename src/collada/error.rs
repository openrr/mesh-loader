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
