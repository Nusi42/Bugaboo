//! The words of the sample issues: the lists of `data/issues.toml` and how a template is filled
//! from them. The same seed picks the same words, so the same text.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::error::SampleError;
use crate::random::SeededRandom;

/// The text of the lists that is part of this program.
const BUNDLED: &str = include_str!("../data/issues.toml");

/// The marker a comment keeps for the mention of a person, which the maker fills in (it knows who
/// the people are).
pub(crate) const MENTION: &str = "{mention}";

/// Titles and sentences for one type of issue.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Templates {
    pub(crate) titles: Vec<String>,
    #[serde(default)]
    pub(crate) happens: Vec<String>,
    #[serde(default)]
    pub(crate) steps: Vec<String>,
    #[serde(default)]
    pub(crate) expected: Vec<String>,
    #[serde(default)]
    pub(crate) about: Vec<String>,
}

/// The sentences of comments.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CommentWords {
    pub(crate) first: Vec<String>,
    pub(crate) middle: Vec<String>,
    pub(crate) last: Vec<String>,
    pub(crate) mentions: Vec<String>,
    pub(crate) edits: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct Customers {
    names: Vec<String>,
}

/// The whole file.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct IssueWords {
    things: BTreeMap<String, Vec<String>>,
    words: BTreeMap<String, Vec<String>>,
    pub(crate) bug: Templates,
    pub(crate) task: Templates,
    pub(crate) feature: Templates,
    pub(crate) goal: Templates,
    pub(crate) comments: CommentWords,
    customers: Customers,
}

/// What kind of thing the words of a project are about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Area {
    App,
    Web,
    Ops,
    Doc,
}

impl Area {
    fn key(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Web => "web",
            Self::Ops => "ops",
            Self::Doc => "doc",
        }
    }

    /// The area of a top-level project, by its short name: the order of the projects in
    /// `sample.json` is the order of the areas, and a project beyond them talks about the web.
    pub(crate) fn of_position(position: usize) -> Self {
        [Self::App, Self::Web, Self::Ops, Self::Doc]
            .get(position)
            .copied()
            .unwrap_or(Self::Web)
    }
}

impl IssueWords {
    /// The lists that are part of this program.
    ///
    /// # Errors
    /// Fails when the bundled file is not usable (a test keeps it usable).
    pub(crate) fn bundled() -> Result<Self, SampleError> {
        Self::parse(BUNDLED)
    }

    /// Lists from the text of an `issues.toml` file.
    ///
    /// # Errors
    /// Fails when the text is no such file or a list is empty.
    pub(crate) fn parse(text: &str) -> Result<Self, SampleError> {
        let words: Self =
            toml::from_str(text).map_err(|error| SampleError::Names(error.to_string()))?;
        let empty = words
            .things
            .values()
            .chain(words.words.values())
            .any(Vec::is_empty)
            || [&words.bug, &words.task, &words.feature, &words.goal]
                .iter()
                .any(|templates| templates.titles.is_empty())
            || words.customers.names.is_empty();
        if empty {
            return Err(SampleError::NamesRepeated);
        }
        Ok(words)
    }

    /// The customers of the sample.
    pub(crate) fn customers(&self) -> &[String] {
        &self.customers.names
    }

    /// The template text filled in: `{thing}` from the things of the area and every other
    /// `{word}` from the list of that name; a first capital letter in the braces (`{Thing}`) makes
    /// the word start with one. A marker that names no list stays (the mention of a comment).
    pub(crate) fn fill(&self, template: &str, area: Area, random: &mut SeededRandom) -> String {
        let mut text = String::with_capacity(template.len() + 16);
        let mut rest = template;
        while let Some(open) = rest.find('{') {
            text.push_str(&rest[..open]);
            let Some(close) = rest[open..].find('}') else {
                break;
            };
            let name = &rest[open + 1..open + close];
            let lower = name.to_lowercase();
            let list = if lower == "thing" {
                self.things.get(area.key())
            } else {
                self.words.get(&lower)
            };
            match list.and_then(|list| random.pick(list)) {
                Some(word) if name.starts_with(char::is_uppercase) => {
                    let mut letters = word.chars();
                    text.extend(letters.next().into_iter().flat_map(char::to_uppercase));
                    text.push_str(letters.as_str());
                }
                Some(word) => text.push_str(word),
                None => text.push_str(&rest[open..=open + close]),
            }
            rest = &rest[open + close + 1..];
        }
        text.push_str(rest);
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_lists_are_usable_and_a_template_is_filled_the_same_way_for_a_seed() {
        let words = IssueWords::bundled().unwrap();
        let fill = |seed| {
            let mut random = SeededRandom::new(seed);
            words.fill(
                "{Thing} crashes when {when}; {verb}",
                Area::App,
                &mut random,
            )
        };
        assert_eq!(fill(1), fill(1));
        assert_ne!(fill(1), fill(2));
        let text = fill(1);
        assert!(!text.contains('{'), "{text}");
        assert!(text.chars().next().unwrap().is_uppercase());
    }

    #[test]
    fn a_marker_without_a_list_stays_for_the_maker_to_fill_in() {
        let words = IssueWords::bundled().unwrap();
        let mut random = SeededRandom::new(3);
        let text = words.fill("Thanks, {mention}.", Area::Web, &mut random);
        assert_eq!(text, format!("Thanks, {MENTION}."));
    }

    #[test]
    fn text_that_is_no_list_of_words_is_refused() {
        assert!(IssueWords::parse("").is_err());
    }
}
