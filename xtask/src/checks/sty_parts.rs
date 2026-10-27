//! STY-PARTS: every part that `design/theme.kdl` styles is a part some component has (ADR-0026).
//!
//! A rule of the theme file names the parts it styles (`part "name-value.value > span"`). A
//! component is a part by `data-part`, which it takes from the generated list of names (`part.fieldLabel`).
//! A rule for a part that no component names styles nothing and says it does: a mistake the style
//! sheet never shows (a bare element name written in a selector is read as a part of that name, and
//! no component has it). This check lists every part of the theme file that is named in no file of
//! the browser app and its packages, so the rule is given a real part or is removed.

use std::collections::{BTreeSet, HashSet};

use super::{Check, Context, Finding};
use crate::generate::all_paths;
use crate::generate::theme_file::{RuleKind, THEME_PATH, ThemeFile};
use crate::source::{Kind, Source};

pub(crate) struct StyParts;

impl Check for StyParts {
    fn id(&self) -> &'static str {
        "STY-PARTS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let file = ThemeFile::load(context.repo)?;
        let generated = all_paths();
        let sources: Vec<Source> = context
            .sources(self.id(), Kind::Web)?
            .into_iter()
            .filter(|(path, _)| !generated.contains(&path.as_str()))
            .collect();
        let mut named: Vec<(String, usize)> = Vec::new();
        for name in file.part_names() {
            let line = file
                .every_rule()
                .filter(|rule| rule.kind == RuleKind::Part)
                .find(|rule| {
                    rule.selectors
                        .iter()
                        .any(|selector| selector.contains(&name))
                })
                .map_or(0, |rule| rule.line);
            named.push((name, line));
        }
        Ok(unused(&named, &sources))
    }
}

/// `field.label` as `fieldLabel`: the way the generated list of names writes a part.
fn camel(name: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in name.chars() {
        if c == '.' || c == '-' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Everything the sources name that can be a part, found in one pass over each text: the word
/// after each `part.` and every text between two neighbouring quotes.
fn names_in(sources: &[Source]) -> HashSet<&str> {
    let mut found = HashSet::new();
    for (_, text) in sources {
        for (at, _) in text.match_indices("part.") {
            let rest = &text[at + "part.".len()..];
            let end = rest
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            found.insert(&rest[..end]);
        }
        let mut quotes = text.match_indices('"').map(|(at, _)| at).peekable();
        while let Some(open) = quotes.next() {
            if let Some(&close) = quotes.peek() {
                found.insert(&text[open + 1..close]);
            }
        }
    }
    found
}

/// The parts, with the line of their first rule, that no source names.
fn unused(parts: &[(String, usize)], sources: &[Source]) -> Vec<Finding> {
    let mut seen = BTreeSet::new();
    let named = names_in(sources);
    parts
        .iter()
        .filter(|(name, _)| seen.insert(name.clone()))
        .filter(|(name, _)| !named.contains(camel(name).as_str()) && !named.contains(name.as_str()))
        .map(|(name, line)| {
            Finding::at(
                format!("{THEME_PATH}:{line}"),
                format!(
                    "No component is the part `{name}`, so the rules for it style nothing. Give an element that part (`partOf(part.{})`) or take the rules away.",
                    camel(name)
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources(text: &str) -> Vec<Source> {
        vec![("web/app/src/a.tsx".to_owned(), text.to_owned())]
    }

    fn found(names: &[&str], text: &str) -> usize {
        let parts: Vec<(String, usize)> = names.iter().map(|n| ((*n).to_owned(), 1)).collect();
        unused(&parts, &sources(text)).len()
    }

    #[test]
    fn good_example_every_part_is_a_part_of_a_component() {
        assert_eq!(
            found(
                &["name-value.value", "button"],
                "<div {...partOf(part.nameValueValue)}><b {...partOf(part.button)} /></div>"
            ),
            0
        );
        // A part named by its string, as the style sheet and the tests do.
        assert_eq!(found(&["app-layout"], "root.closest(\"app-layout\")"), 0);
    }

    #[test]
    fn bad_example_a_rule_for_a_part_no_component_has() {
        // `span` in `name-value.value > span` is read as a part that no element has.
        assert_eq!(
            found(&["name-value.value", "span"], "partOf(part.nameValueValue)"),
            1
        );
        // A part whose name only starts like another one is not that one.
        assert_eq!(found(&["tab"], "partOf(part.tabPanel)"), 1);
    }
}
