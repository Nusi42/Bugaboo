//! The colors of an own theme as the contrast rules read them: the built-in theme it extends with
//! its own values laid over it, every link between colors followed (M8-Q9).

use std::collections::HashMap;

use super::values::OwnValues;
use super::{BuiltIn, built_in, built_in_theme, color_tokens};
use crate::contrast::{Evaluation, Rules, ThemeColors, rate};

/// Which colors a link is followed in.
#[derive(Clone, Copy)]
enum Mode {
    Usual,
    MoreContrast,
}

/// The colors of one theme, each with its links followed, usual and with more contrast.
#[derive(Debug)]
pub struct OwnColors {
    light: bool,
    usual: HashMap<&'static str, String>,
    more: HashMap<&'static str, String>,
}

impl OwnColors {
    /// The colors of `extends` with `values` laid over them. A line of an own theme replaces the
    /// color and its color for more contrast together, as in a theme file: without a color of
    /// its own for more contrast, the color itself shows there.
    #[must_use]
    pub fn new(extends: &'static BuiltIn, values: &OwnValues) -> Self {
        let (mut usual, mut more) = (HashMap::new(), HashMap::new());
        for name in color_tokens() {
            if let Some(color) = value_of(extends, values, name, Mode::Usual) {
                usual.insert(name, color);
            }
            if let Some(color) = value_of(extends, values, name, Mode::MoreContrast) {
                more.insert(name, color);
            }
        }
        Self {
            light: extends.light,
            usual,
            more,
        }
    }
}

/// The color of a token with every link followed; `None` for a name that is no color or a loop.
fn value_of(extends: &BuiltIn, values: &OwnValues, name: &str, mode: Mode) -> Option<String> {
    let mut current = name;
    for _ in 0..=color_tokens().count() {
        let value = if let Some(own) = values.colors.get(current) {
            match mode {
                Mode::Usual => own.value.as_str(),
                Mode::MoreContrast => own.more.as_deref().unwrap_or(&own.value),
            }
        } else {
            let base = extends.color(current)?;
            match mode {
                Mode::Usual => base.value,
                Mode::MoreContrast => base.more.unwrap_or(base.value),
            }
        };
        let Some(next) = value.strip_prefix('$') else {
            return Some(value.to_ascii_lowercase());
        };
        current = next;
    }
    None
}

impl ThemeColors for OwnColors {
    fn is_light(&self) -> bool {
        self.light
    }

    fn color(&self, name: &str) -> Option<&str> {
        self.usual.get(name).map(String::as_str)
    }

    fn color_with_more_contrast(&self, name: &str) -> Option<&str> {
        self.more.get(name).map(String::as_str)
    }

    fn opacity(&self, name: &str) -> Option<f64> {
        built_in::OPACITIES
            .iter()
            .find(|number| number.name == name)
            .map(|number| number.value)
    }

    fn number(&self, name: &str) -> Option<f64> {
        built_in::NUMBERS
            .iter()
            .find(|number| number.name == name)
            .map(|number| number.value)
    }
}

/// Every pair of the contrast rules for the own theme that extends the built-in theme `extends`
/// with `values`, each rated *Easy*, *OK* or *Hard to read*. `None` when no built-in theme has that
/// ID.
#[must_use]
pub fn rate_own(extends: &str, values: &OwnValues, rules: &Rules) -> Option<Evaluation> {
    let base = built_in_theme(extends)?;
    let colors = OwnColors::new(base, values);
    Some(rate(&rules.contrast, &rules.distinct, extends, &colors))
}
