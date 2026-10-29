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
    findings.extend(hovered_findings(file));
    findings.extend(stronger_findings(file));
    findings.extend(dimmed_findings(file));
    findings.extend(distinct_findings(file));
    findings
}

/// A fill that is brightened while it is pointed at (`filter: brightness`) brightens its text too:
/// the pair is checked as it is seen then, as it is and with the device setting for more contrast.
fn hovered_findings(file: &ThemeFile) -> Vec<Finding> {
    let mut findings = Vec::new();
    for rule in &file.contrast.hovered {
        let Some(brightness) = file.number(&rule.brightness) else {
            findings.push(Finding::at(
                THEME_PATH,
                format!(
                    "The hovered pair `{}` on `{}` uses `{}`, which is no number token, so it is not checked. Name a number.",
                    rule.text, rule.on, rule.brightness
                ),
            ));
            continue;
        };
        for theme in &file.themes {
            for (more, mode, min) in [
                (false, "", rule.min),
                (true, " with more contrast", rule.min_more),
            ] {
                let pick = |name: &str| {
                    if more {
                        color_with_more_contrast(file, name, &theme.name)
                    } else {
                        color(file, name, &theme.name)
                    }
                };
                let (Some(text), Some(under)) = (pick(&rule.text), pick(&rule.on)) else {
                    continue;
                };
                let ratio = ratio_of(
                    luminance_of(brighten(text, brightness)),
                    luminance_of(brighten(under, brightness)),
                );
                if ratio < min {
                    findings.push(Finding::at(
                        THEME_PATH,
                        format!(
                            "Theme {}{mode}: `{}` on `{}` while it is hovered (brightened by {brightness}) has contrast {ratio:.2}:1; it needs {min:.1}:1. Change the fill or the text.",
                            theme.name, rule.text, rule.on
                        ),
                    ));
                }
            }
        }
    }
    findings
}

/// A color as `filter: brightness(factor)` draws it: each channel times the factor, at most 255.
fn brighten(color: &str, factor: f64) -> [f64; 3] {
    channels(color).map(|channel| (channel * factor).min(255.0))
}

/// The page behind a sheet, dimmed by the scrim, as it is drawn: the scrim mixed over the fill of the
/// page and over its text. The sheet must stand out from the dimmed page, and the text of the page must
/// be dim against it (M5 exit R4: behind Quick search the text of the page stayed easy to read).
fn dimmed_findings(file: &ThemeFile) -> Vec<Finding> {
    let mut findings = Vec::new();
    for rule in &file.contrast.dimmed {
        let Some(opacity) = file.opacity(&rule.opacity) else {
            findings.push(Finding::at(
                THEME_PATH,
                format!(
                    "The dimmed page uses `{}`, which is no opacity of the theme file, so it is not checked. Name an opacity.",
                    rule.opacity
                ),
            ));
            continue;
        };
        for theme in &file.themes {
            let pick = |name: &str| color(file, name, &theme.name);
            let (Some(scrim), Some(page), Some(text), Some(sheet)) = (
                pick(&rule.scrim),
                pick(&rule.page),
                pick(&rule.text),
                pick(&rule.sheet),
            ) else {
                continue;
            };
            let dimmed_page = luminance_of(fade(scrim, page, opacity));
            let dimmed_text = luminance_of(fade(scrim, text, opacity));
            let sheet_ratio = ratio_of(luminance(sheet), dimmed_page);
            let least = if theme.kind == "light" {
                rule.min_sheet_light
            } else {
                rule.min_sheet_dark
            };
            if sheet_ratio < least {
                findings.push(Finding::at(
                    THEME_PATH,
                    format!(
                        "Theme {}: `{}` is {sheet_ratio:.2}:1 against the page under `{}`; the sheet needs {least:.1}:1 to stand out. Darken `{}`.",
                        theme.name, rule.sheet, rule.scrim, rule.scrim
                    ),
                ));
            }
            let text_ratio = ratio_of(dimmed_text, dimmed_page);
            if text_ratio > rule.max_text {
                findings.push(Finding::at(
                    THEME_PATH,
                    format!(
                        "Theme {}: the text `{}` of the page under `{}` is still {text_ratio:.2}:1 against the page; behind a sheet it may be at most {:.1}:1. Darken `{}` or raise its opacity.",
                        theme.name, rule.text, rule.scrim, rule.max_text, rule.scrim
                    ),
                ));
            }
        }
    }
    findings
}

/// Words that need more than the usual ratio on some fills.
fn stronger_findings(file: &ThemeFile) -> Vec<Finding> {
    let mut findings = Vec::new();
    for rule in &file.contrast.stronger {
        for theme in &file.themes {
            for text in &rule.text {
                for fill in &rule.on {
                    let (Some(a), Some(b)) = (
                        color(file, text, &theme.name),
                        color(file, fill, &theme.name),
                    ) else {
                        continue;
                    };
                    let ratio = contrast_ratio(a, b);
                    if ratio < rule.min {
                        findings.push(Finding::at(
                            THEME_PATH,
                            format!(
                                "Theme {}: `{text}` on `{fill}` has contrast {ratio:.2}:1; the words of a row need {:.1}:1 there. Change one of the two colors.",
                                theme.name, rule.min
                            ),
                        ));
                    }
                }
            }
        }
    }
    findings
}

/// The lines that people have to see (the edge of a box to tick, the edge of a button that cannot be
/// pressed, the ring and the hover line of a row) reach their ratio on their surfaces in every theme,
/// as they are and with the device setting for more contrast. A line that is faded (`faded-by`) is checked as it is seen: its color
/// mixed with the surface behind it by the opacity.
fn line_findings(file: &ThemeFile) -> Vec<Finding> {
    let mut findings = Vec::new();
    for lines in &file.contrast.plain_lines {
        // A line with no `faded-by` is seen as it is. Loading the file checks that a name here is an
        // opacity (`ThemeFile::opacity`); a file that was not loaded that way gets a finding, not a skip.
        let opacity = match lines.faded_by.as_deref() {
            None => 1.0,
            Some(name) => {
                let Some(opacity) = file.opacity(name) else {
                    findings.push(Finding::at(
                        THEME_PATH,
                        format!(
                            "The lines {:?} fade by `{name}`, which is no opacity of the theme file, so their contrast is not checked. Name an opacity.",
                            lines.colors
                        ),
                    ));
                    continue;
                };
                opacity
            }
        };
        for theme in &file.themes {
            for line in &lines.colors {
                for surface in &lines.on {
                    // As they are, and with the colors that more contrast puts in place.
                    for (more, mode) in [(false, ""), (true, " with more contrast")] {
                        let pick = |name: &str| {
                            if more {
                                color_with_more_contrast(file, name, &theme.name)
                            } else {
                                color(file, name, &theme.name)
                            }
                        };
                        let (Some(a), Some(b)) = (pick(line), pick(surface)) else {
                            continue;
                        };
                        let ratio = ratio_of(luminance_of(fade(a, b, opacity)), luminance(b));
                        if ratio < lines.min {
                            findings.push(Finding::at(
                                THEME_PATH,
                                format!(
                                    "Theme {}{mode}: the line `{line}` on `{surface}` has contrast {ratio:.2}:1; it needs {:.1}:1. Change its value.",
                                    theme.name, lines.min
                                ),
                            ));
                        }
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
        let mut check_difference = |first: &str, second: &str, min: f64| {
            let (Some(a), Some(b)) = (
                color(file, first, &theme.name),
                color(file, second, &theme.name),
            ) else {
                return;
            };
            let difference = color_difference(a, b);
            if difference < min {
                findings.push(Finding::at(
                    THEME_PATH,
                    format!(
                        "Theme {}: `{first}` and `{second}` differ in color by only {difference:.1} (delta E); they need {min:.1} to be told apart at a glance.",
                        theme.name
                    ),
                ));
            }
        };
        for group in &spec.groups {
            for (index, state) in group.states.iter().enumerate() {
                if let Some(min) = spec.min_delta_e_against_surfaces {
                    for surface in &group.surfaces {
                        check_difference(state, surface, min);
                    }
                }
                if let Some(min) = spec.min_delta_e_between_states {
                    for other in &group.states[index + 1..] {
                        check_difference(state, other, min);
                    }
                }
            }
        }
    }
    findings
}

/// How far apart two `#RRGGBB` colors look: the distance in the CIE L*a*b* space (delta E 1976).
/// The contrast ratio only knows brightness, so a grey and a blue of the same brightness have a
/// ratio of 1 and a difference of 10 and more here.
fn color_difference(first: &str, second: &str) -> f64 {
    let (a, b) = (lab(first), lab(second));
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// A `#RRGGBB` color in CIE L*a*b* (D65).
fn lab(hex: &str) -> [f64; 3] {
    let [red, green, blue] = channels(hex).map(|channel| {
        let value = channel / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    });
    let x = (0.412_456_4 * red + 0.357_576_1 * green + 0.180_437_5 * blue) / 0.950_47;
    let y = 0.212_672_9 * red + 0.715_152_2 * green + 0.072_175 * blue;
    let z = (0.019_333_9 * red + 0.119_192 * green + 0.950_304_1 * blue) / 1.088_83;
    let curve = |value: f64| {
        if value > 216.0 / 24_389.0 {
            value.cbrt()
        } else {
            (24_389.0 / 27.0 * value + 16.0) / 116.0
        }
    };
    let (fx, fy, fz) = (curve(x), curve(y), curve(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
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
mod tests;
