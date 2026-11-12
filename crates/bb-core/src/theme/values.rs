//! The values an own theme stores, and the check of them (THM-04, ADR-0026 update M8). A value is
//! a color, the parts of a shadow, a corner radius in pixels or the name of a text stack: never
//! text that reaches a style sheet as it is written. Anything that is not on the list of what an
//! own theme may set is refused with its name.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    SHADOW_LAYERS_MOST, SHADOW_PIXELS_MOST, built_in, color_tokens, font_tokens, radius_tokens,
};

/// A color of an own theme and, optionally, its color for a device that asks for more contrast.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColorChoice {
    /// `#rrggbb`.
    pub value: String,
    /// `#rrggbb` for more contrast; it is the color itself where it is not set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub more: Option<String>,
}

/// One layer of a shadow: offsets, blur, spread and a color.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShadowLayer {
    /// How far right, in pixels (left when negative).
    pub x: i32,
    /// How far down, in pixels (up when negative).
    pub y: i32,
    /// How blurred, in pixels.
    pub blur: u32,
    /// How far it spreads, in pixels (shrinks when negative).
    pub spread: i32,
    /// `#rrggbb` or `#rrggbbaa`.
    pub color: String,
}

/// What an own theme changes of the built-in theme it extends.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnValues {
    /// The colors, by the name of the token.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub colors: BTreeMap<String, ColorChoice>,
    /// The shadow, as at most two layers; `None` keeps the shadow of the built-in theme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<Vec<ShadowLayer>>,
    /// The corner radii in pixels, by the name of the token.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub radii: BTreeMap<String, u32>,
    /// The text faces: the name of the token that has the stack to use, by the name of the token.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fonts: BTreeMap<String, String>,
}

words! {
    /// Why a value of an own theme is refused.
    pub enum ValueProblemKind {
        /// The token is not one an own theme may set.
        UnknownToken => "unknown-name",
        /// The value is not a color of the form `#rrggbb`.
        NotAColor => "not-a-color",
        /// The color for more contrast is the color itself, which says nothing.
        MoreIsTheSame => "more-is-the-same",
        /// The shadow has no layer, too many, a part out of range or a color that is none.
        BadShadow => "bad-shadow",
        /// The radius is outside the bounds of the base theme.
        RadiusOutOfBounds => "radius-out-of-bounds",
        /// The value does not name one of the text stacks.
        NotAStack => "not-a-stack",
        /// The value is of another shape than the token takes.
        BadShape => "bad-shape",
    }
}

words_in_api!(
    ValueProblemKind,
    "Why a value of an own theme is refused: a token it may not set, a color, shadow, corner or text face that is not allowed, or a value of the wrong shape."
);

/// A value that is refused, with the name it is stored under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueProblem {
    /// The name of the token, or of the part of the file that is not on the list.
    pub token: String,
    /// Why.
    pub kind: ValueProblemKind,
}

impl ValueProblem {
    pub(crate) fn new(token: &str, kind: ValueProblemKind) -> Self {
        Self {
            token: token.to_owned(),
            kind,
        }
    }
}

/// Whether `text` is `#` and exactly six hexadecimal digits (eight with `alpha`).
fn is_hex_color(text: &str, alpha: bool) -> bool {
    let digits = text.strip_prefix('#').unwrap_or("");
    (digits.len() == 6 || (alpha && digits.len() == 8))
        && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl OwnValues {
    /// Whether the theme changes nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    /// Every value that is refused, by the token it is stored under; none when the values can be
    /// saved.
    #[must_use]
    pub fn problems(&self) -> Vec<ValueProblem> {
        let mut problems = Vec::new();
        let known: Vec<&str> = color_tokens().collect();
        for (name, choice) in &self.colors {
            if !known.contains(&name.as_str()) {
                problems.push(ValueProblem::new(name, ValueProblemKind::UnknownToken));
            } else if !is_hex_color(&choice.value, false)
                || choice
                    .more
                    .as_deref()
                    .is_some_and(|more| !is_hex_color(more, false))
            {
                problems.push(ValueProblem::new(name, ValueProblemKind::NotAColor));
            } else if choice
                .more
                .as_deref()
                .is_some_and(|more| more.eq_ignore_ascii_case(&choice.value))
            {
                problems.push(ValueProblem::new(name, ValueProblemKind::MoreIsTheSame));
            }
        }
        if let Some(layers) = &self.shadow
            && !shadow_is_allowed(layers)
        {
            problems.push(ValueProblem::new(
                built_in::SHADOW_TOKEN,
                ValueProblemKind::BadShadow,
            ));
        }
        for (name, pixels) in &self.radii {
            match radius_tokens().iter().find(|radius| radius.name == name) {
                None => problems.push(ValueProblem::new(name, ValueProblemKind::UnknownToken)),
                Some(radius) if !(radius.min_px..=radius.max_px).contains(pixels) => {
                    problems.push(ValueProblem::new(name, ValueProblemKind::RadiusOutOfBounds));
                }
                Some(_) => {}
            }
        }
        for (name, stack) in &self.fonts {
            let stacks = font_tokens();
            if !stacks.iter().any(|font| font.name == name) {
                problems.push(ValueProblem::new(name, ValueProblemKind::UnknownToken));
            } else if !stacks.iter().any(|font| font.name == stack) {
                problems.push(ValueProblem::new(name, ValueProblemKind::NotAStack));
            }
        }
        problems
    }

    /// The same values with every color written in lower case, so the same color is stored once in
    /// one way. Call it after [`Self::problems`] found nothing.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        for choice in self.colors.values_mut() {
            choice.value.make_ascii_lowercase();
            if let Some(more) = &mut choice.more {
                more.make_ascii_lowercase();
            }
        }
        for layer in self.shadow.iter_mut().flatten() {
            layer.color.make_ascii_lowercase();
        }
        self
    }

    /// Reads values from a JSON object (the part `values` of an export or an upload). A key or a
    /// token that is not on the list, and a value of the wrong shape, come back as problems that
    /// name them, never as text that is kept.
    ///
    /// # Errors
    ///
    /// The problems found, when there are any.
    pub fn from_json(value: &serde_json::Value) -> Result<Self, Vec<ValueProblem>> {
        let Some(object) = value.as_object() else {
            return Err(vec![ValueProblem::new(
                "values",
                ValueProblemKind::BadShape,
            )]);
        };
        let mut problems = Vec::new();
        let mut values = Self::default();
        for (key, part) in object {
            match key.as_str() {
                "colors" => read_map(part, key, &mut problems, &mut values.colors),
                "radii" => read_map(part, key, &mut problems, &mut values.radii),
                "fonts" => read_map(part, key, &mut problems, &mut values.fonts),
                "shadow" => match serde_json::from_value(part.clone()) {
                    Ok(layers) => values.shadow = Some(layers),
                    Err(_) => problems.push(ValueProblem::new(
                        built_in::SHADOW_TOKEN,
                        ValueProblemKind::BadShadow,
                    )),
                },
                other => problems.push(ValueProblem::new(other, ValueProblemKind::UnknownToken)),
            }
        }
        problems.extend(values.problems());
        if problems.is_empty() {
            Ok(values.normalized())
        } else {
            Err(problems)
        }
    }
}

/// Reads `{token: value}`; a token whose value is of the wrong shape is a problem that names it.
fn read_map<T: serde::de::DeserializeOwned>(
    part: &serde_json::Value,
    section: &str,
    problems: &mut Vec<ValueProblem>,
    into: &mut BTreeMap<String, T>,
) {
    let Some(entries) = part.as_object() else {
        problems.push(ValueProblem::new(section, ValueProblemKind::BadShape));
        return;
    };
    for (name, entry) in entries {
        match serde_json::from_value(entry.clone()) {
            Ok(value) => {
                into.insert(name.clone(), value);
            }
            Err(_) => problems.push(ValueProblem::new(name, ValueProblemKind::BadShape)),
        }
    }
}

fn shadow_is_allowed(layers: &[ShadowLayer]) -> bool {
    let in_range = |pixels: i32| (-SHADOW_PIXELS_MOST..=SHADOW_PIXELS_MOST).contains(&pixels);
    (1..=SHADOW_LAYERS_MOST).contains(&layers.len())
        && layers.iter().all(|layer| {
            in_range(layer.x)
                && in_range(layer.y)
                && in_range(layer.spread)
                && i32::try_from(layer.blur).is_ok_and(|blur| blur <= SHADOW_PIXELS_MOST)
                && is_hex_color(&layer.color, true)
        })
}
