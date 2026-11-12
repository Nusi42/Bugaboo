//! Rating the pairs of one theme against the contrast rules (M8-Q9): for each pair of each rule the
//! ratio or the difference in color, and one rating, *Easy*, *OK* or *Hard to read*.

use super::maths::{
    brighten, chroma_of, color_difference, contrast_ratio, fade, luminance, luminance_of, ratio_of,
};
use super::spec::{ContrastSpec, DistinctSpec, Drawn};

/// How readable a pair is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rating {
    /// Passes the rule for more contrast too (text only).
    Easy,
    /// Passes its rule.
    Ok,
    /// Does not pass its rule.
    HardToRead,
}

/// Which colors a pair is read in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The theme as it is.
    Usual,
    /// With the colors that the device setting for more contrast puts in place.
    MoreContrast,
}

/// Which rule a pair belongs to, with what a message about it needs.
#[derive(Clone, Debug, PartialEq)]
pub enum Rule {
    /// A text color on a background (`contrast`).
    Text,
    /// A line or a fill without text (`lines`, `fills`).
    Drawn(Drawn),
    /// A text color with more contrast (`more`).
    MoreText,
    /// A line with more contrast (`more`).
    MoreLine,
    /// A pair as it is seen while it is brightened by this much (`hovered`).
    Hovered {
        /// How much the fill is brightened.
        brightness: f64,
    },
    /// Words that need more than the usual ratio (`stronger`).
    Stronger,
    /// The sheet against the dimmed page; `first` is the sheet, `second` the scrim.
    DimmedSheet,
    /// The text of the dimmed page; `first` is the text, `second` the scrim.
    DimmedText,
    /// Two states by their ratio (`distinct`).
    DistinctRatio,
    /// Two states by their difference in color (`distinct`).
    DistinctDeltaE,
    /// A state that has to be a neutral grey (`distinct`).
    DistinctChroma,
}

/// One pair and how it was rated.
#[derive(Clone, Debug, PartialEq)]
pub struct PairResult {
    /// The rule the pair belongs to.
    pub rule: Rule,
    /// The colors the pair is read in.
    pub mode: Mode,
    /// The first color (the text, the line, the state).
    pub first: String,
    /// The second color (the background, the surface, the other state).
    pub second: String,
    /// The color that has to change to mend the pair: the one a message points at.
    pub place: String,
    /// The ratio, the difference in color (delta E) or the chroma, as the rule measures it.
    pub value: f64,
    /// The least (or, for the dimmed text and the chroma, the most) the rule allows.
    pub limit: f64,
    /// How readable the pair is.
    pub rating: Rating,
}

impl PairResult {
    /// Whether the pair is *Hard to read*.
    #[must_use]
    pub fn is_hard(&self) -> bool {
        self.rating == Rating::HardToRead
    }
}

/// A rule that names something the theme does not have, so it cannot be checked.
#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    /// The `brightness` of a hovered pair is no number token.
    NoNumber {
        /// The text color of the pair.
        text: String,
        /// The fill it is drawn on.
        on: String,
        /// The name that is no number token.
        brightness: String,
    },
    /// The opacity a group of lines fades by is no opacity token.
    NoOpacityForLines {
        /// The colors of the group.
        colors: Vec<String>,
        /// The name that is no opacity token.
        name: String,
    },
    /// The opacity of the dimmed page is no opacity token.
    NoOpacityForDimmed {
        /// The name that is no opacity token.
        name: String,
    },
}

/// What the rules find in one theme: every pair rated, and what could not be checked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Evaluation {
    /// Every pair, rated.
    pub results: Vec<PairResult>,
    /// What could not be checked.
    pub problems: Vec<Problem>,
}

impl Evaluation {
    /// The pairs that are *Hard to read*.
    pub fn hard(&self) -> impl Iterator<Item = &PairResult> {
        self.results.iter().filter(|result| result.is_hard())
    }
}

/// The colors and numbers of one theme, however they are kept: the theme files (the check) or the
/// base theme with the values of an own theme laid over it (the server).
pub trait ThemeColors {
    /// Whether the theme is a light one.
    fn is_light(&self) -> bool;
    /// The `#rrggbb` color of a token, with a token that names another followed to its value.
    fn color(&self, name: &str) -> Option<&str>;
    /// The same with the colors that more contrast puts in place.
    fn color_with_more_contrast(&self, name: &str) -> Option<&str>;
    /// The value of an opacity token, 0 to 1.
    fn opacity(&self, name: &str) -> Option<f64>;
    /// The value of a number token.
    fn number(&self, name: &str) -> Option<f64>;
}

fn text_rating(ratio: f64, min: f64, easy_at: Option<f64>) -> Rating {
    if ratio < min {
        Rating::HardToRead
    } else if easy_at.is_some_and(|easy| ratio >= easy) {
        Rating::Easy
    } else {
        Rating::Ok
    }
}

fn plain_rating(passes: bool) -> Rating {
    if passes {
        Rating::Ok
    } else {
        Rating::HardToRead
    }
}

/// Every pair of every rule in one theme, rated. `theme` is the ID of the theme, for the exclusions
/// that apply to some themes only.
#[must_use]
pub fn rate(
    spec: &ContrastSpec,
    distinct: &[DistinctSpec],
    theme: &str,
    colors: &dyn ThemeColors,
) -> Evaluation {
    let mut found = Evaluation::default();
    text_pairs(spec, theme, colors, &mut found);
    line_pairs(spec, colors, &mut found);
    more_pairs(spec, theme, colors, &mut found);
    hovered_pairs(spec, colors, &mut found);
    stronger_pairs(spec, colors, &mut found);
    dimmed_pairs(spec, colors, &mut found);
    for rule in distinct {
        distinct_pairs(rule, colors, &mut found);
    }
    found
}

fn text_pairs(spec: &ContrastSpec, theme: &str, colors: &dyn ThemeColors, found: &mut Evaluation) {
    for (text, background) in spec.pairs(theme) {
        let (Some(a), Some(b)) = (colors.color(text), colors.color(background)) else {
            continue;
        };
        let ratio = contrast_ratio(a, b);
        let easy_at = spec
            .more
            .text
            .iter()
            .any(|name| name == text)
            .then_some(spec.more.min_text);
        found.results.push(PairResult {
            rule: Rule::Text,
            mode: Mode::Usual,
            first: text.to_owned(),
            second: background.to_owned(),
            place: text.to_owned(),
            value: ratio,
            limit: spec.min,
            rating: text_rating(ratio, spec.min, easy_at),
        });
    }
}

/// Lines that people have to see, and fills that carry no text: as they are and with more contrast. A
/// line that is faded is checked as it is seen: its color mixed with the surface behind it.
fn line_pairs(spec: &ContrastSpec, colors: &dyn ThemeColors, found: &mut Evaluation) {
    for lines in &spec.plain_lines {
        let opacity = match lines.faded_by.as_deref() {
            None => 1.0,
            Some(name) => {
                let Some(opacity) = colors.opacity(name) else {
                    found.problems.push(Problem::NoOpacityForLines {
                        colors: lines.colors.clone(),
                        name: name.to_owned(),
                    });
                    continue;
                };
                opacity
            }
        };
        for line in &lines.colors {
            for surface in &lines.on {
                for mode in [Mode::Usual, Mode::MoreContrast] {
                    let (Some(a), Some(b)) =
                        (pick(colors, mode, line), pick(colors, mode, surface))
                    else {
                        continue;
                    };
                    let ratio = ratio_of(luminance_of(fade(a, b, opacity)), luminance(b));
                    found.results.push(PairResult {
                        rule: Rule::Drawn(lines.drawn),
                        mode,
                        first: line.clone(),
                        second: surface.clone(),
                        place: line.clone(),
                        value: ratio,
                        limit: lines.min,
                        rating: plain_rating(ratio >= lines.min),
                    });
                }
            }
        }
    }
}

fn pick<'a>(colors: &'a dyn ThemeColors, mode: Mode, name: &str) -> Option<&'a str> {
    match mode {
        Mode::Usual => colors.color(name),
        Mode::MoreContrast => colors.color_with_more_contrast(name),
    }
}

/// For people whose device asks for more contrast (THM-06): the listed text reaches the higher ratio
/// on its backgrounds and the listed lines are visible on the plain surfaces.
fn more_pairs(spec: &ContrastSpec, theme: &str, colors: &dyn ThemeColors, found: &mut Evaluation) {
    let more = &spec.more;
    let mut check = |first: &str, second: &str, min: f64, rule: Rule| {
        let (Some(a), Some(b)) = (
            colors.color_with_more_contrast(first),
            colors.color_with_more_contrast(second),
        ) else {
            return;
        };
        let ratio = contrast_ratio(a, b);
        found.results.push(PairResult {
            rule,
            mode: Mode::MoreContrast,
            first: first.to_owned(),
            second: second.to_owned(),
            place: first.to_owned(),
            value: ratio,
            limit: min,
            rating: plain_rating(ratio >= min),
        });
    };
    for (text, background) in spec.pairs(theme) {
        if more.text.iter().any(|name| name == text) {
            check(text, background, more.min_text, Rule::MoreText);
        }
    }
    for line in &more.lines {
        for surface in &more.line_on {
            check(line, surface, more.min_line, Rule::MoreLine);
        }
    }
}

/// A fill that is brightened while it is pointed at brightens its text too: the pair is checked as it
/// is seen then, as it is and with more contrast.
fn hovered_pairs(spec: &ContrastSpec, colors: &dyn ThemeColors, found: &mut Evaluation) {
    for rule in &spec.hovered {
        let Some(brightness) = colors.number(&rule.brightness) else {
            found.problems.push(Problem::NoNumber {
                text: rule.text.clone(),
                on: rule.on.clone(),
                brightness: rule.brightness.clone(),
            });
            continue;
        };
        for (mode, min) in [(Mode::Usual, rule.min), (Mode::MoreContrast, rule.min_more)] {
            let (Some(text), Some(under)) =
                (pick(colors, mode, &rule.text), pick(colors, mode, &rule.on))
            else {
                continue;
            };
            let ratio = ratio_of(
                luminance_of(brighten(text, brightness)),
                luminance_of(brighten(under, brightness)),
            );
            found.results.push(PairResult {
                rule: Rule::Hovered { brightness },
                mode,
                first: rule.text.clone(),
                second: rule.on.clone(),
                place: rule.text.clone(),
                value: ratio,
                limit: min,
                rating: plain_rating(ratio >= min),
            });
        }
    }
}

/// Words that need more than the usual ratio on some fills.
fn stronger_pairs(spec: &ContrastSpec, colors: &dyn ThemeColors, found: &mut Evaluation) {
    for rule in &spec.stronger {
        for text in &rule.text {
            for fill in &rule.on {
                let (Some(a), Some(b)) = (colors.color(text), colors.color(fill)) else {
                    continue;
                };
                let ratio = contrast_ratio(a, b);
                found.results.push(PairResult {
                    rule: Rule::Stronger,
                    mode: Mode::Usual,
                    first: text.clone(),
                    second: fill.clone(),
                    place: text.clone(),
                    value: ratio,
                    limit: rule.min,
                    rating: plain_rating(ratio >= rule.min),
                });
            }
        }
    }
}

/// The page behind a sheet, dimmed by the scrim as it is drawn: the scrim mixed over the fill of the
/// page and over its text. The sheet must stand out from the dimmed page, and the text of the page
/// must be dim against it (M5 exit R4).
fn dimmed_pairs(spec: &ContrastSpec, colors: &dyn ThemeColors, found: &mut Evaluation) {
    for rule in &spec.dimmed {
        let Some(opacity) = colors.opacity(&rule.opacity) else {
            found.problems.push(Problem::NoOpacityForDimmed {
                name: rule.opacity.clone(),
            });
            continue;
        };
        let (Some(scrim), Some(page), Some(text), Some(sheet)) = (
            colors.color(&rule.scrim),
            colors.color(&rule.page),
            colors.color(&rule.text),
            colors.color(&rule.sheet),
        ) else {
            continue;
        };
        let dimmed_page = luminance_of(fade(scrim, page, opacity));
        let dimmed_text = luminance_of(fade(scrim, text, opacity));
        let sheet_ratio = ratio_of(luminance(sheet), dimmed_page);
        let least = if colors.is_light() {
            rule.min_sheet_light
        } else {
            rule.min_sheet_dark
        };
        found.results.push(PairResult {
            rule: Rule::DimmedSheet,
            mode: Mode::Usual,
            first: rule.sheet.clone(),
            second: rule.scrim.clone(),
            place: rule.scrim.clone(),
            value: sheet_ratio,
            limit: least,
            rating: plain_rating(sheet_ratio >= least),
        });
        let text_ratio = ratio_of(dimmed_text, dimmed_page);
        found.results.push(PairResult {
            rule: Rule::DimmedText,
            mode: Mode::Usual,
            first: rule.text.clone(),
            second: rule.scrim.clone(),
            place: rule.scrim.clone(),
            value: text_ratio,
            limit: rule.max_text,
            rating: plain_rating(text_ratio <= rule.max_text),
        });
    }
}

/// States that must stay visible against the surfaces and each other, by ratio, by difference in
/// color, and by how little hue they have.
fn distinct_pairs(rule: &DistinctSpec, colors: &dyn ThemeColors, found: &mut Evaluation) {
    let mut push = |rule: Rule, first: &str, second: &str, value: f64, limit: f64, passes: bool| {
        found.results.push(PairResult {
            rule,
            mode: Mode::Usual,
            first: first.to_owned(),
            second: second.to_owned(),
            place: first.to_owned(),
            value,
            limit,
            rating: plain_rating(passes),
        });
    };
    let mut ratio = |first: &str, second: &str, min: f64| {
        if let (Some(a), Some(b)) = (colors.color(first), colors.color(second)) {
            let value = contrast_ratio(a, b);
            push(Rule::DistinctRatio, first, second, value, min, value >= min);
        }
    };
    for group in &rule.groups {
        for (index, state) in group.states.iter().enumerate() {
            for surface in &group.surfaces {
                ratio(state, surface, rule.min_against_surfaces);
            }
            for other in &group.states[index + 1..] {
                ratio(state, other, rule.min_between_states);
            }
        }
    }
    let mut difference = |first: &str, second: &str, min: f64| {
        if let (Some(a), Some(b)) = (colors.color(first), colors.color(second)) {
            let value = color_difference(a, b);
            push(
                Rule::DistinctDeltaE,
                first,
                second,
                value,
                min,
                value >= min,
            );
        }
    };
    for group in &rule.groups {
        for (index, state) in group.states.iter().enumerate() {
            if let Some(min) = rule.min_delta_e_against_surfaces {
                for surface in &group.surfaces {
                    difference(state, surface, min);
                }
            }
            if let Some(min) = rule.min_delta_e_between_states {
                for other in &group.states[index + 1..] {
                    difference(state, other, min);
                }
            }
        }
    }
    if let Some(max) = rule.max_chroma_of_states {
        for state in rule.groups.iter().flat_map(|group| &group.states) {
            if let Some(value) = colors.color(state) {
                let chroma = chroma_of(value);
                push(
                    Rule::DistinctChroma,
                    state,
                    state,
                    chroma,
                    max,
                    chroma <= max,
                );
            }
        }
    }
}
