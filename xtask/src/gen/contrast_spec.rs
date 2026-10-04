//! The contrast rules of `design/tokens.json`: which text colors must be readable on which
//! backgrounds (WCAG AA, THM-03, UX-04).
//!
//! Every text color is checked on every background, except the restricted and excluded
//! combinations. Each of those is written down with its reason in the token file.

use anyhow::{Context as _, bail};
use serde_json::{Map, Value};

/// A text color that is only drawn on some backgrounds.
pub(crate) struct Restricted {
    pub(crate) text: String,
    pub(crate) on: Vec<String>,
}

/// Text and background combinations that are never used.
pub(crate) struct Exclusion {
    /// Color names; `*` matches any characters.
    pub(crate) text: Vec<String>,
    pub(crate) on: Vec<String>,
    /// Themes the exclusion applies to; empty means all.
    pub(crate) themes: Vec<String>,
}

pub(crate) struct ContrastSpec {
    pub(crate) min: f64,
    pub(crate) text: Vec<String>,
    pub(crate) backgrounds: Vec<String>,
    pub(crate) restricted: Vec<Restricted>,
    pub(crate) exclusions: Vec<Exclusion>,
    /// Colors that are neither text nor a background for text (borders), each with a reason.
    pub(crate) non_text: Vec<String>,
}

impl ContrastSpec {
    /// The (text, background) pairs to check in `theme`.
    pub(crate) fn pairs(&self, theme: &str) -> Vec<(&str, &str)> {
        let mut pairs = Vec::new();
        for text in &self.text {
            if let Some(restricted) = self.restricted.iter().find(|r| &r.text == text) {
                pairs.extend(restricted.on.iter().map(|on| (text.as_str(), on.as_str())));
                continue;
            }
            for background in &self.backgrounds {
                let excluded = self.exclusions.iter().any(|exclusion| {
                    (exclusion.themes.is_empty() || exclusion.themes.iter().any(|t| t == theme))
                        && exclusion.text.iter().any(|p| glob(p, text))
                        && exclusion.on.iter().any(|p| glob(p, background))
                });
                if !excluded {
                    pairs.push((text.as_str(), background.as_str()));
                }
            }
        }
        pairs
    }

    /// Every color name used anywhere in the rules (exact names, no patterns).
    pub(crate) fn named_colors(&self) -> impl Iterator<Item = &String> {
        self.text.iter().chain(&self.backgrounds).chain(
            self.restricted
                .iter()
                .flat_map(|r| std::iter::once(&r.text).chain(&r.on)),
        )
    }

    /// Whether the rules say what this color is: text, a background, or not text at all.
    pub(crate) fn classifies(&self, color: &str) -> bool {
        self.text
            .iter()
            .chain(&self.backgrounds)
            .chain(&self.non_text)
            .any(|name| name == color)
    }

    /// Fails when an exclusion matches nothing (it is stale) or names no color at all.
    pub(crate) fn check_exclusions(&self) -> anyhow::Result<()> {
        for exclusion in &self.exclusions {
            let all_match = |patterns: &[String], names: &[String]| {
                patterns
                    .iter()
                    .all(|pattern| names.iter().any(|name| glob(pattern, name)))
            };
            if !all_match(&exclusion.text, &self.text)
                || !all_match(&exclusion.on, &self.backgrounds)
            {
                bail!(
                    "a contrast exclusion matches no color: {:?}",
                    exclusion.text
                );
            }
        }
        Ok(())
    }

    pub(crate) fn parse(spec: &Value) -> anyhow::Result<Self> {
        let spec = spec.as_object().context("`contrast` must be an object")?;
        let mut restricted = Vec::new();
        for entry in list(spec, "restricted")? {
            let entry = entry.as_object().context("`restricted` holds objects")?;
            reason(entry)?;
            restricted.push(Restricted {
                text: text(entry, "text")?,
                on: names(entry, "on")?,
            });
        }
        let mut exclusions = Vec::new();
        for entry in list(spec, "exclusions")? {
            let entry = entry.as_object().context("`exclusions` holds objects")?;
            reason(entry)?;
            exclusions.push(Exclusion {
                text: names(entry, "text")?,
                on: names(entry, "on")?,
                themes: if entry.contains_key("themes") {
                    names(entry, "themes")?
                } else {
                    Vec::new()
                },
            });
        }
        let mut non_text = Vec::new();
        for entry in list(spec, "nonText")? {
            let entry = entry.as_object().context("`nonText` holds objects")?;
            reason(entry)?;
            non_text.extend(names(entry, "colors")?);
        }
        Ok(Self {
            min: spec
                .get("min")
                .and_then(Value::as_f64)
                .context("`min` of the contrast rules must be a number")?,
            text: names(spec, "text")?,
            backgrounds: names(spec, "backgrounds")?,
            restricted,
            exclusions,
            non_text,
        })
    }
}

fn list<'a>(map: &'a Map<String, Value>, key: &str) -> anyhow::Result<&'a Vec<Value>> {
    map.get(key)
        .and_then(Value::as_array)
        .with_context(|| format!("`{key}` must be a list"))
}

fn text(map: &Map<String, Value>, key: &str) -> anyhow::Result<String> {
    map.get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .with_context(|| format!("`{key}` must be text"))
}

fn names(map: &Map<String, Value>, key: &str) -> anyhow::Result<Vec<String>> {
    list(map, key)?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .with_context(|| format!("`{key}` must hold text"))
        })
        .collect()
}

fn reason(entry: &Map<String, Value>) -> anyhow::Result<()> {
    if text(entry, "reason")?.trim().is_empty() {
        bail!("every contrast restriction and exclusion needs a reason");
    }
    Ok(())
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
