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
    findings.extend(line_findings(file));
    findings.extend(more_contrast_findings(file));
    findings.extend(distinct_findings(file));
    findings
}

/// The lines that people have to see (the edge of a box to tick, the edge of a button that cannot be
/// pressed) reach their ratio on the plain surfaces in every theme, as they are without the device
/// setting for more contrast. A line that is faded (`faded-by`) is checked as it is seen: its color
/// mixed with the surface behind it by the opacity.
fn line_findings(file: &ThemeFile) -> Vec<Finding> {
    let mut findings = Vec::new();
    for lines in &file.contrast.plain_lines {
        // The theme file is valid, so a name here is an opacity (`ThemeFile::opacity`); a line with no
        // `faded-by` is seen as it is.
        let opacity = match lines.faded_by.as_deref() {
            None => 1.0,
            Some(name) => match file.opacity(name) {
                Some(opacity) => opacity,
                None => continue,
            },
        };
        for theme in &file.themes {
            for line in &lines.colors {
                for surface in &lines.on {
                    let (Some(a), Some(b)) = (
                        color(file, line, &theme.name),
                        color(file, surface, &theme.name),
                    ) else {
                        continue;
                    };
                    let ratio = ratio_of(luminance_of(fade(a, b, opacity)), luminance(b));
                    if ratio < lines.min {
                        findings.push(Finding::at(
                            THEME_PATH,
                            format!(
                                "Theme {}: the line `{line}` on `{surface}` has contrast {ratio:.2}:1; it needs {:.1}:1. Change its value.",
                                theme.name, lines.min
                            ),
                        ));
                    }
                }
            }
        }
    }
    findings
}

/// A `#RRGGBB` color as it is seen when it is drawn with `opacity` over `behind`: its three channels,
/// from 0 to 255.
fn fade(color: &str, behind: &str, opacity: f64) -> [f64; 3] {
    let (over, under) = (channels(color), channels(behind));
    std::array::from_fn(|at| over[at].mul_add(opacity, under[at] * (1.0 - opacity)))
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
    ratio_of(luminance(first), luminance(second))
}

/// The contrast ratio of two luminances.
fn ratio_of(a: f64, b: f64) -> f64 {
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// The three channels of a `#RRGGBB` color, from 0 to 255.
fn channels(hex: &str) -> [f64; 3] {
    std::array::from_fn(|at| {
        let digits = hex.get(1 + at * 2..3 + at * 2).unwrap_or("00");
        f64::from(u8::from_str_radix(digits, 16).unwrap_or(0))
    })
}

fn luminance(hex: &str) -> f64 {
    luminance_of(channels(hex))
}

/// How bright a color of three channels (0 to 255) is, by the formula of WCAG.
fn luminance_of(color: [f64; 3]) -> f64 {
    let [red, green, blue] = color.map(|channel| {
        let value = channel / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * red + 0.7152 * green + 0.0722 * blue
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
    fn the_edge_of_a_disabled_button_is_checked_as_it_is_seen() {
        // The edge is the color of quiet words faded by the opacity of a disabled control: this
        // color reaches 2:1 on the raised surface of Dusk as it is (2.56:1) and no longer once it is
        // faded (1.76:1), so only the fade makes the check find it.
        assert!(contrast_ratio("#686f80", "#2d323c") >= 2.0);
        let findings = evaluate(&with_value("fg-muted", "dusk", "#686f80"));
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("the line `fg-muted`")),
            "{:?}",
            messages(&findings)
        );
    }

    #[test]
    fn a_color_fades_toward_the_surface_behind_it() {
        let near = |seen: [f64; 3], wanted: [f64; 3]| {
            seen.iter().zip(wanted).all(|(a, b)| (a - b).abs() < 1e-9)
        };
        assert!(near(fade("#ffffff", "#000000", 0.5), [127.5; 3]));
        assert!(near(fade("#123456", "#000000", 1.0), [18.0, 52.0, 86.0]));
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
    fn a_faint_edge_of_a_control_is_rejected_in_the_plain_mode() {
        // The edge of a box to tick put back to the faint divider of Midnight.
        let findings = evaluate(&with_value("border-control", "midnight", "#272a31"));
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("`border-control`") && f.message.contains("midnight")),
            "{:?}",
            messages(&findings)
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
