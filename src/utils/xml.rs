// A module that provides utilities for parsing and visiting XML nodes.

use std::{fmt, io, iter, str::FromStr};

pub(crate) use roxmltree::*;

pub(crate) trait XmlNodeExt<'a, 'input> {
    fn element_children(&self) -> ElementChildren<'a, 'input>;
    // fn matches_children<'b>(
    //     &self,
    //     name: &'b str,
    // ) -> MatchesChildren<'a, 'b, 'input>;
    fn child(&self, name: &str) -> Option<Node<'a, 'input>>;
    fn required_attribute(&self, name: &str) -> io::Result<&'a str>;
    fn parse_attribute<T>(&self, name: &str) -> io::Result<Option<T>>
    where
        T: FromStr,
        T::Err: fmt::Display;
    fn parse_required_attribute<T>(&self, name: &str) -> io::Result<T>
    where
        T: FromStr,
        T::Err: fmt::Display;
    fn node_location(&self) -> TextPos;
    fn attr_location(&self, name: &str) -> TextPos;
}

impl<'a, 'input> XmlNodeExt<'a, 'input> for Node<'a, 'input> {
    fn element_children(&self) -> ElementChildren<'a, 'input> {
        self.children()
            .filter(|n| n.node_type() == NodeType::Element)
    }

    // fn matches_children<'b>(
    //     &self,
    //     name: &'b str,
    // ) -> MatchesChildren<'a, 'b, 'input> {
    //     MatchesChildren { iter: self.children(), name }
    // }

    fn child(&self, name: &str) -> Option<Node<'a, 'input>> {
        self.element_children()
            .find(|n| n.tag_name().name() == name)
    }

    fn required_attribute(&self, name: &str) -> io::Result<&'a str> {
        match self.attribute(name) {
            Some(v) => Ok(v),
            None => {
                bail!(
                    "expected {} attribute in <{}> element at {}",
                    name,
                    if self.is_element() {
                        self.tag_name().name()
                    } else {
                        self.parent_element().unwrap().tag_name().name()
                    },
                    self.node_location(),
                )
            }
        }
    }

    fn parse_attribute<T>(&self, name: &str) -> io::Result<Option<T>>
    where
        T: FromStr,
        T::Err: fmt::Display,
    {
        match self.attribute(name) {
            Some(v) => Ok(Some(v.parse::<T>().map_err(|e| {
                format_err!(
                    "{} in <{}> element at {}: {:?}",
                    e,
                    self.tag_name().name(),
                    self.attr_location(name),
                    v
                )
            })?)),
            None => Ok(None),
        }
    }

    fn parse_required_attribute<T>(&self, name: &str) -> io::Result<T>
    where
        T: FromStr,
        T::Err: fmt::Display,
    {
        let v = self.required_attribute(name)?;
        v.parse::<T>().map_err(|e| {
            format_err!(
                "{} in <{}> element at {}: {:?}",
                e,
                self.tag_name().name(),
                self.attr_location(name),
                v
            )
        })
    }

    #[cold]
    fn node_location(&self) -> TextPos {
        let range = self.range();
        self.document().text_pos_at(range.start)
    }

    #[cold]
    fn attr_location(&self, name: &str) -> TextPos {
        let start = self.attribute_node(name).unwrap().position();
        self.document().text_pos_at(start)
    }
}

pub(crate) type ElementChildren<'a, 'input> =
    iter::Filter<Children<'a, 'input>, fn(&Node<'a, 'input>) -> bool>;

pub(crate) struct MatchesChildren<'a, 'b, 'input> {
    iter: Children<'a, 'input>,
    name: &'b str,
}

impl<'a, 'input> Iterator for MatchesChildren<'a, '_, 'input> {
    type Item = Node<'a, 'input>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .find(|&n| n.is_element() && n.has_tag_name(self.name))
    }
}
