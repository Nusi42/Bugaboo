//! STY-CONTRAST: every text and background pair of the four built-in themes passes WCAG AA.
//!
//! The pairs are listed in `design/tokens.json` (`$extensions.bugaboo.contrast`), next to the
//! colors they judge (THM-03, UX-04).

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
    for theme in &tokens.themes {
        for rule in &tokens.contrast {
            let Some(text) = theme.color(&rule.text) else {
                continue;
            };
            for background_name in &rule.on {
                let Some(background) = theme.color(background_name) else {
                    continue;
                };
                let ratio = contrast_ratio(text, background);
                if ratio < rule.min {
                    findings.push(Finding::at(
                        TOKENS_PATH,
                        format!(
                            "Theme {}: `{}` on `{background_name}` has contrast {ratio:.2}:1; it needs {:.1}:1. Change one of the two colors.",
                            theme.name, rule.text, rule.min
                        ),
                    ));
                }
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
