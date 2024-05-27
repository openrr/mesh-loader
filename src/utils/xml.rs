// A module that provides utilities for parsing and visiting XML nodes.

use std::{borrow::Cow, fmt, io, iter, marker::PhantomData, str::FromStr};

pub(crate) use roxmltree::*;

use super::{bytes::memchr_naive, float, int};

#[inline]
#[must_use]
pub(crate) const fn is_whitespace(c: char) -> bool {
    // https://www.w3.org/TR/xml/#NT-S
    // Note: Unlike is_ascii_whitespace, FORM FEED ('\x0C') is not included.
    matches!(c, '\t' | '\n' | '\r' | ' ')
}

#[inline]
pub(crate) fn trim(s: &str) -> &str {
    s.trim_matches(is_whitespace)
}
#[inline]
pub(crate) fn trim_start(s: &str) -> &str {
    s.trim_start_matches(is_whitespace)
}

// TODO: https://stackoverflow.com/questions/4325363/converting-a-number-with-comma-as-decimal-point-to-float
#[inline]
pub(crate) fn comma_to_period(s: &str) -> Cow<'_, str> {
    if s.as_bytes().contains(&b',') {
        s.replace(',', ".").into()
    } else {
        s.into()
    }
}

// -----------------------------------------------------------------------------
// Parsing array

/// Parses integer array "<int> <int> <int>...".
pub(crate) fn parse_int_array<T>(text: &str) -> ParseIntArray<'_, T>
where
    T: int::Integer,
{
    ParseIntArray {
        text,
        _marker: PhantomData,
    }
}

pub(crate) struct ParseIntArray<'a, T> {
    text: &'a str,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Iterator for ParseIntArray<'_, T>
where
    T: int::Integer,
{
    type Item = io::Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.text.is_empty() {
            return None;
        }
        match int::parse_partial(self.text.as_bytes()) {
            Some((value, n)) => {
                self.text = trim_start(self.text.get(n..).unwrap_or_default());
                Some(Ok(value))
            }
            None => Some(Err(format_err!("error while parsing an integer"))),
        }
    }
}

/*
/// Parses float array "<float> <float> <float>..."
pub(crate) fn parse_float_array<T>(text: &str) -> ParseFloatArray<'_, T>
where
    T: float::Float,
{
    ParseFloatArray {
        text,
        _marker: PhantomData,
    }
}

pub(crate) struct ParseFloatArray<'a, T> {
    text: &'a str,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Iterator for ParseFloatArray<'_, T>
where
    T: float::Float,
{
    type Item = io::Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.text.is_empty() {
            return None;
        }
        match float::parse_partial::<T>(self.text.as_bytes()) {
            Some((value, n)) => {
                self.text = trim_start(self.text.get(n..).unwrap_or_default());
                Some(Ok(value))
            }
            None => Some(Err(format_err!("error while parsing a float"))),
        }
    }
}
*/

/// Parses float array "<float> <float> <float>..."
pub(crate) fn parse_float_array_exact<T>(text: &str, num: usize) -> ParseFloatArrayExact<'_, T>
where
    T: float::Float,
{
    ParseFloatArrayExact {
        text,
        num,
        count: 0,
        _marker: PhantomData,
    }
}

pub(crate) struct ParseFloatArrayExact<'a, T> {
    text: &'a str,
    num: usize,
    count: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Iterator for ParseFloatArrayExact<'_, T>
where
    T: float::Float,
{
    type Item = io::Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count >= self.num {
            if self.text.is_empty() {
                return None;
            }
            return Some(Err(format_err!(
                "unexpected text {:?} after {} floats",
                self.text,
                self.num
            )));
        }
        match float::parse_partial::<T>(self.text.as_bytes()) {
            Some((value, n)) => {
                self.text = trim_start(self.text.get(n..).unwrap_or_default());
                self.count += 1;
                Some(Ok(value))
            }
            None => Some(Err(format_err!("error while parsing a float"))),
        }
    }
}

// -----------------------------------------------------------------------------
// XmlNodeExt

pub(crate) trait XmlNodeExt<'a, 'input> {
    fn element_children(&self) -> ElementChildren<'a, 'input>;
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
    fn trimmed_text(&self) -> &'a str;
    fn node_location(&self) -> TextPos;
    fn text_location(&self) -> TextPos;
    fn attr_value_location(&self, name: &str) -> TextPos;
}

impl<'a, 'input> XmlNodeExt<'a, 'input> for Node<'a, 'input> {
    fn element_children(&self) -> ElementChildren<'a, 'input> {
        self.children()
            .filter(|n| n.node_type() == NodeType::Element)
    }

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
                    self.attr_value_location(name),
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
                self.attr_value_location(name),
                v
            )
        })
    }

    fn trimmed_text(&self) -> &'a str {
        trim(self.text().unwrap_or_default())
    }

    #[cold]
    fn node_location(&self) -> TextPos {
        let start = self.range().start;
        self.document().text_pos_at(start)
    }
    #[cold]
    fn text_location(&self) -> TextPos {
        let mut start = self.range().start;
        start += memchr_naive(b'>', self.document().input_text()[start..].as_bytes())
            .map_or(0, |p| p + 1);
        self.document().text_pos_at(start)
    }
    #[cold]
    fn attr_value_location(&self, name: &str) -> TextPos {
        let start = self.attribute_node(name).unwrap().range_value().start;
        self.document().text_pos_at(start)
    }
}

pub(crate) type ElementChildren<'a, 'input> =
    iter::Filter<Children<'a, 'input>, fn(&Node<'a, 'input>) -> bool>;
