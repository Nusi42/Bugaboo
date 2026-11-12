//! What the rated pairs of an own theme come to (THM-03): each pair once at its worst rating, the
//! sums of the line on top of the theme editor and of a list of themes, the pairs that are *Hard to
//! read* that a save of a theme for everyone is refused for, and for each color its worst rating and
//! the colors it is hard to read with. Written once, for the server and for the editor (through
//! `bb-wasm`).

use serde::Serialize;

use super::colors::{RatedPair, ShownColor, readability, shown_colors};
use super::values::OwnValues;
use crate::contrast::Rating;

/// A pair of colors by its tokens: the first color, the second and the mode it is read in. The
/// rules may rate one pair several times; it counts once, at its worst rating.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NamedPair {
    /// The first color: the text, the line or the state.
    pub first: String,
    /// The second color: the background, the surface or the other state.
    pub second: String,
    /// The color to change to mend it.
    pub place: String,
    /// Whether the pair is read with the colors for more contrast.
    pub more_contrast: bool,
}

/// Each pair once, at its worst rating, with the color to change of the rule that rated it worst,
/// in the order of the first rating of each pair.
fn worst_of_each(pairs: &[RatedPair]) -> Vec<(NamedPair, Rating)> {
    let mut found: Vec<(NamedPair, Rating)> = Vec::new();
    for pair in pairs {
        let named = NamedPair {
            first: pair.first.clone(),
            second: pair.second.clone(),
            place: pair.place.clone(),
            more_contrast: pair.more_contrast,
        };
        let known = found.iter_mut().find(|(seen, _)| {
            seen.first == named.first
                && seen.second == named.second
                && seen.more_contrast == named.more_contrast
        });
        match known {
            Some(known) if pair.rating > known.1 => *known = (named, pair.rating),
            Some(_) => {}
            None => found.push((named, pair.rating)),
        }
    }
    found
}

/// How many pairs of an own theme are *Easy*, *OK* and *Hard to read*: the line on top of the theme
/// editor and the rating in a list of themes. A pair is its two colors in one mode (as it is, or
/// with more contrast) and counts once, at its worst rating.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ReadabilitySummary {
    /// The pairs that are *Easy*.
    pub easy: u32,
    /// The pairs that are *OK*.
    pub ok: u32,
    /// The pairs that are *Hard to read*.
    pub hard_to_read: u32,
    /// The colors to change to mend them: what a list of themes and the editor say.
    pub colors_hard_to_read: u32,
}

impl ReadabilitySummary {
    /// The summary of the rated pairs of one theme.
    #[must_use]
    pub fn of(pairs: &[RatedPair]) -> Self {
        let mut summary = Self::default();
        let mut places: Vec<String> = Vec::new();
        for (pair, rating) in worst_of_each(pairs) {
            match rating {
                Rating::Easy => summary.easy += 1,
                Rating::Ok => summary.ok += 1,
                Rating::HardToRead => {
                    summary.hard_to_read += 1;
                    if !places.contains(&pair.place) {
                        places.push(pair.place);
                    }
                }
            }
        }
        summary.colors_hard_to_read = u32::try_from(places.len()).unwrap_or(u32::MAX);
        summary
    }
}

/// The pairs that are *Hard to read*, each once: the refusal of a theme for everyone, the warning
/// of a personal one. As many as [`ReadabilitySummary::hard_to_read`].
#[must_use]
pub fn hard_pairs(pairs: &[RatedPair]) -> Vec<NamedPair> {
    worst_of_each(pairs)
        .into_iter()
        .filter(|(_, rating)| *rating == Rating::HardToRead)
        .map(|(pair, _)| pair)
        .collect()
}

/// The worst rating of the pairs one color is in, as it is and with more contrast.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ColorRating {
    /// The name of the token.
    pub name: String,
    /// The worst rating as it is; none when the color is in no pair read so.
    pub usual: Option<Rating>,
    /// The worst rating with more contrast; none when the color is in no pair read so.
    pub more: Option<Rating>,
}

/// For each color in a pair, its worst rating in each mode, in the order the pairs name them.
#[must_use]
pub fn color_ratings(pairs: &[RatedPair]) -> Vec<ColorRating> {
    let mut found: Vec<ColorRating> = Vec::new();
    for pair in pairs {
        for name in [&pair.first, &pair.second, &pair.place] {
            let index = found
                .iter()
                .position(|color| &color.name == name)
                .unwrap_or_else(|| {
                    found.push(ColorRating {
                        name: name.clone(),
                        usual: None,
                        more: None,
                    });
                    found.len() - 1
                });
            if let Some(color) = found.get_mut(index) {
                let mode = if pair.more_contrast {
                    &mut color.more
                } else {
                    &mut color.usual
                };
                *mode = (*mode).max(Some(pair.rating));
            }
        }
    }
    found
}

/// A color to change because it is hard to read, and the colors it is hard to read with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HardColor {
    /// The color to change.
    pub place: String,
    /// The other colors of its pairs that are hard to read, each once, in either mode.
    pub with: Vec<String>,
}

/// The colors to change, each with the colors it is hard to read with, in the order of the pairs.
#[must_use]
pub fn hard_colors(pairs: &[NamedPair]) -> Vec<HardColor> {
    let mut found: Vec<HardColor> = Vec::new();
    for pair in pairs {
        let other = if pair.first == pair.place {
            &pair.second
        } else {
            &pair.first
        };
        let index = found
            .iter()
            .position(|color| color.place == pair.place)
            .unwrap_or_else(|| {
                found.push(HardColor {
                    place: pair.place.clone(),
                    with: Vec::new(),
                });
                found.len() - 1
            });
        if let Some(color) = found.get_mut(index)
            && !color.with.contains(other)
        {
            color.with.push(other.clone());
        }
    }
    found
}

/// What the theme editor is told about the theme it edits, worked out as a save would be rated.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ThemeRead {
    /// How many pairs are *Easy*, *OK* and *Hard to read*.
    pub summary: ReadabilitySummary,
    /// The pairs that are *Hard to read*, each once.
    pub hard: Vec<NamedPair>,
    /// The colors to change, each with the colors it is hard to read with.
    pub hard_colors: Vec<HardColor>,
    /// The worst rating of each color, as it is and with more contrast.
    pub ratings: Vec<ColorRating>,
    /// Every color the theme may set, as the page shows it, in the order of the theme file.
    pub colors: Vec<ShownColor>,
}

/// What the theme editor is told about the own theme that extends `extends` with `values`; `None`
/// when no built-in theme has that ID.
#[must_use]
pub fn theme_read(extends: &str, values: &OwnValues) -> Option<ThemeRead> {
    let pairs = readability(extends, values)?;
    let hard = hard_pairs(&pairs);
    Some(ThemeRead {
        summary: ReadabilitySummary::of(&pairs),
        hard_colors: hard_colors(&hard),
        hard,
        ratings: color_ratings(&pairs),
        colors: shown_colors(extends, values)?,
    })
}
