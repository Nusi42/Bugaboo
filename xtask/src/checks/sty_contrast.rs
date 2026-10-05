//! STY-CONTRAST: every text and background pair of the four built-in themes passes WCAG AA.
//!
//! Every text color is checked on every background color. The few combinations that are never
//! used are written down, with the reason, in `design/tokens.json` (`$extensions.bugaboo.contrast`)
//! (THM-03, UX-04).

use super::{Check, Context, Finding};
use crate::generate::token_file::{TOKENS_PATH, TokenFile};

pub(crate) struct StyContrast;

impl Check for StyContrast {
    fn id(&self) -> &'static str {
        "STY-CONTRAST"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(&TokenFile::load(context.repo)?))
    }
}

fn evaluate(tokens: &TokenFile) -> Vec<Finding> {
    let mut findings = Vec::new();
    let spec = &tokens.contrast;
    for theme in &tokens.themes {
        for (text_name, background_name) in spec.pairs(&theme.name) {
            let (Some(text), Some(background)) =
                (theme.color(text_name), theme.color(background_name))
            else {
                continue;
            };
            let ratio = contrast_ratio(text, background);
            if ratio < spec.min {
                findings.push(Finding::at(
                    TOKENS_PATH,
                    format!(
                        "Theme {}: `{text_name}` on `{background_name}` has contrast {ratio:.2}:1; it needs {:.1}:1. Change one of the two colors, or record why this pair is never used.",
                        theme.name, spec.min
                    ),
                ));
            }
        }
    }
    findings.extend(more_contrast_findings(tokens));
    findings.extend(distinct_findings(tokens));
    findings
}

/// For people whose device asks for more contrast (THM-06): the listed text reaches the higher
/// ratio on its backgrounds and the listed lines are visible on the plain surfaces, with the
/// colors the theme's `contrastMore` puts in place.
fn more_contrast_findings(tokens: &TokenFile) -> Vec<Finding> {
    let more = &tokens.contrast.more;
    let mut findings = Vec::new();
    for theme in &tokens.themes {
        let mut check = |first: &str, second: &str, min: f64, what: &str| {
            let (Some(a), Some(b)) = (
                theme.color_with_more_contrast(first),
                theme.color_with_more_contrast(second),
            ) else {
                return;
            };
            let ratio = contrast_ratio(a, b);
            if ratio < min {
                findings.push(Finding::at(
                    TOKENS_PATH,
                    format!(
                        "Theme {} with more contrast: {what} `{first}` on `{second}` has contrast {ratio:.2}:1; it needs {min:.1}:1. Change it in `contrastMore` of the theme.",
                        theme.name
                    ),
                ));
            }
        };
        for (text, background) in tokens.contrast.pairs(&theme.name) {
            if more.text.iter().any(|name| name == text) {
                check(text, background, more.min_text, "text");
            }
        }
        for line in &more.lines {
            for surface in &more.line_on {
                check(line, surface, more.min_line, "line");
            }
        }
    }
    findings
}

/// Hovered and selected backgrounds must stay visible against the surfaces and each other.
fn distinct_findings(tokens: &TokenFile) -> Vec<Finding> {
    let spec = &tokens.distinct;
    let mut findings = Vec::new();
    for theme in &tokens.themes {
        let mut check = |first: &str, second: &str, min: f64| {
            let (Some(a), Some(b)) = (theme.color(first), theme.color(second)) else {
                return;
            };
            let ratio = contrast_ratio(a, b);
            if ratio < min {
                findings.push(Finding::at(
                    TOKENS_PATH,
                    format!(
                        "Theme {}: `{first}` and `{second}` differ by only {ratio:.3}:1; they need {min:.2}:1 so the state stays visible.",
                        theme.name
                    ),
                ));
            }
        };
        for (index, state) in spec.states.iter().enumerate() {
            for surface in &spec.surfaces {
                check(state, surface, spec.min_against_surfaces);
            }
            for other in &spec.states[index + 1..] {
                check(state, other, spec.min_between_states);
            }
        }
    }
    findings
}

/// WCAG 2.2 contrast ratio between two `#RRGGBB` colors.
fn contrast_ratio(first: &str, second: &str) -> f64 {
    let (a, b) = (luminance(first), luminance(second));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn luminance(hex: &str) -> f64 {
    let channel = |range: std::ops::Range<usize>| {
        let digits = hex.get(range).unwrap_or("00");
        let value = f64::from(u8::from_str_radix(digits, 16).unwrap_or(0)) / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(1..3) + 0.7152 * channel(3..5) + 0.0722 * channel(5..7)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::Repo;

    #[test]
    fn known_ratios() {
        assert!((contrast_ratio("#000000", "#FFFFFF") - 21.0).abs() < 0.01);
        assert!((contrast_ratio("#777777", "#FFFFFF") - 4.48).abs() < 0.01);
    }

    #[test]
    fn good_example() {
        let tokens = TokenFile::load(&Repo::discover().unwrap()).unwrap();
        let findings = evaluate(&tokens);
        let messages: Vec<&str> = findings.iter().map(|f| f.message.as_str()).collect();
        assert!(findings.is_empty(), "{messages:#?}");
    }

    #[test]
    fn every_text_color_is_checked_on_every_neutral_surface() {
        let tokens = TokenFile::load(&Repo::discover().unwrap()).unwrap();
        let pairs = tokens.contrast.pairs("daylight");
        for text in ["fg", "fg-muted", "fg-subtle", "warning", "accent-fg"] {
            for surface in ["bg-canvas", "bg-hover", "bg-selected", "bg-sunken"] {
                assert!(pairs.contains(&(text, surface)), "{text} on {surface}");
            }
        }
    }

    #[test]
    fn a_color_without_a_classification_is_rejected() {
        let repo = Repo::discover().unwrap();
        let text = std::fs::read_to_string(repo.path(TOKENS_PATH)).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let non_text = &mut value["$extensions"]["bugaboo"]["contrast"]["nonText"];
        *non_text = serde_json::json!([{ "colors": ["border"], "reason": "decorative" }]);
        let error = TokenFile::parse(&value.to_string()).err().unwrap();
        assert!(format!("{error:#}").contains("border-subtle"), "{error:#}");
    }

    #[test]
    fn a_hover_color_that_vanishes_into_a_surface_is_rejected() {
        let repo = Repo::discover().unwrap();
        let text = std::fs::read_to_string(repo.path(TOKENS_PATH)).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        // Dusk hover one step away from the raised surface, as in the rejected retune.
        value["themes"]["dusk"]["color"]["bg-hover"]["$value"] = "#2D323D".into();
        let findings = evaluate(&TokenFile::parse(&value.to_string()).unwrap());
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("bg-hover") && f.message.contains("bg-raised")),
            "{:?}",
            findings
                .iter()
                .map(|f| f.message.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn hover_and_selected_that_look_the_same_are_rejected() {
        let repo = Repo::discover().unwrap();
        let text = std::fs::read_to_string(repo.path(TOKENS_PATH)).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let same = value["themes"]["daylight"]["color"]["bg-hover"]["$value"].clone();
        value["themes"]["daylight"]["color"]["bg-selected"]["$value"] = same;
        let findings = evaluate(&TokenFile::parse(&value.to_string()).unwrap());
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("bg-selected") && f.message.contains("bg-hover"))
        );
    }

    #[test]
    fn weak_text_with_more_contrast_is_rejected() {
        let repo = Repo::discover().unwrap();
        let text = std::fs::read_to_string(repo.path(TOKENS_PATH)).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        // The stronger muted text of Daylight put back to a color that only reaches AA.
        value["themes"]["daylight"]["contrastMore"]["color"]["fg-muted"]["$value"] =
            "#5A6170".into();
        let findings = evaluate(&TokenFile::parse(&value.to_string()).unwrap());
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("with more contrast")
                    && f.message.contains("`fg-muted`")),
            "{:?}",
            findings
                .iter()
                .map(|f| f.message.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_weak_accent_fill_with_more_contrast_is_rejected() {
        let repo = Repo::discover().unwrap();
        let text = std::fs::read_to_string(repo.path(TOKENS_PATH)).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        // The accent fill of Daylight put back to the usual one: white text on it reaches only AA.
        value["themes"]["daylight"]["contrastMore"]["color"]["accent"]["$value"] = "#5B3FD9".into();
        let findings = evaluate(&TokenFile::parse(&value.to_string()).unwrap());
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("`on-accent`") && f.message.contains("`accent`")),
            "{:?}",
            findings
                .iter()
                .map(|f| f.message.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn faint_lines_with_more_contrast_are_rejected() {
        let repo = Repo::discover().unwrap();
        let text = std::fs::read_to_string(repo.path(TOKENS_PATH)).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        value["themes"]["dusk"]["contrastMore"]["color"]["border"]["$value"] = "#3B414D".into();
        let findings = evaluate(&TokenFile::parse(&value.to_string()).unwrap());
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("line `border`") && f.message.contains("Theme dusk")),
            "{:?}",
            findings
                .iter()
                .map(|f| f.message.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn bad_example() {
        let repo = Repo::discover().unwrap();
        let text = std::fs::read_to_string(repo.path(TOKENS_PATH)).unwrap();
        // Light grey text on the white page in Daylight.
        let bad = text.replacen("\"#16181D\"", "\"#CCCCCC\"", 1);
        assert_ne!(bad, text);
        let findings = evaluate(&TokenFile::parse(&bad).unwrap());
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("Theme daylight"))
        );
    }
}
