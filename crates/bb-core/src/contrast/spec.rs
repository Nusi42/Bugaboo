//! The contrast rules of `design/theme.kdl`, as data: which text colors must be readable on which
//! backgrounds, which lines and fills must be seen, which states must differ (WCAG, THM-03, UX-04).
//!
//! `cargo xtask gen theme` writes the rules of the base file as [`super::base_rules`]; the reader of
//! the KDL text stays in `xtask`, because the server and the browser never parse KDL.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// A name in the rules: written into the program by the generator, read from the theme file by the
/// check.
pub type Text = Cow<'static, str>;

/// A list in the rules, the same way.
pub type List<T> = Cow<'static, [T]>;

/// A text color that is only drawn on some backgrounds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Restricted {
    /// The text colors (or the one text color).
    pub text: Text,
    /// The backgrounds (or fills) it is drawn on.
    pub on: List<Text>,
}

/// Text and background combinations that are never used.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Exclusion {
    /// Color names; `*` matches any characters.
    pub text: List<Text>,
    /// The backgrounds (or fills) it is drawn on.
    pub on: List<Text>,
    /// Themes the exclusion applies to; empty means all.
    pub themes: List<Text>,
}

/// Some states and the surfaces they sit on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DistinctGroup {
    /// The states.
    pub states: List<Text>,
    /// The surfaces they are drawn on.
    pub surfaces: List<Text>,
}

/// State backgrounds (hover, selected) that must stay visible against the surfaces.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DistinctSpec {
    /// The states and the surfaces they must stay visible on: one group for the plain surfaces, one
    /// for each kind of tinted row.
    pub groups: List<DistinctGroup>,
    /// Smallest contrast ratio between a state and each surface (0 when the rule does not ask for one).
    pub min_against_surfaces: f64,
    /// Smallest contrast ratio between two states (0 when the rule does not ask for one).
    pub min_between_states: f64,
    /// Smallest difference in color (CIE 76 delta E) between a state and each surface. A ratio is
    /// blind to the hue, so a rule that has to tell a blue from a grey of the same brightness says this.
    pub min_delta_e_against_surfaces: Option<f64>,
    /// Smallest difference in color (delta E) between two states.
    pub min_delta_e_between_states: Option<f64>,
    /// Largest chroma (C*ab) of each state: the states are neutral greys, without a hue of their own.
    pub max_chroma_of_states: Option<f64>,
}

/// A text color on a fill that is brightened while it is pointed at (`filter: brightness`): the
/// pair is checked again as it is seen then.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hovered {
    /// The text colors (or the one text color).
    pub text: Text,
    /// The backgrounds (or fills) it is drawn on.
    pub on: Text,
    /// The number token with the brightness of the hover.
    pub brightness: Text,
    /// The smallest ratio as it is, and with the device setting for more contrast.
    pub min: f64,
    /// The least ratio with more contrast.
    pub min_more: f64,
}

/// The page behind a sheet, dimmed by a scrim: the sheet must stand out from it and the text of the
/// page must be dim, in every theme, as it is drawn (the scrim mixed over the fill and over the text).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dimmed {
    /// The scrim color.
    pub scrim: Text,
    /// The opacity token of the scrim.
    pub opacity: Text,
    /// The fill of the page.
    pub page: Text,
    /// The text colors (or the one text color).
    pub text: Text,
    /// The fill of the sheet.
    pub sheet: Text,
    /// The least ratio of the sheet to the dimmed page in a light theme and in a dark one.
    pub min_sheet_light: f64,
    /// The least ratio in a dark theme.
    pub min_sheet_dark: f64,
    /// The most ratio that the dimmed text of the page may have against the dimmed page.
    pub max_text: f64,
}

/// Text colors that need more than the usual ratio on some backgrounds, because small words in a
/// thin face are read on them all day (the muted words of a tinted row).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StrongerText {
    /// The least ratio.
    pub min: f64,
    /// The text colors (or the one text color).
    pub text: List<Text>,
    /// The backgrounds (or fills) it is drawn on.
    pub on: List<Text>,
}

/// The stronger rules for people whose device asks for more contrast (`prefers-contrast: more`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoreSpec {
    /// Smallest contrast ratio of the listed text colors on their backgrounds.
    pub min_text: f64,
    /// Smallest contrast ratio of the listed line colors on the plain surfaces.
    pub min_line: f64,
    /// The text colors (or the one text color).
    pub text: List<Text>,
    /// The line colors.
    pub lines: List<Text>,
    /// The surfaces the lines stand on.
    pub line_on: List<Text>,
}

/// What a group of [`PlainLines`] draws: a line, or a fill that carries no text (the square of a label).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Drawn {
    /// A line (`lines`): the edge of a box to tick, a ring, a divider.
    Line,
    /// A fill without text (`fills`): only its color has to be seen, so it is held to the ratio of a line
    /// (WCAG 1.4.11), not to the ratio of the text that other fills carry (M5-F149).
    Fill,
}

impl Drawn {
    /// The word a finding uses for it.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::Fill => "fill",
        }
    }
}

/// Lines that people have to see (the edge of a box to tick), and fills without text (the square of a
/// label), kept at a ratio on the plain surfaces in every theme, also without the device setting for more
/// contrast.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlainLines {
    /// Whether the colors are lines or fills without text; both are checked the same way.
    pub drawn: Drawn,
    /// The least ratio.
    pub min: f64,
    /// The colors.
    pub colors: List<Text>,
    /// The backgrounds (or fills) it is drawn on.
    pub on: List<Text>,
    /// The opacity token that fades the line before it is seen (`faded-by`), when the line is.
    pub faded_by: Option<Text>,
}

/// The `contrast` rule of the theme file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContrastSpec {
    /// The least ratio.
    pub min: f64,
    /// The lines and fills that people have to see.
    pub plain_lines: List<PlainLines>,
    /// The pairs seen while hovered.
    pub hovered: List<Hovered>,
    /// The words that need a stronger ratio.
    pub stronger: List<StrongerText>,
    /// The page behind a sheet.
    pub dimmed: List<Dimmed>,
    /// The rules for more contrast.
    pub more: MoreSpec,
    /// The text colors (or the one text color).
    pub text: List<Text>,
    /// The backgrounds text may be drawn on.
    pub backgrounds: List<Text>,
    /// The text colors drawn on some backgrounds only.
    pub restricted: List<Restricted>,
    /// The pairs that are never used.
    pub exclusions: List<Exclusion>,
    /// Colors that are neither text nor a background for text (borders), each with a reason.
    pub non_text: List<Text>,
}

impl ContrastSpec {
    /// The (text, background) pairs to check in `theme`.
    #[must_use]
    pub fn pairs(&self, theme: &str) -> Vec<(&str, &str)> {
        let mut pairs = Vec::new();
        for text in self.text.iter() {
            if let Some(restricted) = self.restricted.iter().find(|r| &r.text == text) {
                pairs.extend(restricted.on.iter().map(|on| (text.as_ref(), on.as_ref())));
                continue;
            }
            for background in self.backgrounds.iter() {
                let excluded = self.exclusions.iter().any(|exclusion| {
                    (exclusion.themes.is_empty() || exclusion.themes.iter().any(|t| t == theme))
                        && exclusion.text.iter().any(|p| glob(p, text))
                        && exclusion.on.iter().any(|p| glob(p, background))
                });
                if !excluded {
                    pairs.push((text.as_ref(), background.as_ref()));
                }
            }
        }
        pairs
    }

    /// Every color name used anywhere in the rules (exact names, no patterns).
    pub fn named_colors(&self) -> impl Iterator<Item = &Text> {
        self.text
            .iter()
            .chain(self.backgrounds.iter())
            .chain(self.more.text.iter())
            .chain(self.more.lines.iter())
            .chain(self.more.line_on.iter())
            .chain(
                self.plain_lines
                    .iter()
                    .flat_map(|lines| lines.colors.iter().chain(lines.on.iter())),
            )
            .chain(
                self.restricted
                    .iter()
                    .flat_map(|r| std::iter::once(&r.text).chain(r.on.iter())),
            )
            .chain(self.hovered.iter().flat_map(|rule| [&rule.text, &rule.on]))
            .chain(
                self.dimmed
                    .iter()
                    .flat_map(|rule| [&rule.scrim, &rule.page, &rule.text, &rule.sheet]),
            )
            .chain(
                self.stronger
                    .iter()
                    .flat_map(|rule| rule.text.iter().chain(rule.on.iter())),
            )
    }

    /// Whether the rules say what this color is: text, a background, or not text at all.
    #[must_use]
    pub fn classifies(&self, color: &str) -> bool {
        self.text
            .iter()
            .chain(self.backgrounds.iter())
            .chain(self.non_text.iter())
            .any(|name| name == color)
    }

    /// The first exclusion that matches no color (it is stale), if there is one.
    #[must_use]
    pub fn stale_exclusion(&self) -> Option<&Exclusion> {
        self.exclusions.iter().find(|exclusion| {
            let all_match = |patterns: &[Text], names: &[Text]| {
                patterns
                    .iter()
                    .all(|pattern| names.iter().any(|name| glob(pattern, name)))
            };
            !all_match(&exclusion.text, &self.text) || !all_match(&exclusion.on, &self.backgrounds)
        })
    }
}

/// Matches `name` against a pattern where `*` stands for any characters.
fn glob(pattern: &str, name: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == name,
        Some((start, rest)) => name.strip_prefix(start).is_some_and(|tail| {
            (0..=tail.len()).any(|skip| tail.get(skip..).is_some_and(|rest_of| glob(rest, rest_of)))
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::glob;

    #[test]
    fn star_matches_any_characters() {
        assert!(glob("st-*-soft", "st-ip-soft"));
        assert!(glob("cat*-bg", "cat3-bg"));
        assert!(glob("fg-*", "fg-muted"));
        assert!(!glob("fg-*", "fg"));
        assert!(!glob("danger", "danger-soft"));
    }
}
