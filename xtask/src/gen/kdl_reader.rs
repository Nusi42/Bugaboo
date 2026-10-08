//! A reader for KDL documents that says where a problem is: the one place the KDL crate is used.
//!
//! `design/theme.kdl` is read only here and only by xtask; the server and the browser never parse
//! KDL (ADR-0026). Every reader of the file asks for a [`Node`] and its words, and every problem it
//! reports names the line.

use anyhow::{Context as _, anyhow};
use kdl::{KdlDocument, KdlNode, KdlValue};

/// A parsed document and its text, so that a node can say which line it is on.
pub(crate) struct Document {
    text: String,
    parsed: KdlDocument,
}

impl Document {
    pub(crate) fn parse(text: &str) -> anyhow::Result<Self> {
        let parsed: KdlDocument = text.parse().map_err(|error| anyhow!("{error}"))?;
        Ok(Self {
            text: text.to_owned(),
            parsed,
        })
    }

    /// The nodes of the top level, in file order.
    pub(crate) fn nodes(&self) -> Vec<Node<'_>> {
        nodes_of(&self.parsed, &self.text)
    }
}

fn nodes_of<'a>(document: &'a KdlDocument, text: &'a str) -> Vec<Node<'a>> {
    document
        .nodes()
        .iter()
        .map(|raw| Node { raw, text })
        .collect()
}

/// One node: its name, its words, its `key=value` pairs and its children.
#[derive(Clone, Copy)]
pub(crate) struct Node<'a> {
    raw: &'a KdlNode,
    text: &'a str,
}

/// The text of a value that is a word or a number; `None` for `#true`, `#null` and the like.
fn scalar(value: &KdlValue) -> Option<String> {
    if let Some(text) = value.as_string() {
        Some(text.to_owned())
    } else if let Some(whole) = value.as_integer() {
        Some(whole.to_string())
    } else {
        value.as_float().map(|number| number.to_string())
    }
}

impl<'a> Node<'a> {
    pub(crate) fn name(&self) -> &'a str {
        self.raw.name().value()
    }

    /// The line the node starts on, counted from 1.
    pub(crate) fn line(&self) -> usize {
        let offset = self.raw.span().offset().min(self.text.len());
        self.text
            .get(..offset)
            .map_or(0, |before| before.matches('\n').count())
            + 1
    }

    /// A problem at this node, as an error.
    pub(crate) fn problem(&self, message: impl std::fmt::Display) -> anyhow::Error {
        anyhow!("line {}: `{}`: {message}", self.line(), self.name())
    }

    /// The values that have no key, as text. A value that is not a word or a number is a problem.
    pub(crate) fn args(&self) -> anyhow::Result<Vec<String>> {
        self.raw
            .entries()
            .iter()
            .filter(|entry| entry.name().is_none())
            .map(|entry| {
                scalar(entry.value())
                    .ok_or_else(|| self.problem("a value must be a word or a number"))
            })
            .collect()
    }

    /// The one value without a key, which a node of this kind must have.
    pub(crate) fn only_arg(&self) -> anyhow::Result<String> {
        let mut args = self.args()?;
        if args.len() == 1 {
            Ok(args.remove(0))
        } else {
            Err(self.problem(format!("needs exactly one value, has {}", args.len())))
        }
    }

    /// The `key=value` pairs, in file order.
    pub(crate) fn props(&self) -> anyhow::Result<Vec<(String, String)>> {
        self.raw
            .entries()
            .iter()
            .filter_map(|entry| entry.name().map(|key| (key, entry)))
            .map(|(key, entry)| {
                let value = scalar(entry.value()).ok_or_else(|| {
                    self.problem(format!("`{}` must be a word or a number", key.value()))
                })?;
                Ok((key.value().to_owned(), value))
            })
            .collect()
    }

    pub(crate) fn prop(&self, key: &str) -> anyhow::Result<Option<String>> {
        Ok(self
            .props()?
            .into_iter()
            .find_map(|(name, value)| (name == key).then_some(value)))
    }

    /// A pair that this node must have.
    pub(crate) fn required_prop(&self, key: &str) -> anyhow::Result<String> {
        self.prop(key)?
            .with_context(|| format!("line {}: `{}` needs `{key}=`", self.line(), self.name()))
    }

    /// A pair that is a number.
    pub(crate) fn number_prop(&self, key: &str) -> anyhow::Result<f64> {
        self.required_prop(key)?
            .parse()
            .map_err(|_| self.problem(format!("`{key}` must be a number")))
    }

    pub(crate) fn children(&self) -> Vec<Node<'a>> {
        self.raw
            .children()
            .map(|document| nodes_of(document, self.text))
            .unwrap_or_default()
    }

    /// The child with this name, which a node of this kind must have.
    pub(crate) fn child(&self, name: &str) -> anyhow::Result<Node<'a>> {
        self.children()
            .into_iter()
            .find(|child| child.name() == name)
            .with_context(|| {
                format!(
                    "line {}: `{}` needs a `{name}` line",
                    self.line(),
                    self.name()
                )
            })
    }

    /// The words of the child with this name.
    pub(crate) fn words(&self, name: &str) -> anyhow::Result<Vec<String>> {
        self.child(name)?.args()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_words_pairs_and_children_with_their_lines() {
        let document = Document::parse("a \"x\" 3 k=\"v\" {\n  b 1.5\n}\n").unwrap();
        let nodes = document.nodes();
        let node = nodes[0];
        assert_eq!(node.args().unwrap(), ["x", "3"]);
        assert_eq!(node.prop("k").unwrap().as_deref(), Some("v"));
        let child = node.child("b").unwrap();
        assert_eq!(child.line(), 2);
        assert_eq!(child.only_arg().unwrap(), "1.5");
        assert!(node.child("c").is_err());
    }

    #[test]
    fn a_value_that_is_not_a_word_is_a_problem_with_a_line() {
        let document = Document::parse("a #true\n").unwrap();
        let error = document.nodes()[0].args().unwrap_err().to_string();
        assert!(error.starts_with("line 1:"), "{error}");
    }
}
