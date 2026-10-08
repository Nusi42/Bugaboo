//! A reader for KDL documents that says where a problem is: the one place the KDL crate is used.
//!
//! `design/theme.kdl` is read only here and only by xtask; the server and the browser never parse
//! KDL (ADR-0026). Every reader of the file asks for a [`Node`] and its words, and every problem it
//! reports names the line.

use anyhow::{Context as _, anyhow};
use kdl::{KdlDocument, KdlNode, KdlValue};

/// A parsed document and its text, so that a node can say which line it is on.
pub(crate) struct Document {
    /// Where each line of the text starts (a byte), so that a line is found without counting.
    line_starts: Vec<usize>,
    parsed: KdlDocument,
}

impl Document {
    pub(crate) fn parse(text: &str) -> anyhow::Result<Self> {
        let parsed: KdlDocument = text
            .parse()
            .map_err(|error: kdl::KdlError| parse_problem(text, &error))?;
        let line_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(at, _)| at + 1))
            .collect();
        Ok(Self {
            line_starts,
            parsed,
        })
    }

    /// The nodes of the top level, in file order.
    pub(crate) fn nodes(&self) -> Vec<Node<'_>> {
        nodes_of(&self.parsed, &self.line_starts)
    }
}

/// The line and the column (both from 1) of a byte of the text.
fn place(text: &str, offset: usize) -> (usize, usize) {
    let before = text.get(..offset.min(text.len())).unwrap_or_default();
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count())
        + 1;
    (line, column)
}

/// What the KDL parser found wrong, each problem with its line and column.
fn parse_problem(text: &str, error: &kdl::KdlError) -> anyhow::Error {
    let found: Vec<String> = error
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let (line, column) = place(text, diagnostic.span.offset());
            let what = diagnostic
                .message
                .as_deref()
                .or(diagnostic.label.as_deref())
                .unwrap_or("not valid KDL");
            format!("line {line}, column {column}: {what}")
        })
        .collect();
    if found.is_empty() {
        anyhow!("line 1, column 1: not valid KDL")
    } else {
        anyhow!("{}", found.join("; "))
    }
}

fn nodes_of<'a>(document: &'a KdlDocument, line_starts: &'a [usize]) -> Vec<Node<'a>> {
    document
        .nodes()
        .iter()
        .map(|raw| Node { raw, line_starts })
        .collect()
}

/// One node: its name, its words, its `key=value` pairs and its children.
#[derive(Clone, Copy)]
pub(crate) struct Node<'a> {
    raw: &'a KdlNode,
    line_starts: &'a [usize],
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
        let offset = self.raw.span().offset();
        self.line_starts.partition_point(|start| *start <= offset)
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
            .map(|document| nodes_of(document, self.line_starts))
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
    fn bad_example_a_document_that_is_not_kdl_says_where() {
        let error = Document::parse("a \"x\"\nb {\n  c \"y\n}\n").err().unwrap();
        let text = error.to_string();
        assert!(text.contains("line 3, column"), "{text}");
    }

    #[test]
    fn a_value_that_is_not_a_word_is_a_problem_with_a_line() {
        let document = Document::parse("a #true\n").unwrap();
        let error = document.nodes()[0].args().unwrap_err().to_string();
        assert!(error.starts_with("line 1:"), "{error}");
    }
}
