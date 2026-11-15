//! The order of emphasis of the text colors (UX §5.6): the main text stands out more than the quiet
//! text, and the quiet text more than a hint. How much a color stands out is its contrast against
//! the background it is read on, so the order holds in a light theme and in a dark one alike. A theme
//! that turns it upside down is warned, never refused: its colors may still all be readable.

use serde::Serialize;

use super::maths::contrast_ratio;
use super::rating::ThemeColors;
use super::spec::ContrastSpec;

/// Two text colors whose order of emphasis is upside down: the quieter one stands out more.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UpsideDown {
    /// The color that is meant to stand out more (the quiet text).
    pub louder: String,
    /// The color that is meant to stand out less but does (the hint).
    pub quieter: String,
    /// The background the two are read on.
    pub on: String,
}

/// Every pair of text colors of the order rules whose order is upside down in `colors`, as they are
/// (not with more contrast), each once, in the order of the rules.
#[must_use]
pub fn upside_down(spec: &ContrastSpec, colors: &dyn ThemeColors) -> Vec<UpsideDown> {
    let mut found = Vec::new();
    for order in spec.order.iter() {
        let Some(background) = colors.color(&order.on) else {
            continue;
        };
        let stands_out = |name: &str| {
            colors
                .color(name)
                .map(|color| contrast_ratio(color, background))
        };
        for (at, louder) in order.text.iter().enumerate() {
            for quieter in order.text.iter().skip(at + 1) {
                if let (Some(loud), Some(quiet)) = (stands_out(louder), stands_out(quieter))
                    && quiet > loud
                {
                    found.push(UpsideDown {
                        louder: louder.to_string(),
                        quieter: quieter.to_string(),
                        on: order.on.to_string(),
                    });
                }
            }
        }
    }
    found
}
