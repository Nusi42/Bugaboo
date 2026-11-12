//! The style sheets of an own theme (M8-Q8): custom properties and nothing else, written from the
//! stored values alone. No text of a person reaches them: colors are six or eight hexadecimal
//! digits, numbers are numbers, and a text stack is the stack of the base theme file that the value
//! names.
//!
//! A saved theme has one sheet for the page and for its samples; the draft of the theme editor has
//! one for the preview of the editor only, so unsaved values never reach the page around it.

use std::fmt::Write as _;

use super::values::OwnValues;
use super::{built_in, font_tokens, radius_tokens};

/// Whether `id` can stand in a selector as it is: letters, digits and hyphens.
fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

/// Which elements a sheet sets its variables on.
#[derive(Clone, Copy)]
enum Reach {
    /// The page whose frame carries the theme, and every sample of a theme that names it.
    Saved,
    /// The preview of the theme editor that names the draft, and nothing else.
    Draft,
}

/// The style sheet of the saved theme `id` with these values: the variables under the attribute of
/// the page that carries the theme and under a sample of a theme that names it (a sample in the
/// theme pickers), and the colors for more contrast in their own block. `None` when the ID is not
/// of a shape a selector can take or a value is not one an own theme may set, so no sheet is ever
/// made from values that were not checked.
#[must_use]
pub fn style_sheet(id: &str, values: &OwnValues) -> Option<String> {
    sheet(id, values, Reach::Saved)
}

/// The style sheet of the theme editor's draft of the theme `id`: the variables only under the
/// preview that names the draft, never on the page. The corners and the text faces are written
/// whether the draft changes them or not, so the preview does not show those of a saved theme the
/// page may wear. `None` as for [`style_sheet`].
#[must_use]
pub fn draft_style_sheet(id: &str, values: &OwnValues) -> Option<String> {
    sheet(id, values, Reach::Draft)
}

fn sheet(id: &str, values: &OwnValues, reach: Reach) -> Option<String> {
    if !is_safe_id(id) || !values.problems().is_empty() {
        return None;
    }
    let preview = built_in::PREVIEW_ATTRIBUTE;
    let mut sheet = String::from(
        "/* An own theme: variables only, written by the server from stored values (ADR-0026). */\n",
    );
    match reach {
        Reach::Saved => {
            let own = built_in::OWN_ATTRIBUTE;
            block(&mut sheet, &format!(":root[{own}=\"{id}\"]"), values, false);
            sheet.push('\n');
            block(
                &mut sheet,
                &format!("[{preview}][{own}=\"{id}\"]"),
                values,
                true,
            );
        }
        Reach::Draft => {
            let draft = built_in::OWN_DRAFT_ATTRIBUTE;
            block(
                &mut sheet,
                &format!("[{preview}][{draft}=\"{id}\"]"),
                values,
                true,
            );
        }
    }
    Some(sheet)
}

/// Writes the variables of `values` under `selector`, the colors for more contrast in a block of
/// their own. With `every_shape` the corners and the faces are written whether the theme changes
/// them or not: a sample or a preview then shows its own and none of the page around it.
fn block(sheet: &mut String, selector: &str, values: &OwnValues, every_shape: bool) {
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
    for radius in radius_tokens() {
        let pixels = match values.radii.get(radius.name) {
            Some(pixels) => *pixels,
            None if every_shape => radius.start_px,
            None => continue,
        };
        let _ = writeln!(usual, "  --{}: {pixels}px;", radius.name);
    }
    for font in font_tokens() {
        let chosen = match values.fonts.get(font.name) {
            Some(stack) => stack.as_str(),
            None if every_shape => font.name,
            None => continue,
        };
        if let Some(stack) = font_tokens().iter().find(|each| each.name == chosen) {
            let _ = writeln!(usual, "  --{}: {};", font.name, stack.stack);
        }
    }
    let _ = write!(sheet, "{selector} {{\n{usual}}}\n");
    if !more.is_empty() {
        let _ = write!(
            sheet,
            "\n@media (prefers-contrast: more) {{\n  {selector} {{\n{more}  }}\n}}\n"
        );
    }
}
