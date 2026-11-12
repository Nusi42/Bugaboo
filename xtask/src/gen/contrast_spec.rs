//! The reader of the contrast rules of `design/theme.kdl`. The rules themselves, as data, and the
//! maths that judges them live in `bb_core::contrast` (ADR-0026, update M8); only the reading of
//! the KDL text stays here, because the server and the browser never parse KDL.
//!
//! Every text color is checked on every background, except the restricted and excluded
//! combinations. Each of those is written down with its reason in the theme file.

use anyhow::bail;
pub(crate) use bb_core::contrast::{
    ContrastSpec, Dimmed, DistinctGroup, DistinctSpec, Drawn, Exclusion, Hovered, MoreSpec,
    PlainLines, Restricted, StrongerText,
};
use bb_core::contrast::{List, Text};

use super::kdl_reader::Node;

fn read_hovered(node: Node<'_>) -> anyhow::Result<Hovered> {
    reason(node)?;
    Ok(Hovered {
        text: node.child("text")?.only_arg()?.into(),
        on: node.child("on")?.only_arg()?.into(),
        brightness: node.required_prop("brightness")?.into(),
        min: node.number_prop("min")?,
        min_more: node.number_prop("min-more")?,
    })
}

fn read_dimmed(node: Node<'_>) -> anyhow::Result<Dimmed> {
    reason(node)?;
    Ok(Dimmed {
        scrim: node.required_prop("scrim")?.into(),
        opacity: node.required_prop("opacity")?.into(),
        page: node.required_prop("page")?.into(),
        text: node.required_prop("text")?.into(),
        sheet: node.required_prop("sheet")?.into(),
        min_sheet_light: node.number_prop("min-sheet-light")?,
        min_sheet_dark: node.number_prop("min-sheet-dark")?,
        max_text: node.number_prop("max-text")?,
    })
}

fn read_stronger(node: Node<'_>) -> anyhow::Result<StrongerText> {
    reason(node)?;
    Ok(StrongerText {
        min: node.number_prop("min")?,
        text: texts(node.words("text")?),
        on: texts(node.words("on")?),
    })
}

fn group(node: Node<'_>) -> anyhow::Result<DistinctGroup> {
    Ok(DistinctGroup {
        states: texts(node.words("states")?),
        surfaces: texts(node.words("surfaces")?),
    })
}

fn read_more(node: Node<'_>) -> anyhow::Result<MoreSpec> {
    reason(node)?;
    Ok(MoreSpec {
        min_text: node.number_prop("min-text")?,
        min_line: node.number_prop("min-line")?,
        text: texts(node.words("text")?),
        lines: texts(node.words("lines")?),
        line_on: texts(node.words("line-on")?),
    })
}

pub(crate) fn distinct(node: Node<'_>) -> anyhow::Result<DistinctSpec> {
    reason(node)?;
    let children: Vec<Node<'_>> = node
        .children()
        .into_iter()
        .filter(|child| child.name() == "group")
        .collect();
    let groups = if children.is_empty() {
        vec![group(node)?]
    } else {
        children
            .into_iter()
            .map(group)
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
    Ok(DistinctSpec {
        groups: groups.into(),
        min_against_surfaces: optional_number(node, "min-against-surfaces")?.unwrap_or(0.0),
        min_between_states: optional_number(node, "min-between-states")?.unwrap_or(0.0),
        min_delta_e_against_surfaces: optional_number(node, "min-delta-e-against-surfaces")?,
        min_delta_e_between_states: optional_number(node, "min-delta-e-between-states")?,
        max_chroma_of_states: optional_number(node, "max-chroma-of-states")?,
    })
}

pub(crate) fn contrast(node: Node<'_>) -> anyhow::Result<ContrastSpec> {
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
                    text: child.only_arg()?.into(),
                    on: child.required_prop("on").map(|on| {
                        on.split_whitespace()
                            .map(|word| Text::from(word.to_owned()))
                            .collect()
                    })?,
                });
            }
            "exclude" => {
                reason(child)?;
                exclusions.push(Exclusion {
                    text: texts(child.words("text")?),
                    on: texts(child.words("on")?),
                    themes: child
                        .children()
                        .iter()
                        .find(|inner| inner.name() == "themes")
                        .map(Node::args)
                        .transpose()?
                        .map(texts)
                        .unwrap_or_default(),
                });
            }
            "non-text" => {
                reason(child)?;
                non_text.extend(child.args()?);
            }
            "lines" | "fills" => {
                reason(child)?;
                plain_lines.push(PlainLines {
                    drawn: if child.name() == "fills" {
                        Drawn::Fill
                    } else {
                        Drawn::Line
                    },
                    min: child.number_prop("min")?,
                    colors: texts(child.words("colors")?),
                    on: texts(child.words("on")?),
                    faded_by: child.prop("faded-by")?.map(Text::from),
                });
            }
            "hovered" => hovered.push(read_hovered(child)?),
            "stronger" => stronger.push(read_stronger(child)?),
            "dimmed" => dimmed.push(read_dimmed(child)?),
            "more" => more = Some(read_more(child)?),
            other => bail!("line {}: unknown contrast rule `{other}`", child.line()),
        }
    }
    Ok(ContrastSpec {
        min: node.number_prop("min")?,
        plain_lines: plain_lines.into(),
        hovered: hovered.into(),
        stronger: stronger.into(),
        dimmed: dimmed.into(),
        more: more.ok_or_else(|| node.problem("needs a `more` rule"))?,
        text: texts(node.words("text")?),
        backgrounds: texts(node.words("backgrounds")?),
        restricted: restricted.into(),
        exclusions: exclusions.into(),
        non_text: texts(non_text),
    })
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

/// The words of a line as a list of names of the rules.
fn texts(words: Vec<String>) -> List<Text> {
    words.into_iter().map(Text::from).collect()
}
