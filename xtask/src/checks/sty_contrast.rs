//! STY-CONTRAST: every text and background pair of the four built-in themes passes WCAG AA.
//!
//! Every text color is checked on every background color. The few combinations that are never
//! used are written down, with the reason, in `design/theme.kdl` (the `contrast` line) (THM-03,
//! UX-04). The check runs for every theme and for every mode that changes colors: the usual one,
//! and the one for a device that asks for more contrast (THM-06).

use super::{Check, Context, Finding};
use crate::generate::contrast_spec::DistinctSpec;
use crate::generate::theme_file::{THEME_PATH, ThemeFile};

pub(crate) struct StyContrast;

impl Check for StyContrast {
    fn id(&self) -> &'static str {
        "STY-CONTRAST"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(&ThemeFile::load(context.repo)?))
    }
}

/// The value of a color token in a theme.
fn color<'a>(file: &'a ThemeFile, name: &str, theme: &str) -> Option<&'a str> {
    file.token(name)?.in_theme(theme)
}

/// The same, with the colors that more contrast replaces.
fn color_with_more_contrast<'a>(file: &'a ThemeFile, name: &str, theme: &str) -> Option<&'a str> {
    file.token(name)?.in_theme_with_more_contrast(theme)
}

fn evaluate(file: &ThemeFile) -> Vec<Finding> {
    let mut findings = Vec::new();
    let spec = &file.contrast;
    for theme in &file.themes {
        for (text_name, background_name) in spec.pairs(&theme.name) {
            let (Some(text), Some(background)) = (
                color(file, text_name, &theme.name),
                color(file, background_name, &theme.name),
            ) else {
                continue;
            };
            let ratio = contrast_ratio(text, background);
            if ratio < spec.min {
                findings.push(Finding::at(
                    THEME_PATH,
                    format!(
                        "Theme {}: `{text_name}` on `{background_name}` has contrast {ratio:.2}:1; it needs {:.1}:1. Change one of the two colors, or record why this pair is never used.",
                        theme.name, spec.min
                    ),
                ));
            }
        }
    }
    findings.extend(more_contrast_findings(file));
    findings.extend(distinct_findings(file));
    findings
}

/// For people whose device asks for more contrast (THM-06): the listed text reaches the higher
/// ratio on its backgrounds and the listed lines are visible on the plain surfaces, with the
/// colors that the `<theme>-more=` values of the tokens put in place.
fn more_contrast_findings(file: &ThemeFile) -> Vec<Finding> {
    let more = &file.contrast.more;
    let mut findings = Vec::new();
    for theme in &file.themes {
        let mut check = |first: &str, second: &str, min: f64, what: &str| {
            let (Some(a), Some(b)) = (
                color_with_more_contrast(file, first, &theme.name),
                color_with_more_contrast(file, second, &theme.name),
            ) else {
                return;
            };
            let ratio = contrast_ratio(a, b);
            if ratio < min {
                findings.push(Finding::at(
                    THEME_PATH,
                    format!(
                        "Theme {} with more contrast: {what} `{first}` on `{second}` has contrast {ratio:.2}:1; it needs {min:.1}:1. Change its `{}-more=` value.",
                        theme.name, theme.name
                    ),
                ));
            }
        };
        for (text, background) in file.contrast.pairs(&theme.name) {
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

/// Hovered and selected backgrounds must stay visible against the surfaces and each other, on the
/// plain surfaces and on the tinted rows.
fn distinct_findings(file: &ThemeFile) -> Vec<Finding> {
    file.distinct
        .iter()
        .flat_map(|spec| distinct_findings_of(file, spec))
        .collect()
}

fn distinct_findings_of(file: &ThemeFile, spec: &DistinctSpec) -> Vec<Finding> {
    let mut findings = Vec::new();
    for theme in &file.themes {
        let mut check = |first: &str, second: &str, min: f64| {
            let (Some(a), Some(b)) = (
                color(file, first, &theme.name),
                color(file, second, &theme.name),
            ) else {
                return;
            };
            let ratio = contrast_ratio(a, b);
            if ratio < min {
                findings.push(Finding::at(
                    THEME_PATH,
                    format!(
                        "Theme {}: `{first}` and `{second}` differ by only {ratio:.3}:1; they need {min:.2}:1 so the state stays visible.",
                        theme.name
                    ),
                ));
            }
        };
        for group in &spec.groups {
            for (index, state) in group.states.iter().enumerate() {
                for surface in &group.surfaces {
                    check(state, surface, spec.min_against_surfaces);
                }
                for other in &group.states[index + 1..] {
                    check(state, other, spec.min_between_states);
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

    fn text() -> String {
        crate::source::read_text(&Repo::discover().unwrap(), THEME_PATH).unwrap()
    }

    /// The theme file with one value of a color token replaced: `key` is `dusk` or `dusk-more`.
    fn with_value(token: &str, key: &str, value: &str) -> ThemeFile {
        let text = text();
        let start = text
            .find(&format!("color \"{token}\""))
            .expect("the token is in the file");
        let line_end = text[start..].find('\n').map_or(text.len(), |at| start + at);
        let line = &text[start..line_end];
        let marker = format!(" {key}=\"");
        let value_at = line.find(&marker).expect("the token has this mode") + marker.len();
        let value_end = line[value_at..].find('"').expect("the value ends") + value_at;
        let changed = format!(
            "{}{}{}{}",
            &text[..start + value_at],
            value,
            &line[value_end..],
            &text[line_end..]
        );
        ThemeFile::parse(&changed).unwrap()
    }

    fn messages(findings: &[Finding]) -> Vec<String> {
        findings
            .iter()
            .map(|finding| finding.message.clone())
            .collect()
    }

    #[test]
    fn known_ratios() {
        assert!((contrast_ratio("#000000", "#FFFFFF") - 21.0).abs() < 0.01);
        assert!((contrast_ratio("#777777", "#FFFFFF") - 4.48).abs() < 0.01);
    }

    #[test]
    fn good_example() {
        let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
        let findings = evaluate(&file);
        assert!(findings.is_empty(), "{:#?}", messages(&findings));
    }

    #[test]
    fn every_text_color_is_checked_on_every_neutral_surface() {
        let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
        let pairs = file.contrast.pairs("daylight");
        for text in ["fg", "fg-muted", "fg-subtle", "warning", "accent-fg"] {
            for surface in ["bg-canvas", "bg-hover", "bg-selected", "bg-sunken"] {
                assert!(pairs.contains(&(text, surface)), "{text} on {surface}");
            }
        }
    }

    #[test]
    fn cf_04_every_look_is_readable_in_all_four_themes() {
        // Every choice color the server can store is a pair of tokens in the theme, and the
        // text of a badge, a pill and the muted text on a plain surface of that color all pass,
        // in the four themes and with more contrast.
        let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
        for theme in file.themes.iter().map(|theme| theme.name.as_str()) {
            let pairs = file.contrast.pairs(theme);
            for color in bb_core::own_fields::ChoiceColor::ALL {
                let (text, surface) = (
                    format!("{}-fg", color.code()),
                    format!("{}-bg", color.code()),
                );
                assert!(
                    pairs.contains(&(text.as_str(), surface.as_str())),
                    "{text} on {surface} is not checked in {theme}"
                );
                assert!(
                    pairs.contains(&("fg-muted", surface.as_str())),
                    "fg-muted on {surface} is not checked in {theme}"
                );
            }
        }
        let findings = evaluate(&file);
        assert!(findings.is_empty(), "{:#?}", messages(&findings));
    }

    #[test]
    fn cf_04_the_progress_bar_and_the_avatar_are_seen_in_all_four_themes() {
        // The fill of a progress bar is a shape, not text: it is seen against every plain surface
        // at 3:1 (WCAG 1.4.11). The initials of an avatar are text on the soft accent.
        let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
        for theme in file.themes.iter().map(|theme| theme.name.as_str()) {
            let value = |name: &str| color(&file, name, theme).expect("the token has a color");
            for surface in ["bg-canvas", "bg-surface", "bg-raised"] {
                let ratio = contrast_ratio(value("accent"), value(surface));
                assert!(ratio >= 3.0, "{theme}: accent on {surface} is {ratio:.2}:1");
            }
            let ratio = contrast_ratio(value("accent-fg"), value("accent-soft"));
            assert!(ratio >= 4.5, "{theme}: the initials are {ratio:.2}:1");
        }
    }

    #[test]
    fn a_choice_color_that_is_hard_to_read_is_rejected() {
        // Category 3 text on a background that is nearly the same color in Dusk.
        let findings = evaluate(&with_value("cat3-bg", "dusk", "#6a8f78"));
        assert!(
            findings.iter().any(|f| f.message.contains("`cat3-fg`")),
            "{:?}",
            messages(&findings)
        );
    }

    #[test]
    fn a_color_without_a_classification_is_rejected() {
        let text = text();
        let start = text.find("    non-text").expect("the rule is in the file");
        let end = text[start..]
            .find("reason=")
            .map(|at| start + at)
            .expect("it has a reason");
        let bad = format!("{}    non-text \"border\" {}", &text[..start], &text[end..]);
        let error = ThemeFile::parse(&bad).err().unwrap();
        assert!(format!("{error:#}").contains("border-subtle"), "{error:#}");
    }

    #[test]
    fn a_hover_color_that_vanishes_into_a_tint_is_rejected() {
        let findings = evaluate(&with_value("tint-ip-hover", "daylight", "#f4f8fe"));
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("tint-ip-hover") && f.message.contains("`tint-ip`")),
            "{:?}",
            messages(&findings)
        );
    }

    #[test]
    fn a_hover_color_that_vanishes_into_a_surface_is_rejected() {
        // Dusk hover one step away from the raised surface, as in the rejected retune.
        let findings = evaluate(&with_value("bg-hover", "dusk", "#2d323d"));
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("bg-hover") && f.message.contains("bg-raised")),
            "{:?}",
            messages(&findings)
        );
    }

    #[test]
    fn hover_and_selected_that_look_the_same_are_rejected() {
        let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
        let hover = file
            .token("bg-hover")
            .unwrap()
            .in_theme("daylight")
            .unwrap()
            .to_owned();
        let findings = evaluate(&with_value("bg-selected", "daylight", &hover));
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("bg-selected") && f.message.contains("bg-hover"))
        );
    }

    #[test]
    fn weak_text_with_more_contrast_is_rejected() {
        // The stronger muted text of Daylight put back to a color that only reaches AA.
        let findings = evaluate(&with_value("fg-muted", "daylight-more", "#5a6170"));
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("with more contrast")
                    && f.message.contains("`fg-muted`")),
            "{:?}",
            messages(&findings)
        );
    }

    #[test]
    fn a_weak_accent_fill_with_more_contrast_is_rejected() {
        // The accent fill of Daylight put back to the usual one: white text on it reaches only AA.
        let findings = evaluate(&with_value("accent", "daylight-more", "#5b3fd9"));
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("`on-accent`") && f.message.contains("`accent`")),
            "{:?}",
            messages(&findings)
        );
    }

    #[test]
    fn faint_lines_with_more_contrast_are_rejected() {
        let findings = evaluate(&with_value("border", "dusk-more", "#3b414d"));
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("line `border`") && f.message.contains("Theme dusk")),
            "{:?}",
            messages(&findings)
        );
    }

    #[test]
    fn bad_example() {
        // Light grey text on the white page in Daylight.
        let findings = evaluate(&with_value("fg", "daylight", "#cccccc"));
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("Theme daylight"))
        );
    }
}
