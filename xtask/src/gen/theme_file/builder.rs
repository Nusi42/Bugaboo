//! Building the theme from the nodes of the KDL file.

use super::*;

#[derive(Default)]
pub(super) struct Builder {
    pub(super) themes: Vec<Theme>,
    pub(super) defaults: Option<(String, String)>,
    pub(super) frame: Option<Frame>,
    pub(super) variants: Vec<Variant>,
    pub(super) tokens: Vec<Token>,
    pub(super) mixins: Vec<Mixin>,
    pub(super) keyframes: Vec<Keyframes>,
    pub(super) rules: Vec<Rule>,
    pub(super) docs: Vec<DocSheet>,
    pub(super) contrast: Option<ContrastSpec>,
    pub(super) distinct: Vec<DistinctSpec>,
    pub(super) bold: Option<BoldSpec>,
}

impl Builder {
    pub(super) fn add(&mut self, node: Node<'_>) -> anyhow::Result<()> {
        match node.name() {
            "defaults" => {
                self.defaults = Some((node.required_prop("light")?, node.required_prop("dark")?));
            }
            "frame" => {
                self.frame = Some(Frame {
                    look: node.required_prop("look")?,
                    mode: node.required_prop("mode")?,
                    preview: node.required_prop("preview")?,
                });
            }
            "themes" => {
                for child in node.children() {
                    if child.name() != "theme" {
                        bail!("line {}: `themes` holds `theme` lines only", child.line());
                    }
                    self.themes.push(Theme {
                        name: child.only_arg()?,
                        kind: child.required_prop("kind")?,
                    });
                }
            }
            "variants" => {
                for child in node.children() {
                    self.variants.push(variant(child)?);
                }
            }
            "tokens" => {
                for child in node.children() {
                    let token = self.token(child)?;
                    self.tokens.push(token);
                }
            }
            "contrast" => self.contrast = Some(ContrastSpec::parse(node)?),
            "distinct" => self.distinct.push(DistinctSpec::parse(node)?),
            "bold" => self.bold = Some(BoldSpec::parse(node)?),
            "mixin" => {
                let name = node.only_arg()?;
                if self.mixins.iter().any(|mixin| mixin.name == name) {
                    return Err(node.problem(format!("the mixin `{name}` is defined twice")));
                }
                self.mixins.push(Mixin {
                    name,
                    items: items(node)?,
                });
            }
            "keyframes" => {
                let frames = keyframes(node)?;
                if self.keyframes.iter().any(|known| known.name == frames.name) {
                    return Err(
                        node.problem(format!("the keyframes `{}` are defined twice", frames.name))
                    );
                }
                self.keyframes.push(frames);
            }
            "docs" => {
                let sheet = self.doc_sheet(node)?;
                self.docs.push(sheet);
            }
            "base" => self.rules.push(rule(node, RuleKind::Base)?),
            "part" => self.rules.push(rule(node, RuleKind::Part)?),
            other => bail!("line {}: unknown line `{other}`", node.line()),
        }
        Ok(())
    }

    /// `docs "components" description="…" { tokens { … } rule "…" { … } keyframes "…" { … } }`.
    pub(super) fn doc_sheet(&self, node: Node<'_>) -> anyhow::Result<DocSheet> {
        let name = node.only_arg()?;
        if !DOC_SHEETS.iter().any(|(known, _)| *known == name) {
            return Err(node.problem(format!(
                "`{name}` is not a sheet of the design pages; use one of {}",
                DOC_SHEETS.map(|(known, _)| known).join(", ")
            )));
        }
        if self.docs.iter().any(|sheet| sheet.name == name) {
            return Err(node.problem(format!("the sheet `{name}` is defined twice")));
        }
        let mut sheet = DocSheet {
            name,
            description: node.required_prop("description")?,
            tokens: Vec::new(),
            rules: Vec::new(),
            keyframes: Vec::new(),
        };
        for child in node.children() {
            match child.name() {
                "tokens" => {
                    for line in child.children() {
                        let token = self.doc_token(&sheet, line)?;
                        sheet.tokens.push(token);
                    }
                }
                "rule" => sheet.rules.push(rule(child, RuleKind::Doc)?),
                "keyframes" => {
                    let frames = keyframes(child)?;
                    if sheet
                        .keyframes
                        .iter()
                        .any(|known| known.name == frames.name)
                    {
                        return Err(child.problem(format!(
                            "the keyframes `{}` are defined twice",
                            frames.name
                        )));
                    }
                    sheet.keyframes.push(frames);
                }
                other => bail!("line {}: unknown line `{other}` in `docs`", child.line()),
            }
        }
        Ok(sheet)
    }

    /// A token of the design pages: the same in every theme, named `doc-…`.
    pub(super) fn doc_token(&self, sheet: &DocSheet, node: Node<'_>) -> anyhow::Result<Token> {
        let token = self.token(node)?;
        if !token.name.starts_with("doc-") {
            return Err(node.problem("a token of the design pages is named `doc-…`"));
        }
        if token.same.is_none() || token.forced.is_some() || !token.variants.is_empty() {
            return Err(
                node.problem("a token of the design pages has one value, the same everywhere")
            );
        }
        if self
            .docs
            .iter()
            .chain(std::iter::once(sheet))
            .any(|other| other.tokens.iter().any(|known| known.name == token.name))
        {
            return Err(node.problem(format!("`{}` is defined twice", token.name)));
        }
        Ok(token)
    }

    pub(super) fn token(&self, node: Node<'_>) -> anyhow::Result<Token> {
        let kind = TokenKind::of(node.name()).ok_or_else(|| {
            let kinds: Vec<&str> = TOKEN_KINDS.iter().map(|(name, _)| *name).collect();
            node.problem(format!(
                "is not a kind of token; use one of {}",
                kinds.join(", ")
            ))
        })?;
        let args = node.args()?;
        let name = args
            .first()
            .cloned()
            .ok_or_else(|| node.problem("needs a name"))?;
        let values = &args[1..];
        let same = match (kind, values) {
            (TokenKind::Color | TokenKind::Shadow, []) => None,
            (TokenKind::Color | TokenKind::Shadow, _) => {
                return Err(
                    node.problem("has a value for each theme (`daylight=…`), not one value")
                );
            }
            (TokenKind::Font, families) if !families.is_empty() => Some(font_stack(families)),
            (_, [one]) if kind != TokenKind::Font => Some(one.clone()),
            _ => return Err(node.problem("needs one value after its name")),
        };
        let mut token = Token {
            name,
            kind,
            same,
            themes: Vec::new(),
            more: Vec::new(),
            forced: None,
            variants: Vec::new(),
            run_time: false,
            line: node.line(),
        };
        if self.tokens.iter().any(|other| other.name == token.name) {
            return Err(node.problem(format!("`{}` is defined twice", token.name)));
        }
        for (key, value) in node.props()? {
            if key == "run-time" {
                if value != "true" || token.kind != TokenKind::Size {
                    return Err(
                        node.problem("`run-time=\"true\"` marks a size, and says nothing else")
                    );
                }
                token.run_time = true;
            } else {
                self.mode_value(&mut token, node, &key, value)?;
            }
        }
        self.check_token(&token, node)?;
        Ok(token)
    }

    /// Puts the value of one mode (`daylight=`, `dusk-more=`, `forced=`, `compact=`) in the token.
    pub(super) fn mode_value(
        &self,
        token: &mut Token,
        node: Node<'_>,
        key: &str,
        value: String,
    ) -> anyhow::Result<()> {
        let value = if token.kind == TokenKind::Color && key != "forced" {
            value.to_ascii_lowercase()
        } else {
            value
        };
        check_text(node, &value)?;
        if key == "forced" {
            if token.kind == TokenKind::Color
                && !SYSTEM_COLORS
                    .iter()
                    .any(|color| color.eq_ignore_ascii_case(&value))
            {
                return Err(node.problem(format!(
                    "`forced={value}` must be a CSS system color (Highlight, ButtonText, …)"
                )));
            }
            token.forced = Some(value);
        } else if self.themes.iter().any(|theme| theme.name == key) {
            token.themes.push((key.to_owned(), value));
        } else if let Some(theme) = key
            .strip_suffix("-more")
            .filter(|theme| self.themes.iter().any(|known| known.name == *theme))
        {
            token.more.push((theme.to_owned(), value));
        } else if self.variants.iter().any(|variant| variant.name == key) {
            token.variants.push((key.to_owned(), value));
        } else {
            return Err(node.problem(format!(
                "`{key}=` is not a mode: use a theme, `<theme>-more`, `forced` or a choice from `variants`"
            )));
        }
        Ok(())
    }

    pub(super) fn check_token(&self, token: &Token, node: Node<'_>) -> anyhow::Result<()> {
        if token.kind.varies_by_theme() {
            let names: Vec<&str> = token.themes.iter().map(|(name, _)| name.as_str()).collect();
            let wanted: Vec<&str> = self
                .themes
                .iter()
                .map(|theme| theme.name.as_str())
                .collect();
            if names != wanted {
                return Err(node.problem(format!(
                    "needs a value for each theme, in this order: {}",
                    wanted.join(", ")
                )));
            }
        } else if !token.themes.is_empty() || !token.more.is_empty() {
            return Err(node.problem("only colors and shadows differ by theme"));
        }
        if token.kind == TokenKind::Color {
            for (_, value) in token.themes.iter().chain(&token.more) {
                let valid = value.len() == 7
                    && value.starts_with('#')
                    && value[1..].chars().all(|c| c.is_ascii_hexdigit());
                if !valid {
                    return Err(node.problem(format!("`{value}` must look like #rrggbb")));
                }
            }
        }
        if !token.more.is_empty() && token.kind != TokenKind::Color {
            return Err(node.problem("only colors have a value for more contrast"));
        }
        if !token.variants.is_empty() && token.same.is_none() {
            return Err(
                node.problem("a choice can only change a token that is the same in every theme")
            );
        }
        Ok(())
    }

    pub(super) fn finish(self) -> anyhow::Result<ThemeFile> {
        let (default_light, default_dark) = self.defaults.context("`defaults` is missing")?;
        let file = ThemeFile {
            themes: self.themes,
            default_light,
            default_dark,
            frame: self.frame.context("`frame` is missing")?,
            variants: self.variants,
            tokens: self.tokens,
            mixins: self.mixins,
            keyframes: self.keyframes,
            rules: self.rules,
            docs: self.docs,
            contrast: self.contrast.context("`contrast` is missing")?,
            bold: self.bold.context("`bold` is missing")?,
            distinct: if self.distinct.is_empty() {
                bail!("`distinct` is missing")
            } else {
                self.distinct
            },
        };
        file.validate()?;
        Ok(file)
    }
}
