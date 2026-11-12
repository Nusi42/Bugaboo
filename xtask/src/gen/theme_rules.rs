//! The contrast rules of the base theme file, written as data into `bb-core` (ADR-0026, update M8),
//! so the server and the theme editor read the rules that the check of the built-in files reads.
//!
//! The rules are one `static` of Rust values (borrowed names and lists), not a text read when the
//! program runs: nothing is parsed on a rating, and the browser module carries no reader of them
//! (M8-F44). A `static` is no function, so its length meets no limit of the lints.

use std::fmt::Write as _;

use super::theme_file::ThemeFile;
use bb_core::contrast::{
    ContrastSpec, Dimmed, DistinctGroup, DistinctSpec, Drawn, Exclusion, Hovered, MoreSpec,
    PlainLines, Restricted, StrongerText, Text,
};

/// Where the rules are written.
pub(super) const OUTPUT: &str = "crates/bb-core/src/contrast/base_rules.rs";

/// A value of the rules as a Rust expression of a `static`.
trait Literal {
    fn literal(&self, out: &mut String);
}

impl Literal for Text {
    fn literal(&self, out: &mut String) {
        let _ = write!(out, "Cow::Borrowed({:?})", self.as_ref());
    }
}

impl Literal for f64 {
    fn literal(&self, out: &mut String) {
        let _ = write!(out, "{self:?}");
    }
}

impl<T: Literal> Literal for Option<T> {
    fn literal(&self, out: &mut String) {
        match self {
            Some(value) => {
                out.push_str("Some(");
                value.literal(out);
                out.push(')');
            }
            None => out.push_str("None"),
        }
    }
}

impl<T: Literal + Clone> Literal for std::borrow::Cow<'static, [T]> {
    fn literal(&self, out: &mut String) {
        out.push_str("Cow::Borrowed(&[");
        for each in self.iter() {
            each.literal(out);
            out.push_str(", ");
        }
        out.push_str("])");
    }
}

impl<T: Literal> Literal for Vec<T> {
    fn literal(&self, out: &mut String) {
        out.push_str("Cow::Borrowed(&[");
        for each in self {
            each.literal(out);
            out.push_str(", ");
        }
        out.push_str("])");
    }
}

impl Literal for Drawn {
    fn literal(&self, out: &mut String) {
        out.push_str(match self {
            Self::Line => "Drawn::Line",
            Self::Fill => "Drawn::Fill",
        });
    }
}

/// A struct of the rules: its name and its fields, each written as a literal.
macro_rules! literal_struct {
    ($($name:ident { $($field:ident),+ $(,)? })+) => {
        $(impl Literal for $name {
            fn literal(&self, out: &mut String) {
                out.push_str(concat!(stringify!($name), " { "));
                $(
                    out.push_str(concat!(stringify!($field), ": "));
                    self.$field.literal(out);
                    out.push_str(", ");
                )+
                out.push('}');
            }
        })+
    };
}

literal_struct! {
    Restricted { text, on }
    Exclusion { text, on, themes }
    DistinctGroup { states, surfaces }
    DistinctSpec {
        groups,
        min_against_surfaces,
        min_between_states,
        min_delta_e_against_surfaces,
        min_delta_e_between_states,
        max_chroma_of_states,
    }
    Hovered { text, on, brightness, min, min_more }
    Dimmed { scrim, opacity, page, text, sheet, min_sheet_light, min_sheet_dark, max_text }
    StrongerText { min, text, on }
    MoreSpec { min_text, min_line, text, lines, line_on }
    PlainLines { drawn, min, colors, on, faded_by }
    ContrastSpec {
        min,
        plain_lines,
        hovered,
        stronger,
        dimmed,
        more,
        text,
        backgrounds,
        restricted,
        exclusions,
        non_text,
    }
}

/// The source of `base_rules.rs`, before `rustfmt`.
pub(super) fn rust(file: &ThemeFile, generated_by: &str) -> String {
    let mut contrast = String::new();
    file.contrast.literal(&mut contrast);
    let mut distinct = String::new();
    file.distinct.literal(&mut distinct);
    format!(
        "//! {generated_by}\n//!\n//! The contrast rules of the base theme file (`contrast` and `distinct`).\n\nuse std::borrow::Cow;\n\nuse super::{{\n    ContrastSpec, Dimmed, DistinctGroup, DistinctSpec, Drawn, Exclusion, Hovered, MoreSpec,\n    PlainLines, Restricted, Rules, StrongerText,\n}};\n\n/// The rules of the base theme file.\nstatic RULES: Rules = Rules {{\n    contrast: {contrast},\n    distinct: {distinct},\n}};\n\n/// The rules of the base theme file.\n#[must_use]\npub fn base_rules() -> &'static Rules {{\n    &RULES\n}}\n"
    )
}
