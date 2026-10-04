//! `design/tokens.json` (W3C Design Tokens format) as typed data (ADR-0010).
//!
//! The generator and the contrast check both read it through here, so the file has one reader.

use anyhow::{Context as _, bail};
use serde_json::{Map, Value};

use super::contrast_spec::ContrastSpec;
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
    /// A length such as `14px`.
    Dimension(String),
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
    pub(crate) shadow: Vec<ShadowLayer>,
}

impl Theme {
    pub(crate) fn color(&self, name: &str) -> Option<&str> {
        self.colors
            .iter()
            .find(|(color, _)| color == name)
            .map(|(_, value)| value.as_str())
    }
}

pub(crate) struct TokenFile {
    pub(crate) base: Vec<BaseToken>,
    pub(crate) themes: Vec<Theme>,
    pub(crate) default_light: String,
    pub(crate) default_dark: String,
    /// Lines of explanation for the top of the generated CSS.
    pub(crate) css_header: Vec<String>,
    pub(crate) contrast: ContrastSpec,
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
            default_light: string(field(defaults, "light")?, "defaultThemes.light")?,
            default_dark: string(field(defaults, "dark")?, "defaultThemes.dark")?,
            css_header: string_list(field(extension, "cssHeader")?, "cssHeader")?,
            contrast,
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
        self.contrast.check_exclusions()?;
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
        other => bail!("base token `{name}` has unsupported type `{other}`"),
    };
    Ok(BaseToken {
        name: name.to_owned(),
        value,
    })
}

fn parse_theme(name: &str, theme: &Value) -> anyhow::Result<Theme> {
    let theme = object(theme, name)?;
    let extension = object(
        field(object(field(theme, "$extensions")?, name)?, "bugaboo")?,
        name,
    )?;
    let colors = object(field(theme, "color")?, name)?
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
        .collect::<anyhow::Result<Vec<_>>>()?;
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

fn string_list(value: &Value, what: &str) -> anyhow::Result<Vec<String>> {
    array(value, what)?
        .iter()
        .map(|item| string(item, what))
        .collect()
}
