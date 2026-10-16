//! The contrast rules of `design/theme.kdl`: which text colors must be readable on which
//! backgrounds (WCAG AA, THM-03, UX-04).
//!
//! Every text color is checked on every background, except the restricted and excluded
//! combinations. Each of those is written down with its reason in the theme file.

use anyhow::bail;

use super::kdl_reader::Node;

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

/// State backgrounds (hover, selected) that must stay visible against the surfaces.
pub(crate) struct DistinctSpec {
    /// The states and the surfaces they must stay visible on: one group for the plain surfaces, one
    /// for each kind of tinted row. A rule has one group of its own, or `group { … }` children.
    pub(crate) groups: Vec<DistinctGroup>,
    /// Smallest contrast ratio between a state and each surface.
    pub(crate) min_against_surfaces: f64,
    /// Smallest contrast ratio between two states.
    pub(crate) min_between_states: f64,
}

/// Some states and the surfaces they sit on.
pub(crate) struct DistinctGroup {
    pub(crate) states: Vec<String>,
    pub(crate) surfaces: Vec<String>,
}

impl DistinctGroup {
    fn parse(node: Node<'_>) -> anyhow::Result<Self> {
        Ok(Self {
            states: node.words("states")?,
            surfaces: node.words("surfaces")?,
        })
    }
}

impl DistinctSpec {
    pub(crate) fn parse(node: Node<'_>) -> anyhow::Result<Self> {
        reason(node)?;
        let children: Vec<Node<'_>> = node
            .children()
            .into_iter()
            .filter(|child| child.name() == "group")
            .collect();
        let groups = if children.is_empty() {
            vec![DistinctGroup::parse(node)?]
        } else {
            children
                .into_iter()
                .map(DistinctGroup::parse)
                .collect::<anyhow::Result<Vec<_>>>()?
        };
        Ok(Self {
            groups,
            min_against_surfaces: node.number_prop("min-against-surfaces")?,
            min_between_states: node.number_prop("min-between-states")?,
        })
    }
}

/// The stronger rules for people whose device asks for more contrast (`prefers-contrast: more`).
pub(crate) struct MoreSpec {
    /// Smallest contrast ratio of the listed text colors on their backgrounds.
    pub(crate) min_text: f64,
    /// Smallest contrast ratio of the listed line colors on the plain surfaces.
    pub(crate) min_line: f64,
    pub(crate) text: Vec<String>,
    pub(crate) lines: Vec<String>,
    pub(crate) line_on: Vec<String>,
}

impl MoreSpec {
    fn parse(node: Node<'_>) -> anyhow::Result<Self> {
        reason(node)?;
        Ok(Self {
            min_text: node.number_prop("min-text")?,
            min_line: node.number_prop("min-line")?,
            text: node.words("text")?,
            lines: node.words("lines")?,
            line_on: node.words("line-on")?,
        })
    }
}

pub(crate) struct ContrastSpec {
    pub(crate) min: f64,
    pub(crate) more: MoreSpec,
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
        self.text
            .iter()
            .chain(&self.backgrounds)
            .chain(&self.more.text)
            .chain(&self.more.lines)
            .chain(&self.more.line_on)
            .chain(
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

    pub(crate) fn parse(node: Node<'_>) -> anyhow::Result<Self> {
        let mut restricted = Vec::new();
        let mut exclusions = Vec::new();
        let mut non_text = Vec::new();
        let mut more = None;
        for child in node.children() {
            match child.name() {
                "text" | "backgrounds" => {}
                "restricted" => {
                    reason(child)?;
                    restricted.push(Restricted {
                        text: child.only_arg()?,
                        on: child
                            .required_prop("on")
                            .map(|on| on.split_whitespace().map(str::to_owned).collect())?,
                    });
                }
                "exclude" => {
                    reason(child)?;
                    exclusions.push(Exclusion {
                        text: child.words("text")?,
                        on: child.words("on")?,
                        themes: child
                            .children()
                            .iter()
                            .find(|inner| inner.name() == "themes")
                            .map(Node::args)
                            .transpose()?
                            .unwrap_or_default(),
                    });
                }
                "non-text" => {
                    reason(child)?;
                    non_text.extend(child.args()?);
                }
                "more" => more = Some(MoreSpec::parse(child)?),
                other => bail!("line {}: unknown contrast rule `{other}`", child.line()),
            }
        }
        Ok(Self {
            min: node.number_prop("min")?,
            more: more.ok_or_else(|| node.problem("needs a `more` rule"))?,
            text: node.words("text")?,
            backgrounds: node.words("backgrounds")?,
            restricted,
            exclusions,
            non_text,
        })
    }
}

fn reason(node: Node<'_>) -> anyhow::Result<()> {
    if node.required_prop("reason")?.trim().is_empty() {
        bail!(
            "line {}: every contrast restriction and exclusion needs a reason",
            node.line()
        );
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
