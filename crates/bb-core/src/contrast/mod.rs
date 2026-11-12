//! Readability of a theme, worked out once (ADR-0026, update M8; THM-03). The maths of the contrast
//! ratio and of the difference in color, the contrast rules of the base theme file as data, and the
//! rating of each pair as *Easy*, *OK* or *Hard to read*. The check of the built-in theme files
//! (STY-CONTRAST), the server when an own theme is saved and the theme editor (through `bb-wasm`)
//! all call this and nothing else. Pure: no I/O.

use serde::{Deserialize, Serialize};

mod base_rules;
mod maths;
mod rating;
mod spec;

pub use base_rules::base_rules;
pub use maths::{
    brighten, channels, chroma_of, color_difference, contrast_ratio, fade, luminance, luminance_of,
    ratio_of,
};
pub use rating::{Evaluation, Mode, PairResult, Problem, Rating, Rule, ThemeColors, rate};
pub use spec::{
    ContrastSpec, Dimmed, DistinctGroup, DistinctSpec, Drawn, Exclusion, Hovered, List, MoreSpec,
    PlainLines, Restricted, StrongerText, Text,
};

/// The rules of a theme file: the `contrast` rule and every `distinct` rule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rules {
    /// The `contrast` rule.
    pub contrast: ContrastSpec,
    /// The `distinct` rules.
    pub distinct: List<DistinctSpec>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rules_of_the_base_file_are_written_into_the_crate() {
        let rules = base_rules();
        assert!((rules.contrast.min - 4.5).abs() < f64::EPSILON);
        assert!(rules.contrast.text.iter().any(|name| name == "fg"));
        assert!(!rules.distinct.is_empty());
    }
}
