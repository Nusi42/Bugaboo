//! The style sheet of an own theme (M8-Q8): custom properties and nothing else, written from the
//! stored values alone. No text of a person reaches it: colors are six or eight hexadecimal digits,
//! numbers are numbers, and a text stack is the stack of the base theme file that the value names.

use std::fmt::Write as _;

use super::values::OwnValues;
use super::{built_in, font_tokens};

/// Whether `id` can stand in a selector as it is: letters, digits and hyphens.
fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

/// The style sheet of the theme `id` with these values: the variables under the attribute of the
/// page that carries the theme, and the colors for more contrast in their own block. `None` when
/// the ID is not of a shape a selector can take or a value is not one an own theme may set, so no
/// sheet is ever made from values that were not checked.
#[must_use]
pub fn style_sheet(id: &str, values: &OwnValues) -> Option<String> {
    if !is_safe_id(id) || !values.problems().is_empty() {
        return None;
    }
    let selector = format!(":root[{}=\"{id}\"]", built_in::OWN_ATTRIBUTE);
    let mut usual = String::new();
    let mut more = String::new();
    for (name, choice) in &values.colors {
        let _ = writeln!(usual, "  --{name}: {};", choice.value.to_ascii_lowercase());
        if let Some(value) = &choice.more {
            let _ = writeln!(more, "    --{name}: {};", value.to_ascii_lowercase());
        }
    }
    if let Some(layers) = &values.shadow {
        let parts: Vec<String> = layers
            .iter()
            .map(|layer| {
                format!(
                    "{}px {}px {}px {}px {}",
                    layer.x,
                    layer.y,
                    layer.blur,
                    layer.spread,
                    layer.color.to_ascii_lowercase()
                )
            })
            .collect();
        let _ = writeln!(
            usual,
            "  --{}: {};",
            built_in::SHADOW_TOKEN,
            parts.join(", ")
        );
    }
    for (name, pixels) in &values.radii {
        let _ = writeln!(usual, "  --{name}: {pixels}px;");
    }
    for (name, stack) in &values.fonts {
        if let Some(font) = font_tokens().iter().find(|font| font.name == stack) {
            let _ = writeln!(usual, "  --{name}: {};", font.stack);
        }
    }
    let mut sheet = String::from(
        "/* An own theme: variables only, written by the server from stored values (ADR-0026). */\n",
    );
    let _ = write!(sheet, "{selector} {{\n{usual}}}\n");
    if !more.is_empty() {
        let _ = write!(
            sheet,
            "\n@media (prefers-contrast: more) {{\n  {selector} {{\n{more}  }}\n}}\n"
        );
    }
    Some(sheet)
}
