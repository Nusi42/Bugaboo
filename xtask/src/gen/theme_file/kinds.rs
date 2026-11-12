//! What a theme is (light or dark, M6-F13) and what an own theme may do with a token (ADR-0026, update
//! M8): small types of the theme reader.

/// Whether a theme is a light one or a dark one (M6-F13).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ThemeKind {
    Light,
    Dark,
}

impl ThemeKind {
    /// The kinds in the order the themes are listed in (light ones first), which is also the order
    /// of the two themes that start (`defaults light= dark=`).
    pub(crate) const ALL: [Self; 2] = [Self::Light, Self::Dark];

    /// The word of the theme file, of the style sheet's `color-scheme` and of the generated names.
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// The kind a word of the theme file names.
    pub(crate) fn of(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.word() == word)
    }
}

/// What an own theme may do with a token. Colors are always open to it; the other kinds only where
/// the line says so: `own="true"` on a shadow (stored as parts) and on a font (the choice of a stack),
/// `own-min` and `own-max` on a size in pixels (a radius, within those bounds).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Own {
    /// Not open to an own theme.
    No,
    /// Open: a shadow or a font stack.
    Yes,
    /// Open within the bounds, in pixels.
    Pixels { min: u32, max: u32 },
}
