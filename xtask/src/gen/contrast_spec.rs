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
    /// Smallest contrast ratio between a state and each surface (0 when the rule does not ask for one).
    pub(crate) min_against_surfaces: f64,
    /// Smallest contrast ratio between two states (0 when the rule does not ask for one).
    pub(crate) min_between_states: f64,
    /// Smallest difference in color (CIE 76 delta E) between a state and each surface. A ratio is
    /// blind to the hue, so a rule that has to tell a blue from a grey of the same brightness says this.
    pub(crate) min_delta_e_against_surfaces: Option<f64>,
    /// Smallest difference in color (delta E) between two states.
    pub(crate) min_delta_e_between_states: Option<f64>,
    /// Largest chroma (C*ab) of each state: the states are neutral greys, without a hue of their own.
    pub(crate) max_chroma_of_states: Option<f64>,
}

/// A text color on a fill that is brightened while it is pointed at (`filter: brightness`): the
/// pair is checked again as it is seen then.
pub(crate) struct Hovered {
    pub(crate) text: String,
    pub(crate) on: String,
    /// The number token with the brightness of the hover.
    pub(crate) brightness: String,
    /// The smallest ratio as it is, and with the device setting for more contrast.
    pub(crate) min: f64,
    pub(crate) min_more: f64,
}

impl Hovered {
    fn parse(node: Node<'_>) -> anyhow::Result<Self> {
        reason(node)?;
        Ok(Self {
            text: node.child("text")?.only_arg()?,
            on: node.child("on")?.only_arg()?,
            brightness: node.required_prop("brightness")?,
            min: node.number_prop("min")?,
            min_more: node.number_prop("min-more")?,
        })
    }
}

/// The page behind a sheet, dimmed by a scrim: the sheet must stand out from it and the text of the
/// page must be dim, in every theme, as it is drawn (the scrim mixed over the fill and over the text).
pub(crate) struct Dimmed {
    pub(crate) scrim: String,
    pub(crate) opacity: String,
    pub(crate) page: String,
    pub(crate) text: String,
    pub(crate) sheet: String,
    /// The least ratio of the sheet to the dimmed page in a light theme and in a dark one.
    pub(crate) min_sheet_light: f64,
    pub(crate) min_sheet_dark: f64,
    /// The most ratio that the dimmed text of the page may have against the dimmed page.
    pub(crate) max_text: f64,
}

impl Dimmed {
    fn parse(node: Node<'_>) -> anyhow::Result<Self> {
        reason(node)?;
        Ok(Self {
            scrim: node.required_prop("scrim")?,
            opacity: node.required_prop("opacity")?,
            page: node.required_prop("page")?,
            text: node.required_prop("text")?,
            sheet: node.required_prop("sheet")?,
            min_sheet_light: node.number_prop("min-sheet-light")?,
            min_sheet_dark: node.number_prop("min-sheet-dark")?,
            max_text: node.number_prop("max-text")?,
        })
    }
}

/// Text colors that need more than the usual ratio on some backgrounds, because small words in a
/// thin face are read on them all day (the muted words of a tinted row).
pub(crate) struct StrongerText {
    pub(crate) min: f64,
    pub(crate) text: Vec<String>,
    pub(crate) on: Vec<String>,
}

impl StrongerText {
    fn parse(node: Node<'_>) -> anyhow::Result<Self> {
        reason(node)?;
        Ok(Self {
            min: node.number_prop("min")?,
            text: node.words("text")?,
            on: node.words("on")?,
        })
    }
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
        let named = [
            "min-against-surfaces",
            "min-between-states",
            "min-delta-e-against-surfaces",
            "min-delta-e-between-states",
            "max-chroma-of-states",
        ];
        if !named
            .iter()
            .any(|key| node.prop(key).ok().flatten().is_some())
        {
            bail!(
                "line {}: a `distinct` rule names at least one minimum ({}), or it checks nothing",
                node.line(),
                named.join(", ")
            );
        }
        Ok(Self {
            groups,
            min_against_surfaces: optional_number(node, "min-against-surfaces")?.unwrap_or(0.0),
            min_between_states: optional_number(node, "min-between-states")?.unwrap_or(0.0),
            min_delta_e_against_surfaces: optional_number(node, "min-delta-e-against-surfaces")?,
            min_delta_e_between_states: optional_number(node, "min-delta-e-between-states")?,
            max_chroma_of_states: optional_number(node, "max-chroma-of-states")?,
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

/// Lines that people have to see (the edge of a box to tick), kept at a ratio on the plain surfaces
/// in every theme, also without the device setting for more contrast.
pub(crate) struct PlainLines {
    pub(crate) min: f64,
    pub(crate) colors: Vec<String>,
    pub(crate) on: Vec<String>,
    /// The opacity token that fades the line before it is seen (`faded-by`), when the line is.
    pub(crate) faded_by: Option<String>,
}

pub(crate) struct ContrastSpec {
    pub(crate) min: f64,
    pub(crate) plain_lines: Vec<PlainLines>,
    pub(crate) hovered: Vec<Hovered>,
    pub(crate) stronger: Vec<StrongerText>,
    pub(crate) dimmed: Vec<Dimmed>,
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
                self.plain_lines
                    .iter()
                    .flat_map(|lines| lines.colors.iter().chain(&lines.on)),
            )
            .chain(
                self.restricted
                    .iter()
                    .flat_map(|r| std::iter::once(&r.text).chain(&r.on)),
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
                    .flat_map(|rule| rule.text.iter().chain(&rule.on)),
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
        let mut plain_lines = Vec::new();
        let mut hovered = Vec::new();
        let mut stronger = Vec::new();
        let mut dimmed = Vec::new();
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
                "lines" => {
                    reason(child)?;
                    plain_lines.push(PlainLines {
                        min: child.number_prop("min")?,
                        colors: child.words("colors")?,
                        on: child.words("on")?,
                        faded_by: child.prop("faded-by")?,
                    });
                }
                "hovered" => hovered.push(Hovered::parse(child)?),
                "stronger" => stronger.push(StrongerText::parse(child)?),
                "dimmed" => dimmed.push(Dimmed::parse(child)?),
                "more" => more = Some(MoreSpec::parse(child)?),
                other => bail!("line {}: unknown contrast rule `{other}`", child.line()),
            }
        }
        Ok(Self {
            min: node.number_prop("min")?,
            plain_lines,
            hovered,
            stronger,
            dimmed,
            more: more.ok_or_else(|| node.problem("needs a `more` rule"))?,
            text: node.words("text")?,
            backgrounds: node.words("backgrounds")?,
            restricted,
            exclusions,
            non_text,
        })
    }
}

/// A pair that is a number, when the node has it.
fn optional_number(node: Node<'_>, key: &str) -> anyhow::Result<Option<f64>> {
    node.prop(key)?
        .map(|text| {
            text.parse()
                .map_err(|_| node.problem(format!("`{key}` must be a number")))
        })
        .transpose()
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

/// A group of lines faded by the opacity of a disabled control, for the tests of `faded-by`: the theme file has none
/// since a button that cannot be pressed is a grey chip (M5 exit F-A9). Written before the `more` rule of the file.
#[cfg(test)]
pub(crate) const FADED_LINES_EXAMPLE: &str =
    "    lines min=2 faded-by=\"opacity-disabled\" reason=\"A faded edge.\" {
        colors \"fg-muted\"
        on \"bg-canvas\" \"bg-raised\"
    }
    // With the device setting for more contrast.";

/// The theme text with [`FADED_LINES_EXAMPLE`] in it.
#[cfg(test)]
pub(crate) fn with_faded_lines(text: &str) -> String {
    text.replacen(
        "    // With the device setting for more contrast.",
        FADED_LINES_EXAMPLE,
        1,
    )
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
