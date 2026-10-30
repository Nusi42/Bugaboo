//! `design/theme.kdl` as typed data (ADR-0026): the one theming file.
//!
//! The generator, the contrast check and the style checks all read it through here, so the file
//! has one reader. The file holds the themes, the tokens with their variants per mode on one line,
//! the contrast rules, and the rules of the components (`part` rules that use tokens only).

use std::collections::BTreeSet;

use anyhow::{Context as _, bail};

use super::bold_spec::BoldSpec;
use super::contrast_spec::{ContrastSpec, DistinctSpec};
use super::kdl_reader::{Document, Node};
use crate::repo::Repo;

mod builder;
use builder::Builder;

/// Where the theme file lives, relative to the repository root.
pub(crate) const THEME_PATH: &str = "design/theme.kdl";

/// What a token is a value of. The word is the name of its line in the file.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TokenKind {
    Color,
    Font,
    Size,
    Number,
    Duration,
    Easing,
    Shadow,
    Weight,
    LineHeight,
    BorderWidth,
    Layer,
    Opacity,
}

/// The line names of the tokens, once.
const TOKEN_KINDS: [(&str, TokenKind); 12] = [
    ("color", TokenKind::Color),
    ("font", TokenKind::Font),
    ("size", TokenKind::Size),
    ("number", TokenKind::Number),
    ("duration", TokenKind::Duration),
    ("easing", TokenKind::Easing),
    ("shadow", TokenKind::Shadow),
    ("weight", TokenKind::Weight),
    ("line-height", TokenKind::LineHeight),
    ("border-width", TokenKind::BorderWidth),
    ("layer", TokenKind::Layer),
    ("opacity", TokenKind::Opacity),
];

impl TokenKind {
    fn of(word: &str) -> Option<Self> {
        TOKEN_KINDS
            .iter()
            .find(|(name, _)| *name == word)
            .map(|(_, kind)| *kind)
    }

    /// The kinds that have a value for each theme.
    pub(crate) fn varies_by_theme(self) -> bool {
        matches!(self, Self::Color | Self::Shadow)
    }
}

/// One built-in theme.
pub(crate) struct Theme {
    pub(crate) name: String,
    /// `light` or `dark`; decides the browser's own controls and scrollbars.
    pub(crate) kind: String,
}

/// A choice that changes token values without changing the theme: spacing, text size, motion.
pub(crate) struct Variant {
    pub(crate) name: String,
    /// The attribute on `<html>` that switches it on, and its value. A choice that only the device
    /// makes (a touch screen) has none and is switched on by its `media` alone.
    pub(crate) attribute: Option<(String, String)>,
    /// A device setting that switches it on as well (`(prefers-reduced-motion: reduce)`).
    pub(crate) media: Option<String>,
}

/// One token: a name and its value in every mode, on one line of the file.
pub(crate) struct Token {
    /// The CSS variable name without `--`.
    pub(crate) name: String,
    pub(crate) kind: TokenKind,
    /// The value that is the same in every theme; `$other` names another token.
    pub(crate) same: Option<String>,
    /// The value in each theme, in the order of the themes.
    pub(crate) themes: Vec<(String, String)>,
    /// The value in a theme when the device asks for more contrast.
    pub(crate) more: Vec<(String, String)>,
    /// The value while forced colors are on.
    pub(crate) forced: Option<String>,
    /// The value while a choice is on (the name of a [`Variant`]).
    pub(crate) variants: Vec<(String, String)>,
    /// Marked `run-time="true"`: the code may set this variable while the person drags something
    /// (the size of a window). Only a size can be one.
    pub(crate) run_time: bool,
    pub(crate) line: usize,
}

impl Token {
    pub(crate) fn in_theme(&self, theme: &str) -> Option<&str> {
        self.themes
            .iter()
            .find(|(name, _)| name == theme)
            .map(|(_, value)| value.as_str())
    }

    /// The value in a theme with more contrast: the token's own, where it has one.
    pub(crate) fn in_theme_with_more_contrast(&self, theme: &str) -> Option<&str> {
        self.more
            .iter()
            .find(|(name, _)| name == theme)
            .map(|(_, value)| value.as_str())
            .or_else(|| self.in_theme(theme))
    }
}

/// A line of a rule: `property value`.
pub(crate) struct Declaration {
    pub(crate) property: String,
    /// The words of the value, joined; `$token` is not resolved yet.
    pub(crate) value: String,
    pub(crate) line: usize,
}

/// What a rule or a mixin holds.
pub(crate) enum Item {
    Declaration(Declaration),
    /// `use "name"`: the lines of a mixin, in this place.
    Use {
        mixin: String,
        line: usize,
    },
    /// `forced { … }`, `more-contrast { … }`, `media "…" { … }`, `container "…" { … }` or `supports "…" { … }`.
    Media {
        at: AtRule,
        query: String,
        items: Vec<Item>,
    },
}

/// The at-rule that a block of lines is written under.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AtRule {
    /// The device: its screen and its settings (`@media`).
    Media,
    /// The box the element is in (`@container`), so a window follows its own width.
    Container,
    /// What the browser knows (`@supports`): the lines are only read by a browser that knows the feature.
    Supports,
}

impl AtRule {
    pub(crate) const fn css(self) -> &'static str {
        match self {
            Self::Media => "@media",
            Self::Container => "@container",
            Self::Supports => "@supports",
        }
    }
}

/// What a rule styles.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuleKind {
    /// `base "html, body"`: elements of the page itself.
    Base,
    /// `part "button[variant=primary]:hover"`: an element that says which part it is.
    Part,
    /// `rule ".btn.primary"` inside `docs`: a selector of the design pages and mockups, written
    /// as CSS (the pages are documents, not the app).
    Doc,
}

pub(crate) struct Rule {
    pub(crate) kind: RuleKind,
    /// One or more selectors that share the lines of the rule.
    pub(crate) selectors: Vec<String>,
    pub(crate) items: Vec<Item>,
    pub(crate) line: usize,
}

pub(crate) struct Mixin {
    pub(crate) name: String,
    pub(crate) items: Vec<Item>,
}

pub(crate) struct Keyframes {
    pub(crate) name: String,
    /// `from`, `to` or a percentage, with its lines.
    pub(crate) frames: Vec<(String, Vec<Declaration>)>,
    pub(crate) line: usize,
}

/// The names of the attributes that the page frame (`<html>`) carries: written once, here, and
/// used by the style sheets, the TypeScript and the end-to-end tests (M2C-F91).
pub(crate) struct Frame {
    /// The theme on the page (`data-look`).
    pub(crate) look: String,
    /// The light or dark mode that the design pages force (`data-theme`).
    pub(crate) mode: String,
    /// A sample of a theme next to the page's own (`data-preview-look`).
    pub(crate) preview: String,
}

/// The style sheet of the design pages that one `docs` block makes: its own tokens, the rules (CSS
/// selectors) and the keyframes.
pub(crate) struct DocSheet {
    /// The name of the sheet: `components` or `docpage`.
    pub(crate) name: String,
    /// What the sheet is for, in the notice at its top.
    pub(crate) description: String,
    pub(crate) tokens: Vec<Token>,
    pub(crate) rules: Vec<Rule>,
    pub(crate) keyframes: Vec<Keyframes>,
}

/// The sheets of the design pages and the files they are written to.
pub(crate) const DOC_SHEETS: [(&str, &str); 4] = [
    ("components", "docs/design/components.css"),
    ("docpage", "docs/design/docpage.css"),
    ("ux-design", "docs/design/ux-design.css"),
    ("mockup", "docs/design/mockup.css"),
];

pub(crate) struct ThemeFile {
    pub(crate) themes: Vec<Theme>,
    pub(crate) default_light: String,
    pub(crate) default_dark: String,
    pub(crate) frame: Frame,
    pub(crate) variants: Vec<Variant>,
    pub(crate) tokens: Vec<Token>,
    pub(crate) mixins: Vec<Mixin>,
    pub(crate) keyframes: Vec<Keyframes>,
    pub(crate) rules: Vec<Rule>,
    pub(crate) docs: Vec<DocSheet>,
    pub(crate) contrast: ContrastSpec,
    /// One rule for the plain surfaces, and one for each kind of tinted row.
    pub(crate) distinct: Vec<DistinctSpec>,
    /// The one list of what may be bold.
    pub(crate) bold: BoldSpec,
}

/// The names that are not CSS properties inside a rule.
const FORCED: &str = "forced";
const MORE_CONTRAST: &str = "more-contrast";
const MEDIA: &str = "media";
const CONTAINER: &str = "container";
const SUPPORTS: &str = "supports";
const USE: &str = "use";
/// The media queries of the two short names.
pub(crate) const FORCED_QUERY: &str = "(forced-colors: active)";
pub(crate) const MORE_CONTRAST_QUERY: &str = "(prefers-contrast: more)";

impl ThemeFile {
    pub(crate) fn load(repo: &Repo) -> anyhow::Result<Self> {
        let path = repo.path(THEME_PATH);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        Self::parse(&crate::source::normalize(&text))
            .with_context(|| format!("{THEME_PATH} is not valid"))
    }

    pub(crate) fn parse(text: &str) -> anyhow::Result<Self> {
        let document = Document::parse(text)?;
        let mut builder = Builder::default();
        for node in document.nodes() {
            builder.add(node)?;
        }
        builder.finish()
    }

    pub(crate) fn theme(&self, name: &str) -> Option<&Theme> {
        self.themes.iter().find(|theme| theme.name == name)
    }

    pub(crate) fn token(&self, name: &str) -> Option<&Token> {
        self.tokens
            .iter()
            .chain(self.docs.iter().flat_map(|sheet| &sheet.tokens))
            .find(|token| token.name == name)
    }

    /// The value of an opacity token (0 to 1, the same in every theme); `None` for a name that is no
    /// opacity token or whose value is not a number from 0 to 1.
    pub(crate) fn opacity(&self, name: &str) -> Option<f64> {
        let token = self
            .token(name)
            .filter(|token| token.kind == TokenKind::Opacity)?;
        token
            .same
            .as_deref()?
            .parse::<f64>()
            .ok()
            .filter(|opacity| (0.0..=1.0).contains(opacity))
    }

    /// The value of a number token (the same in every theme); `None` for a name that is no number token.
    pub(crate) fn number(&self, name: &str) -> Option<f64> {
        self.token(name)
            .filter(|token| token.kind == TokenKind::Number)?
            .same
            .as_deref()?
            .parse()
            .ok()
    }

    /// The rules of the app and of the design pages, for a check that reads every value.
    pub(crate) fn every_rule(&self) -> impl Iterator<Item = &Rule> {
        self.rules
            .iter()
            .chain(self.docs.iter().flat_map(|sheet| &sheet.rules))
    }

    /// The keyframes of the app and of the design pages.
    pub(crate) fn every_keyframes(&self) -> impl Iterator<Item = &Keyframes> {
        self.keyframes
            .iter()
            .chain(self.docs.iter().flat_map(|sheet| &sheet.keyframes))
    }

    /// The colors, in file order.
    pub(crate) fn colors(&self) -> impl Iterator<Item = &Token> {
        self.tokens
            .iter()
            .filter(|token| token.kind == TokenKind::Color)
    }

    /// The words of `data-variant` and `data-state` that the rules ask of each part, once each, in
    /// file order. A part that the rules style without a word is there with no words.
    pub(crate) fn part_words(&self) -> Vec<super::theme_selector::Words> {
        let mut parts: Vec<super::theme_selector::Words> = Vec::new();
        for rule in self.rules.iter().filter(|rule| rule.kind == RuleKind::Part) {
            for found in rule
                .selectors
                .iter()
                .flat_map(|selector| super::theme_selector::part_words(selector))
            {
                let at = parts.iter().position(|known| known.part == found.part);
                let known = if let Some(at) = at {
                    &mut parts[at]
                } else {
                    parts.push(super::theme_selector::Words {
                        part: found.part.clone(),
                        variants: Vec::new(),
                        states: Vec::new(),
                    });
                    let last = parts.len() - 1;
                    &mut parts[last]
                };
                for word in found.variants {
                    if !known.variants.contains(&word) {
                        known.variants.push(word);
                    }
                }
                for word in found.states {
                    if !known.states.contains(&word) {
                        known.states.push(word);
                    }
                }
            }
        }
        parts
    }

    /// The names of the parts that the rules style, once each, in file order.
    pub(crate) fn part_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for rule in self.rules.iter().filter(|rule| rule.kind == RuleKind::Part) {
            for name in rule
                .selectors
                .iter()
                .flat_map(|selector| super::theme_selector::part_names(selector))
            {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
        names
    }
}

impl ThemeFile {
    fn validate(&self) -> anyhow::Result<()> {
        if self.themes.is_empty() {
            bail!("there are no themes");
        }
        for theme in &self.themes {
            if theme.kind != "light" && theme.kind != "dark" {
                bail!("theme `{}` must be `light` or `dark`", theme.name);
            }
        }
        for (what, name, kind) in [
            ("light", &self.default_light, "light"),
            ("dark", &self.default_dark, "dark"),
        ] {
            match self.theme(name) {
                Some(theme) if theme.kind == kind => {}
                _ => bail!("the default {what} theme `{name}` is not a {kind} theme"),
            }
        }
        self.validate_colors()?;
        self.validate_references()?;
        Ok(())
    }

    fn validate_colors(&self) -> anyhow::Result<()> {
        let spec = &self.contrast;
        if let Some(unknown) = spec
            .named_colors()
            .chain(self.distinct.iter().flat_map(|spec| {
                spec.groups
                    .iter()
                    .flat_map(|group| group.states.iter().chain(&group.surfaces))
            }))
            .find(|name| {
                self.token(name)
                    .is_none_or(|token| token.kind != TokenKind::Color)
            })
        {
            bail!("contrast rules use unknown color `{unknown}`");
        }
        for fading in spec
            .plain_lines
            .iter()
            .filter_map(|lines| lines.faded_by.as_deref())
        {
            if self.opacity(fading).is_none() {
                bail!("a contrast rule is faded by `{fading}`, which is not an opacity token");
            }
        }
        if let Some(color) = self.colors().find(|color| !spec.classifies(&color.name)) {
            bail!(
                "color `{}` is not classified: list it under contrast `text`, `backgrounds` or `non-text` (with a reason)",
                color.name
            );
        }
        spec.check_exclusions()
    }

    /// Every `$token` and every `use` names something that is defined.
    fn validate_references(&self) -> anyhow::Result<()> {
        let mut known: BTreeSet<&str> = BTreeSet::new();
        let every_token = self
            .tokens
            .iter()
            .chain(self.docs.iter().flat_map(|sheet| &sheet.tokens));
        for token in every_token.clone() {
            known.insert(&token.name);
        }
        let unknown_in = |value: &str, line: usize| -> anyhow::Result<()> {
            for name in references(value) {
                if !known.contains(name.as_str()) {
                    bail!("line {line}: `${name}` is not a token");
                }
            }
            Ok(())
        };
        for token in every_token {
            for value in token
                .same
                .iter()
                .chain(token.variants.iter().map(|(_, value)| value))
                .chain(token.forced.iter())
            {
                unknown_in(value, token.line)?;
            }
        }
        let mixin_names: BTreeSet<&str> = self
            .mixins
            .iter()
            .map(|mixin| mixin.name.as_str())
            .collect();
        let all_items = self
            .every_rule()
            .map(|rule| &rule.items)
            .chain(self.mixins.iter().map(|mixin| &mixin.items));
        for items in all_items {
            check_items(items, &mixin_names, &unknown_in)?;
        }
        Ok(())
    }
}

fn check_items(
    items: &[Item],
    mixins: &BTreeSet<&str>,
    unknown_in: &dyn Fn(&str, usize) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    for item in items {
        match item {
            Item::Declaration(declaration) => unknown_in(&declaration.value, declaration.line)?,
            Item::Use { mixin, line } => {
                if !mixins.contains(mixin.as_str()) {
                    bail!("line {line}: `use \"{mixin}\"` names no mixin");
                }
            }
            Item::Media { items, .. } => check_items(items, mixins, unknown_in)?,
        }
    }
    Ok(())
}

/// The names of the tokens a value refers to with `$name`.
pub(crate) fn references(value: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = value;
    while let Some(at) = rest.find('$') {
        let after = &rest[at + 1..];
        let length = after
            .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
            .unwrap_or(after.len());
        if length > 0 {
            names.push(after[..length].to_owned());
        }
        rest = &after[length..];
    }
    names
}

/// A value with `$name` written as `var(--name)`: how a value of the file is put into CSS.
pub(crate) fn substitute(value: &str) -> String {
    let mut out = String::new();
    let mut rest = value;
    while let Some(at) = rest.find('$') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let length = after
            .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
            .unwrap_or(after.len());
        if length == 0 {
            out.push('$');
        } else {
            out.push_str("var(--");
            out.push_str(&after[..length]);
            out.push(')');
        }
        rest = &after[length..];
    }
    out.push_str(rest);
    out
}

/// A font stack in CSS: a name with a space in it is quoted.
fn font_stack(families: &[String]) -> String {
    families
        .iter()
        .map(|name| {
            if name.contains(' ') {
                format!("\"{name}\"")
            } else {
                name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn is_property(name: &str) -> bool {
    let mut letters = name.trim_start_matches('-').chars();
    letters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && letters.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// What would end a value and start another rule or comment in the style sheet.
const BREAKOUT: [&str; 6] = ["{", "}", ";", "/*", "*/", "<"];

/// The system colors of CSS, the only values that `forced=` of a color may have.
const SYSTEM_COLORS: [&str; 19] = [
    "AccentColor",
    "AccentColorText",
    "ActiveText",
    "ButtonBorder",
    "ButtonFace",
    "ButtonText",
    "Canvas",
    "CanvasText",
    "Field",
    "FieldText",
    "GrayText",
    "Highlight",
    "HighlightText",
    "LinkText",
    "Mark",
    "MarkText",
    "SelectedItem",
    "SelectedItemText",
    "VisitedText",
];

/// A value, and a media query, may not hold what lets it break out of its rule.
fn check_text(node: Node<'_>, value: &str) -> anyhow::Result<()> {
    match BREAKOUT.iter().find(|bad| value.contains(**bad)) {
        Some(bad) => Err(node.problem(format!(
            "the value `{value}` holds `{bad}`, which could end the rule and start another"
        ))),
        None => Ok(()),
    }
}

fn declaration(node: Node<'_>) -> anyhow::Result<Declaration> {
    if !is_property(node.name()) {
        return Err(node.problem("is not a CSS property"));
    }
    let words = node.args()?;
    if words.is_empty() {
        return Err(node.problem("needs a value"));
    }
    let value = words.join(" ");
    check_text(node, &value)?;
    Ok(Declaration {
        property: node.name().to_owned(),
        value,
        line: node.line(),
    })
}

/// The lines inside a rule, a mixin or a media block.
fn items(node: Node<'_>) -> anyhow::Result<Vec<Item>> {
    node.children()
        .into_iter()
        .map(|child| match child.name() {
            USE => Ok(Item::Use {
                mixin: child.only_arg()?,
                line: child.line(),
            }),
            FORCED => media(child, AtRule::Media, FORCED_QUERY.to_owned()),
            MORE_CONTRAST => media(child, AtRule::Media, MORE_CONTRAST_QUERY.to_owned()),
            MEDIA => media(child, AtRule::Media, child.only_arg()?),
            CONTAINER => media(child, AtRule::Container, child.only_arg()?),
            SUPPORTS => media(child, AtRule::Supports, child.only_arg()?),
            _ => Ok(Item::Declaration(declaration(child)?)),
        })
        .collect()
}

fn media(node: Node<'_>, at: AtRule, query: String) -> anyhow::Result<Item> {
    check_text(node, &query)?;
    Ok(Item::Media {
        at,
        query,
        items: items(node)?,
    })
}

/// `variant "touch" media="(pointer: coarse)"` or `variant "compact" attribute="data-spacing" value="compact"`.
fn variant(node: Node<'_>) -> anyhow::Result<Variant> {
    let (attribute, value, media) = (
        node.prop("attribute")?,
        node.prop("value")?,
        node.prop("media")?,
    );
    let attribute = match (attribute, value) {
        (Some(attribute), Some(value)) => Some((attribute, value)),
        (None, None) if media.is_some() => None,
        _ => {
            return Err(node.problem(
                "needs `attribute=` and `value=` (a choice of the person), or only `media=` (a choice of the device)",
            ));
        }
    };
    Ok(Variant {
        name: node.only_arg()?,
        attribute,
        media,
    })
}

fn rule(node: Node<'_>, kind: RuleKind) -> anyhow::Result<Rule> {
    let selectors = node.args()?;
    if selectors.is_empty() {
        return Err(node.problem("needs a selector"));
    }
    Ok(Rule {
        kind,
        selectors,
        items: items(node)?,
        line: node.line(),
    })
}

fn keyframes(node: Node<'_>) -> anyhow::Result<Keyframes> {
    let mut frames = Vec::new();
    for frame in node.children() {
        let declarations = frame
            .children()
            .into_iter()
            .map(declaration)
            .collect::<anyhow::Result<Vec<_>>>()?;
        frames.push((frame.name().to_owned(), declarations));
    }
    Ok(Keyframes {
        name: node.only_arg()?,
        frames,
        line: node.line(),
    })
}

#[cfg(test)]
mod tests;
