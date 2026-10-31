//! The files of `design/themes/`: one built-in theme each, which sets colors over the base.
//!
//! `design/themes/dusk.kdl` holds `theme "dusk" extends="default" kind="dark" { tokens { … } }`.
//! The file name is the theme's ID, so the list of themes is the list of files and is written
//! nowhere else. A theme sets only the colors (and the shadows) it changes; every other value is the
//! base's, so no value is written twice.

use super::*;
use crate::generate::theme_selector::is_word;

/// The folder of the built-in themes: one file per theme, `<id>.kdl`, and nothing else.
pub(crate) const THEMES_FOLDER: &str = "design/themes";
/// The ending of a theme file.
pub(super) const THEME_FILE_ENDING: &str = ".kdl";

/// Whether a file of the repository is one the theme is read from: the base or a theme file.
pub(crate) fn is_theme_source(path: &str) -> bool {
    path == THEME_PATH
        || path
            .strip_prefix(THEMES_FOLDER)
            .and_then(|rest| rest.strip_prefix('/'))
            .is_some_and(|name| name.ends_with(THEME_FILE_ENDING))
}

/// One file of `design/themes/`: where it is and what it says.
pub(crate) struct ThemeSource {
    pub(crate) path: String,
    pub(crate) text: String,
}

/// The files of `design/themes/`, in name order.
pub(crate) fn theme_sources(repo: &Repo) -> anyhow::Result<Vec<ThemeSource>> {
    crate::docs::source::sorted_files(repo, THEMES_FOLDER, |_| true)?
        .into_iter()
        .map(|name| {
            let path = format!("{THEMES_FOLDER}/{name}");
            Ok(ThemeSource {
                text: crate::source::read_text(repo, &path)?,
                path,
            })
        })
        .collect()
}

#[cfg(test)]
impl ThemeFile {
    /// A base text with the theme files of the repository: for the tests that change the base.
    pub(crate) fn parse_base(base: &str) -> anyhow::Result<Self> {
        Self::parse(base, &theme_sources(&Repo::discover()?)?)
    }
}

/// The name that `extends=` gives the base, `design/theme.kdl`: the only theme a theme extends.
pub(super) const BASE_THEME: &str = "default";
/// The key of a color's value for a device that asks for more contrast.
pub(super) const MORE: &str = "more";
/// The kinds of theme, in the order the themes are listed in (light ones first), which is also the
/// order of the two themes that start (`defaults light= dark=`).
pub(super) const KINDS: [&str; 2] = ["light", "dark"];

/// The theme's ID: the name of its file without `.kdl`, in small letters, digits and `-`.
fn theme_id(path: &str) -> anyhow::Result<&str> {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(THEME_FILE_ENDING)
        .filter(|id| is_word(id))
        .with_context(|| {
            format!(
                "{THEMES_FOLDER}/ holds one file per theme, named by its ID in small letters (`dusk{THEME_FILE_ENDING}`), and nothing else"
            )
        })
}

/// A value of a color (`#rrggbb`, in small letters, or `$other`, a color that names another) or
/// of a shadow.
pub(super) fn theme_value(kind: TokenKind, node: Node<'_>, value: &str) -> anyhow::Result<String> {
    check_text(node, value)?;
    if kind != TokenKind::Color {
        return Ok(value.to_owned());
    }
    let value = value.to_ascii_lowercase();
    let hex = value.len() == 7
        && value.starts_with('#')
        && value[1..].chars().all(|c| c.is_ascii_hexdigit());
    let link = value.strip_prefix('$').is_some_and(is_word);
    if hex || link {
        Ok(value)
    } else {
        Err(node.problem(format!(
            "`{value}` must look like #rrggbb, or name another color as `$name`"
        )))
    }
}

/// A `more=` that is the color itself: it says nothing, and only adds a line to the sheet.
pub(super) fn same_more(node: Node<'_>) -> anyhow::Error {
    node.problem(format!(
        "`{MORE}=` is the color itself; leave it out, the color stays the same with more contrast"
    ))
}

impl Builder {
    /// One theme file, read after the base.
    pub(super) fn add_theme(&mut self, source: &ThemeSource) -> anyhow::Result<()> {
        let id = theme_id(&source.path)?;
        let document = Document::parse(&source.text)?;
        let nodes = document.nodes();
        let [node] = nodes.as_slice() else {
            bail!(
                "holds one `theme \"{id}\" extends=\"{BASE_THEME}\" kind=\"…\" {{ tokens {{ … }} }}`, not {} lines",
                nodes.len()
            );
        };
        if node.name() != "theme" {
            return Err(node.problem(format!("a theme file holds one `theme \"{id}\"`")));
        }
        if node.only_arg()? != id {
            return Err(node.problem(format!(
                "is called `{id}` in `{id}{THEME_FILE_ENDING}`: the file name is the theme's ID"
            )));
        }
        if node.prop("extends")?.as_deref() != Some(BASE_THEME) {
            return Err(node.problem(format!(
                "needs `extends=\"{BASE_THEME}\"`: a theme sets colors over the base theme, {THEME_PATH}"
            )));
        }
        let kind = node.required_prop("kind")?;
        if !KINDS.contains(&kind.as_str()) {
            return Err(node.problem(format!("`kind` is {}", KINDS.join(" or "))));
        }
        if let Some((key, _)) = node
            .props()?
            .into_iter()
            .find(|(key, _)| key != "extends" && key != "kind")
        {
            return Err(node.problem(format!("`{key}=` is not a property of a theme")));
        }
        for child in node.children() {
            if child.name() != "tokens" {
                return Err(child.problem("a theme file holds `tokens` only"));
            }
            for line in child.children() {
                let value = self.theme_line(id, &source.path, line)?;
                self.theme_values.push(value);
            }
        }
        self.themes.push(Theme {
            name: id.to_owned(),
            kind,
            path: source.path.clone(),
        });
        Ok(())
    }

    /// `color "bg-canvas" "#1d2027" more="#000000"`: a color of the base, as this theme has it.
    fn theme_line(
        &self,
        theme: &str,
        path: &str,
        node: Node<'_>,
    ) -> anyhow::Result<(String, ThemeValue)> {
        let kind = TokenKind::of(node.name())
            .filter(|kind| kind.varies_by_theme())
            .ok_or_else(|| {
                node.problem(format!(
                    "a theme sets colors (and shadows) only; every other value is the base's, in {THEME_PATH}"
                ))
            })?;
        let [name, value] = node.args()?.try_into().map_err(|_| {
            node.problem("needs the name of a token of the base and its value in this theme")
        })?;
        if !self
            .tokens
            .iter()
            .any(|token| token.name == name && token.kind == kind)
        {
            return Err(node.problem(format!(
                "the base, {THEME_PATH}, has no {} token `{name}`",
                node.name()
            )));
        }
        if self
            .theme_values
            .iter()
            .any(|(known, value)| *known == name && value.theme == theme)
        {
            return Err(node.problem(format!("`{name}` is set twice")));
        }
        let mut more = None;
        for (key, value) in node.props()? {
            if key != MORE || kind != TokenKind::Color {
                return Err(node.problem(format!(
                    "`{key}=` is not a mode of a theme: a color has `{MORE}=`, the rest is the base's"
                )));
            }
            more = Some(theme_value(kind, node, &value)?);
        }
        let value = theme_value(kind, node, &value)?;
        if more.as_ref() == Some(&value) {
            return Err(same_more(node));
        }
        Ok((
            name,
            ThemeValue {
                theme: theme.to_owned(),
                value,
                more,
                path: path.to_owned(),
                line: node.line(),
            },
        ))
    }

    /// The tokens with a value of each color and shadow for every theme: the theme's own where its
    /// file sets one, the base's where it does not. The themes are listed light ones first, each
    /// kind in name order.
    pub(super) fn merge_themes(&mut self) -> Vec<Token> {
        let order = |theme: &Theme| {
            (
                KINDS.iter().position(|kind| *kind == theme.kind),
                theme.name.clone(),
            )
        };
        self.themes.sort_by_key(order);
        let mut tokens = std::mem::take(&mut self.tokens);
        for token in tokens
            .iter_mut()
            .filter(|token| token.kind.varies_by_theme())
        {
            let Some(base) = token.themes.pop() else {
                continue;
            };
            token.themes = self
                .themes
                .iter()
                .map(|theme| {
                    self.theme_values
                        .iter()
                        .find(|(name, value)| *name == token.name && value.theme == theme.name)
                        .map_or_else(
                            || ThemeValue {
                                theme: theme.name.clone(),
                                ..base.clone()
                            },
                            |(_, value)| value.clone(),
                        )
                })
                .collect();
        }
        tokens
    }
}
