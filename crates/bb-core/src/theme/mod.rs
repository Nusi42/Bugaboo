//! Themes of your own (ADR-0026, update M8; THM-03, THM-04): what an own theme may set, how its
//! values are checked, the style sheet that is made from them, and how readable it is.
//!
//! An own theme extends one built-in theme and stores only the values it changes: colors, the
//! shadow as parts, the corner radii and the choice of a text face among the stacks of the base
//! theme. Nothing else, and never CSS text: the style sheet is built from these values alone.
//! Pure: no I/O.

mod built_in;
mod choice;
mod colors;
mod file;
mod readable;
mod record;
mod sheet;
mod values;

pub use built_in::{SHADOW_LAYERS_MOST, SHADOW_PIXELS_MOST};
pub use choice::OwnChoice;
pub use colors::{OwnColors, RatedPair, ShownColor, rate_own, readability, shown_colors};
pub use file::{FileProblem, ThemeFile};
pub use readable::{
    ColorRating, HardColor, NamedPair, ReadabilitySummary, ThemeRead, color_ratings, hard_colors,
    hard_pairs, theme_read,
};
pub use record::{OwnTheme, ThemeKind, ThemeName, ThemeNameError};
pub use sheet::{draft_style_sheet, style_sheet};
pub use values::{ColorChoice, OwnValues, ShadowLayer, ValueProblem, ValueProblemKind};

/// The most characters of the name of a theme.
pub const THEME_NAME_MOST: usize = 60;

/// The most themes for everyone an organization has.
pub const THEMES_FOR_EVERYONE_MOST: usize = 50;

/// The most personal themes one person has.
pub const PERSONAL_THEMES_MOST: usize = 10;

/// The most bytes of an uploaded theme (a JSON file).
pub const THEME_UPLOAD_MOST: usize = 64 * 1024;

/// A built-in theme: its colors and its shadow, as the theme files give them.
#[derive(Clone, Copy, Debug)]
pub struct BuiltIn {
    /// The ID of the theme: the name of its file (`dusk`).
    pub id: &'static str,
    /// Whether the theme is a light one; a dark one is the other kind.
    pub light: bool,
    pub(crate) colors: &'static [ColorValue],
    pub(crate) shadow: &'static str,
}

/// The value of a color token in a built-in theme: a color, or `$other` for a link to another
/// color token, and its value for more contrast where the theme has one of its own.
#[derive(Clone, Copy, Debug)]
pub struct ColorValue {
    /// The name of the token.
    pub name: &'static str,
    /// `#rrggbb`, or `$name` of another color token.
    pub value: &'static str,
    /// The value for a device that asks for more contrast.
    pub more: Option<&'static str>,
}

impl ColorValue {
    /// A color value; the theme generator writes the lists of them.
    #[must_use]
    pub const fn new(name: &'static str, value: &'static str, more: Option<&'static str>) -> Self {
        Self { name, value, more }
    }
}

/// A corner radius an own theme may set.
#[derive(Clone, Copy, Debug)]
pub struct RadiusToken {
    /// The name of the token.
    pub name: &'static str,
    /// The value of the base theme, in pixels.
    pub start_px: u32,
    /// The least an own theme may set.
    pub min_px: u32,
    /// The most an own theme may set.
    pub max_px: u32,
}

/// A text face an own theme may choose the stack of.
#[derive(Clone, Copy, Debug)]
pub struct FontToken {
    /// The name of the token; also the name of the stack it is chosen by.
    pub name: &'static str,
    /// The stack as CSS (a list of faces), written by the base theme file.
    pub stack: &'static str,
}

/// A named number of the base theme: an opacity or a brightness.
#[derive(Clone, Copy, Debug)]
pub struct Number {
    /// The name of the token.
    pub name: &'static str,
    /// Its value.
    pub value: f64,
}

/// Every built-in theme, light ones first.
#[must_use]
pub fn built_in_themes() -> &'static [BuiltIn] {
    built_in::BUILT_IN
}

/// The built-in theme with this ID.
#[must_use]
pub fn built_in_theme(id: &str) -> Option<&'static BuiltIn> {
    built_in::BUILT_IN.iter().find(|theme| theme.id == id)
}

/// The tokens an own theme may set as a color, in the order of the theme file.
pub fn color_tokens() -> impl Iterator<Item = &'static str> {
    built_in::BUILT_IN
        .first()
        .into_iter()
        .flat_map(|theme| theme.colors.iter().map(|color| color.name))
}

/// The corner radii an own theme may set.
#[must_use]
pub fn radius_tokens() -> &'static [RadiusToken] {
    built_in::RADII
}

/// The text faces an own theme may choose a stack for; each stack is named by one of these.
#[must_use]
pub fn font_tokens() -> &'static [FontToken] {
    built_in::FONTS
}

impl BuiltIn {
    /// The color token `name` of this theme, as the theme file gives it.
    #[must_use]
    pub fn color(&self, name: &str) -> Option<&ColorValue> {
        self.colors.iter().find(|color| color.name == name)
    }

    /// The shadow of this theme as the theme file writes it (CSS), for showing what an own shadow
    /// replaces; never read back into a style sheet.
    #[must_use]
    pub const fn shadow(&self) -> &'static str {
        self.shadow
    }
}

#[cfg(test)]
mod tests;
