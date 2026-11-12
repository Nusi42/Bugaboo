//! What an own theme may set, and the built-in themes it starts from, written as Rust data into
//! `bb-core` (ADR-0026, update M8), so the server, the upload and the theme editor read one list.

use std::fmt::Write as _;

use super::theme_file::{LightOrDark, Own, ThemeFile, TokenKind};

/// Where the data is written.
pub(super) const OUTPUT: &str = "crates/bb-core/src/theme/built_in.rs";

fn more(value: Option<&str>) -> String {
    value.map_or_else(|| "None".to_owned(), |value| format!("Some({value:?})"))
}

/// The source of `built_in.rs`, before `rustfmt`.
pub(super) fn rust(file: &ThemeFile, generated_by: &str) -> String {
    let mut out = format!(
        "//! {generated_by}\n//!\n//! The built-in themes with their colors and shadow, the tokens an own theme may set and the\n//! numbers the contrast rules read (ADR-0026, update M8).\n\nuse super::{{BuiltIn, ColorValue, FontToken, Number, RadiusToken}};\n\n/// The attribute of the page that carries the ID of an own theme (the `own` of the `frame` line).\npub(super) const OWN_ATTRIBUTE: &str = {:?};\n\n/// The attribute of a sample of a theme inside the page (the `preview` of the `frame` line).\npub(super) const PREVIEW_ATTRIBUTE: &str = {:?};\n\n/// The attribute of the preview of the theme editor that names the draft (the `draft` of the\n/// `frame` line).\npub(super) const OWN_DRAFT_ATTRIBUTE: &str = {:?};\n\n",
        file.frame.own, file.frame.preview, file.frame.draft
    );
    out.push_str("/// The built-in themes, light ones first, in the order of the theme files.\npub(super) const BUILT_IN: &[BuiltIn] = &[\n");
    for theme in &file.themes {
        let _ = writeln!(
            out,
            "    BuiltIn {{\n        id: {:?},\n        light: {},\n        colors: &[",
            theme.name,
            theme.kind == LightOrDark::Light
        );
        for token in file.colors() {
            if let Some(value) = token.in_theme(&theme.name) {
                let _ = writeln!(
                    out,
                    "            ColorValue::new({:?}, {value:?}, {}),",
                    token.name,
                    more(token.more_in(&theme.name))
                );
            }
        }
        let shadow = file
            .tokens
            .iter()
            .find(|token| matches!(token.own, Own::Shadow { .. }))
            .and_then(|token| token.in_theme(&theme.name))
            .unwrap_or_default();
        let _ = writeln!(out, "        ],\n        shadow: {shadow:?},\n    }},");
    }
    out.push_str("];\n");
    let shadow = file.tokens.iter().find_map(|token| match token.own {
        Own::Shadow { layers, pixels } => Some((token.name.as_str(), layers, pixels)),
        _ => None,
    });
    let (name, layers, pixels) = shadow.unwrap_or_default();
    let _ = write!(
        out,
        "\n/// The shadow token an own theme may set, as parts.\npub(super) const SHADOW_TOKEN: &str = {name:?};\n\n/// The most layers a shadow of an own theme has (`own-layers`).\npub const SHADOW_LAYERS_MOST: usize = {layers};\n\n/// The largest offset, blur or spread of a layer of a shadow of an own theme, in pixels\n/// (`own-pixels`).\npub const SHADOW_PIXELS_MOST: i32 = {pixels};\n"
    );
    out.push_str("\n/// The corner radii an own theme may set, in pixels.\npub(super) const RADII: &[RadiusToken] = &[\n");
    for token in &file.tokens {
        if let Own::Pixels { min, max } = token.own {
            let start = token
                .same
                .as_deref()
                .and_then(|same| same.strip_suffix("px"))
                .unwrap_or("0");
            let _ = writeln!(
                out,
                "    RadiusToken {{ name: {:?}, start_px: {start}, min_px: {min}, max_px: {max} }},",
                token.name
            );
        }
    }
    out.push_str("];\n\n/// The text faces an own theme may choose a stack for. It names one of these stacks by its token.\npub(super) const FONTS: &[FontToken] = &[\n");
    for token in &file.tokens {
        if token.kind == TokenKind::Font && token.own == Own::Yes {
            let _ = writeln!(
                out,
                "    FontToken {{ name: {:?}, stack: {:?} }},",
                token.name,
                token.same.as_deref().unwrap_or_default()
            );
        }
    }
    out.push_str("];\n\n/// The opacity tokens, 0 to 1, for the contrast rules that fade a color.\npub(super) const OPACITIES: &[Number] = &[\n");
    numbers(&mut out, file, TokenKind::Opacity);
    out.push_str("];\n\n/// The number tokens, for the contrast rules that brighten a color.\npub(super) const NUMBERS: &[Number] = &[\n");
    numbers(&mut out, file, TokenKind::Number);
    out.push_str("];\n");
    out
}

fn numbers(out: &mut String, file: &ThemeFile, kind: TokenKind) {
    for token in file.tokens.iter().filter(|token| token.kind == kind) {
        if let Some(value) = token.same.as_deref().and_then(|v| v.parse::<f64>().ok()) {
            let _ = writeln!(
                out,
                "    Number {{ name: {:?}, value: {value:?} }},",
                token.name
            );
        }
    }
}

/// What an own theme may set besides its colors, for the theme editor: the corners with their
/// bounds, the text faces and the limits of the shadow, from the same lines as the Rust data.
pub(super) fn typescript(file: &ThemeFile) -> String {
    let mut out = String::from(
        "\n/** The corners an own theme may set, in pixels: the base theme's value and the bounds (ADR-0026 M8). */\nexport const ownRadii = [\n",
    );
    for token in &file.tokens {
        if let Own::Pixels { min, max } = token.own {
            let start = token
                .same
                .as_deref()
                .and_then(|same| same.strip_suffix("px"))
                .unwrap_or("0");
            let _ = writeln!(
                out,
                "  {{ name: {:?}, start: {start}, min: {min}, max: {max} }},",
                token.name
            );
        }
    }
    out.push_str("] as const;\nexport type OwnRadius = (typeof ownRadii)[number][\"name\"];\n\n/** The text faces an own theme may choose a stack for; a stack is named by one of these. */\nexport const ownFonts = [\n");
    for token in &file.tokens {
        if token.kind == TokenKind::Font && token.own == Own::Yes {
            let _ = writeln!(out, "  {:?},", token.name);
        }
    }
    out.push_str("] as const;\nexport type OwnFont = (typeof ownFonts)[number];\n");
    if let Some((name, layers, pixels)) = file.tokens.iter().find_map(|token| match token.own {
        Own::Shadow { layers, pixels } => Some((token.name.as_str(), layers, pixels)),
        _ => None,
    }) {
        let _ = write!(
            out,
            "\n/** The shadow an own theme may set: at most `layers` layers, each offset, blur and spread within `pixels`. */\nexport const ownShadow = {{ name: {name:?}, layers: {layers}, pixels: {pixels} }} as const;\n"
        );
    }
    out
}
