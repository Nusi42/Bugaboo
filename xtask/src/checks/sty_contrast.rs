//! STY-CONTRAST: every text and background pair of the four built-in themes passes WCAG AA.
//!
//! Every text color is checked on every background color. The few combinations that are never
//! used are written down, with the reason, in `design/theme.kdl` (the `contrast` line) (THM-03,
//! UX-04). The check runs for every theme file of `design/themes/` and for every mode that changes
//! colors: the usual one, and the one for a device that asks for more contrast (THM-06). A color
//! that names another is checked with the color it leads to, and a finding about a theme names the
//! theme's file.

use bb_core::contrast::{Evaluation, Mode, PairResult, Problem, Rule, ThemeColors, rate};

use super::{Check, Context, Finding};
use crate::generate::theme_file::{LightOrDark, THEME_PATH, Theme, ThemeFile};

pub(crate) struct StyContrast;

impl Check for StyContrast {
    fn id(&self) -> &'static str {
        "STY-CONTRAST"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(&ThemeFile::load(context.repo)?))
    }
}

/// Where a color of a theme is set, `file:line`: the line of the theme's file, or of the base where
/// the theme keeps the base's color. A finding points there, at what is changed to fix it.
fn place(file: &ThemeFile, name: &str, theme: &Theme) -> String {
    file.place_of(name, &theme.name)
        .unwrap_or_else(|| theme.path.clone())
}

/// One theme of the file, for the maths of `bb_core::contrast`.
struct InTheme<'a> {
    file: &'a ThemeFile,
    theme: &'a Theme,
}

impl ThemeColors for InTheme<'_> {
    fn is_light(&self) -> bool {
        self.theme.kind == LightOrDark::Light
    }

    fn color(&self, name: &str) -> Option<&str> {
        self.file.color(name, &self.theme.name)
    }

    fn color_with_more_contrast(&self, name: &str) -> Option<&str> {
        self.file.color_with_more_contrast(name, &self.theme.name)
    }

    fn opacity(&self, name: &str) -> Option<f64> {
        self.file.opacity(name)
    }

    fn number(&self, name: &str) -> Option<f64> {
        self.file.number(name)
    }
}

fn evaluate(file: &ThemeFile) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    for theme in &file.themes {
        let colors = InTheme { file, theme };
        let Evaluation { results, problems } =
            rate(&file.contrast, &file.distinct, &theme.name, &colors);
        let new =
            problems
                .iter()
                .map(problem_finding)
                .chain(results.iter().filter(|r| r.is_hard()).map(|result| {
                    Finding::at(
                        place(file, &result.place, theme),
                        message(file, theme, result),
                    )
                }));
        for finding in new {
            // A rule that names something the theme file lacks says so once, not once per theme.
            if !findings
                .iter()
                .any(|known| known.message == finding.message && known.location == finding.location)
            {
                findings.push(finding);
            }
        }
    }
    findings
}

fn problem_finding(problem: &Problem) -> Finding {
    Finding::at(
        THEME_PATH,
        match problem {
            Problem::NoNumber {
                text,
                on,
                brightness,
            } => format!(
                "The hovered pair `{text}` on `{on}` uses `{brightness}`, which is no number token, so it is not checked. Name a number."
            ),
            Problem::NoOpacityForLines { colors, name } => format!(
                "The lines {colors:?} fade by `{name}`, which is no opacity of the theme file, so their contrast is not checked. Name an opacity."
            ),
            Problem::NoOpacityForDimmed { name } => format!(
                "The dimmed page uses `{name}`, which is no opacity of the theme file, so it is not checked. Name an opacity."
            ),
        },
    )
}

/// What a finding says about a pair that is *Hard to read*.
fn message(file: &ThemeFile, theme: &Theme, result: &PairResult) -> String {
    let name = &theme.name;
    let (first, second) = (&result.first, &result.second);
    let (value, limit) = (result.value, result.limit);
    let mode = match result.mode {
        Mode::Usual => "",
        Mode::MoreContrast => " with more contrast",
    };
    match &result.rule {
        Rule::Text => format!(
            "Theme {name}: `{first}` on `{second}` has contrast {value:.2}:1; it needs {limit:.1}:1. Change one of the two colors, or record why this pair is never used."
        ),
        Rule::Drawn(drawn) => format!(
            "Theme {name}{mode}: the {} `{first}` on `{second}` has contrast {value:.2}:1; it needs {limit:.1}:1. Change its value.",
            drawn.word()
        ),
        Rule::MoreText | Rule::MoreLine => format!(
            "Theme {name} with more contrast: {} `{first}` on `{second}` has contrast {value:.2}:1; it needs {limit:.1}:1. Change its `more=` value in {}.",
            if result.rule == Rule::MoreText {
                "text"
            } else {
                "line"
            },
            place(file, first, theme)
        ),
        Rule::Hovered { brightness } => format!(
            "Theme {name}{mode}: `{first}` on `{second}` while it is hovered (brightened by {brightness}) has contrast {value:.2}:1; it needs {limit:.1}:1. Change the fill or the text."
        ),
        Rule::Stronger => format!(
            "Theme {name}: `{first}` on `{second}` has contrast {value:.2}:1; the words of a row need {limit:.1}:1 there. Change one of the two colors."
        ),
        Rule::DimmedSheet => format!(
            "Theme {name}: `{first}` is {value:.2}:1 against the page under `{second}`; the sheet needs {limit:.1}:1 to stand out. Darken `{second}`."
        ),
        Rule::DimmedText => format!(
            "Theme {name}: the text `{first}` of the page under `{second}` is still {value:.2}:1 against the page; behind a sheet it may be at most {limit:.1}:1. Darken `{second}` or raise its opacity."
        ),
        Rule::DistinctRatio => format!(
            "Theme {name}: `{first}` and `{second}` differ by only {value:.3}:1; they need {limit:.2}:1 so the state stays visible."
        ),
        Rule::DistinctDeltaE => format!(
            "Theme {name}: `{first}` and `{second}` differ in color by only {value:.1} (delta E); they need {limit:.1} to be told apart at a glance."
        ),
        Rule::DistinctChroma => format!(
            "Theme {name}: `{first}` has a hue of its own (chroma {value:.1}); it must be a neutral grey, at most {limit:.1}."
        ),
    }
}

#[cfg(test)]
use bb_core::contrast::{contrast_ratio, fade};

#[cfg(test)]
mod tests;
