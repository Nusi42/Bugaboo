//! The files of `design/themes/`: one built-in theme each, which sets colors over the base.
//!
//! `design/themes/dusk.kdl` holds `theme "dusk" extends="default" kind="dark" { tokens { … } }`.
//! The file name is the theme's ID, so the list of themes is the list of files and is written
//! nowhere else. A theme sets only the colors (and the shadows) it changes; every other value is the
//! base's, so no value is written twice.

use super::*;

/// The name that `extends=` gives the base, `design/theme.kdl`: the only theme a theme extends.
pub(super) const BASE_THEME: &str = "default";
/// The key of a color's value for a device that asks for more contrast.
pub(super) const MORE: &str = "more";
/// The kinds of theme, in the order the themes are listed in (light ones first).
const KINDS: [&str; 2] = ["light", "dark"];

/// The theme's ID: the name of its file without `.kdl`, in small letters, digits and `-`.
fn theme_id(path: &str) -> anyhow::Result<&str> {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(THEME_FILE_ENDING)
        .filter(|id| {
            id.starts_with(|c: char| c.is_ascii_lowercase())
                && id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
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
    let link = value.strip_prefix('$').is_some_and(|name| {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    });
    if hex || link {
        Ok(value)
    } else {
        Err(node.problem(format!(
            "`{value}` must look like #rrggbb, or name another color as `$name`"
        )))
    }
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
        Ok((
            name,
            ThemeValue {
                theme: theme.to_owned(),
                value: theme_value(kind, node, &value)?,
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
