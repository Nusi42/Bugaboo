//! `design/tokens.json` (W3C Design Tokens format) as typed data (ADR-0010).
//!
//! The generator and the contrast check both read it through here, so the file has one reader.

use anyhow::{Context as _, bail};
use serde_json::{Map, Value};

use super::contrast_spec::{ContrastSpec, DistinctSpec};
use crate::repo::Repo;

/// Where the token file lives, relative to the repository root.
pub(crate) const TOKENS_PATH: &str = "design/tokens.json";

/// A token that is the same in every theme.
pub(crate) struct BaseToken {
    /// The CSS variable name without `--`.
    pub(crate) name: String,
    pub(crate) value: BaseValue,
}

pub(crate) enum BaseValue {
    /// A font stack, first choice first.
    FontFamily(Vec<String>),
    /// A length such as `14px`, possibly scaled by another token (`calc(4px * var(--density))`).
    Dimension(String),
    /// A duration such as `120ms` or a plain number such as `1`.
    Plain(String),
}

/// A choice that changes token values without changing the theme: spacing, text size, motion.
pub(crate) struct Variant {
    pub(crate) name: String,
    /// The attribute on `<html>` that switches it on, and its value.
    pub(crate) attribute: String,
    pub(crate) value: String,
    /// A device setting that switches it on as well (`(prefers-reduced-motion: reduce)`).
    pub(crate) media: Option<String>,
    /// Token name (without `--`) and the value it has while the variant is on.
    pub(crate) tokens: Vec<(String, String)>,
}

/// One layer of a shadow.
pub(crate) struct ShadowLayer {
    pub(crate) color: String,
    pub(crate) offset_x: String,
    pub(crate) offset_y: String,
    pub(crate) blur: String,
    pub(crate) spread: String,
}

/// One built-in theme.
pub(crate) struct Theme {
    pub(crate) name: String,
    /// `light` or `dark`; decides the browser's own controls and scrollbars.
    pub(crate) color_scheme: String,
    /// Color name and `#RRGGBB` value, in file order.
    pub(crate) colors: Vec<(String, String)>,
    /// The colors that change when the device asks for more contrast (`prefers-contrast: more`).
    pub(crate) more_contrast: Vec<(String, String)>,
    pub(crate) shadow: Vec<ShadowLayer>,
}

impl Theme {
    pub(crate) fn color(&self, name: &str) -> Option<&str> {
        self.colors
            .iter()
            .find(|(color, _)| color == name)
            .map(|(_, value)| value.as_str())
    }

    /// The colors with more contrast: the theme's own, where `contrastMore` replaces them.
    pub(crate) fn color_with_more_contrast(&self, name: &str) -> Option<&str> {
        self.more_contrast
            .iter()
            .find(|(color, _)| color == name)
            .map(|(_, value)| value.as_str())
            .or_else(|| self.color(name))
    }
}

pub(crate) struct TokenFile {
    pub(crate) base: Vec<BaseToken>,
    pub(crate) themes: Vec<Theme>,
    pub(crate) variants: Vec<Variant>,
    pub(crate) default_light: String,
    pub(crate) default_dark: String,
    pub(crate) contrast: ContrastSpec,
    pub(crate) distinct: DistinctSpec,
}

impl TokenFile {
    pub(crate) fn load(repo: &Repo) -> anyhow::Result<Self> {
        let path = repo.path(TOKENS_PATH);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("{TOKENS_PATH} is not valid"))
    }

    pub(crate) fn parse(text: &str) -> anyhow::Result<Self> {
        let root: Value = serde_json::from_str(text)?;
        let root = object(&root, "the file")?;
        let base = object(field(root, "base")?, "base")?
            .iter()
            .map(|(name, token)| parse_base(name, token))
            .collect::<anyhow::Result<Vec<_>>>()?;
        let themes = object(field(root, "themes")?, "themes")?
            .iter()
            .map(|(name, theme)| parse_theme(name, theme))
            .collect::<anyhow::Result<Vec<_>>>()?;
        let variants = object(field(root, "variants")?, "variants")?
            .iter()
            .filter(|(name, _)| !name.starts_with('$'))
            .map(|(name, variant)| parse_variant(name, variant))
            .collect::<anyhow::Result<Vec<_>>>()?;
        let extension = object(
            field(
                object(field(root, "$extensions")?, "$extensions")?,
                "bugaboo",
            )?,
            "$extensions.bugaboo",
        )?;
        let defaults = object(field(extension, "defaultThemes")?, "defaultThemes")?;
        let contrast = ContrastSpec::parse(field(extension, "contrast")?)?;
        let file = Self {
            base,
            themes,
            variants,
            default_light: string(field(defaults, "light")?, "defaultThemes.light")?,
            default_dark: string(field(defaults, "dark")?, "defaultThemes.dark")?,
            contrast,
            distinct: DistinctSpec::parse(field(extension, "distinct")?)?,
        };
        file.validate()?;
        Ok(file)
    }

    fn validate(&self) -> anyhow::Result<()> {
        let Some(first) = self.themes.first() else {
            bail!("there are no themes");
        };
        for theme in &self.themes {
            if theme.colors.len() != first.colors.len()
                || theme
                    .colors
                    .iter()
                    .zip(&first.colors)
                    .any(|(a, b)| a.0 != b.0)
            {
                bail!(
                    "theme `{}` must define exactly the same colors, in the same order, as `{}`",
                    theme.name,
                    first.name
                );
            }
        }
        for name in [&self.default_light, &self.default_dark] {
            if !self.themes.iter().any(|theme| &theme.name == name) {
                bail!("default theme `{name}` is not one of the themes");
            }
        }
        if let Some(unknown) = self
            .contrast
            .named_colors()
            .find(|name| first.color(name).is_none())
        {
            bail!("contrast rules use unknown color `{unknown}`");
        }
        if let Some((name, _)) = first
            .colors
            .iter()
            .find(|(name, _)| !self.contrast.classifies(name))
        {
            bail!(
                "color `{name}` is not classified: list it under contrast `text`, `backgrounds` or `nonText` (with a reason)"
            );
        }
        for theme in &self.themes {
            if let Some((name, _)) = theme
                .more_contrast
                .iter()
                .find(|(name, _)| theme.color(name).is_none())
            {
                bail!(
                    "`contrastMore` of theme `{}` changes unknown color `{name}`",
                    theme.name
                );
            }
        }
        for variant in &self.variants {
            if let Some((name, _)) = variant
                .tokens
                .iter()
                .find(|(name, _)| !self.base.iter().any(|token| &token.name == name))
            {
                bail!("variant `{}` changes unknown token `{name}`", variant.name);
            }
        }
        self.contrast.check_exclusions()?;
        let distinct = &self.distinct;
        if let Some(unknown) = distinct
            .states
            .iter()
            .chain(&distinct.surfaces)
            .find(|name| first.color(name).is_none())
        {
            bail!("distinct rules use unknown color `{unknown}`");
        }
        Ok(())
    }
}

fn parse_base(name: &str, token: &Value) -> anyhow::Result<BaseToken> {
    let token = object(token, name)?;
    let kind = string(field(token, "$type")?, name)?;
    let value = field(token, "$value")?;
    let value = match kind.as_str() {
        "fontFamily" => BaseValue::FontFamily(
            array(value, name)?
                .iter()
                .map(|item| string(item, name))
                .collect::<anyhow::Result<_>>()?,
        ),
        "dimension" => BaseValue::Dimension(string(value, name)?),
        "duration" | "number" => BaseValue::Plain(string(value, name)?),
        other => bail!("base token `{name}` has unsupported type `{other}`"),
    };
    Ok(BaseToken {
        name: name.to_owned(),
        value,
    })
}

fn parse_variant(name: &str, variant: &Value) -> anyhow::Result<Variant> {
    let variant = object(variant, name)?;
    let tokens = object(field(variant, "tokens")?, name)?
        .iter()
        .map(|(token, value)| Ok((token.clone(), string(value, token)?)))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(Variant {
        name: name.to_owned(),
        attribute: string(field(variant, "attribute")?, name)?,
        value: string(field(variant, "value")?, name)?,
        media: variant
            .get("media")
            .map(|media| string(media, name))
            .transpose()?,
        tokens,
    })
}

/// A list of `#RRGGBB` colors by name, as the `color` group of a theme holds them.
fn parse_colors(name: &str, group: &Value) -> anyhow::Result<Vec<(String, String)>> {
    object(group, name)?
        .iter()
        .map(|(color, token)| {
            let value = string(field(object(token, color)?, "$value")?, color)?;
            let valid = value.len() == 7
                && value.starts_with('#')
                && value[1..].chars().all(|c| c.is_ascii_hexdigit());
            if !valid {
                bail!("color `{color}` in theme `{name}` must look like #RRGGBB, not `{value}`");
            }
            Ok((color.clone(), value.to_ascii_uppercase()))
        })
        .collect()
}

fn parse_theme(name: &str, theme: &Value) -> anyhow::Result<Theme> {
    let theme = object(theme, name)?;
    let extension = object(
        field(object(field(theme, "$extensions")?, name)?, "bugaboo")?,
        name,
    )?;
    let colors = parse_colors(name, field(theme, "color")?)?;
    let more_contrast = match theme.get("contrastMore") {
        Some(more) => parse_colors(name, field(object(more, name)?, "color")?)?,
        None => Vec::new(),
    };
    let shadow = array(
        field(object(field(theme, "shadow")?, "shadow")?, "$value")?,
        "shadow",
    )?
    .iter()
    .map(parse_shadow_layer)
    .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(Theme {
        name: name.to_owned(),
        color_scheme: string(field(extension, "colorScheme")?, name)?,
        colors,
        more_contrast,
        shadow,
    })
}

fn parse_shadow_layer(layer: &Value) -> anyhow::Result<ShadowLayer> {
    let layer = object(layer, "shadow layer")?;
    let part = |key: &str| string(field(layer, key)?, key);
    Ok(ShadowLayer {
        color: part("color")?,
        offset_x: part("offsetX")?,
        offset_y: part("offsetY")?,
        blur: part("blur")?,
        spread: part("spread")?,
    })
}

fn field<'a>(map: &'a Map<String, Value>, key: &str) -> anyhow::Result<&'a Value> {
    map.get(key).with_context(|| format!("`{key}` is missing"))
}

fn object<'a>(value: &'a Value, what: &str) -> anyhow::Result<&'a Map<String, Value>> {
    value
        .as_object()
        .with_context(|| format!("`{what}` must be an object"))
}

fn array<'a>(value: &'a Value, what: &str) -> anyhow::Result<&'a Vec<Value>> {
    value
        .as_array()
        .with_context(|| format!("`{what}` must be a list"))
}

fn string(value: &Value, what: &str) -> anyhow::Result<String> {
    value
        .as_str()
        .map(str::to_owned)
        .with_context(|| format!("`{what}` must be text"))
}
