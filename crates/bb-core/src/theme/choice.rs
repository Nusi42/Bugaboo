//! A choice of an own theme in the settings *Light theme* and *Dark theme* (ADR-0026, update M8).
//!
//! The setting keeps the built-in theme the own theme extends together with its ID, so a choice of a
//! theme that was deleted, or that the person may no longer use, still says which built-in theme
//! counts in its place. Only the server reads and writes this text; the browser gets the choice and
//! the address of its style sheet from the server.

use super::{BuiltIn, built_in_theme};
use crate::id::ThemeId;

/// What stands between the built-in theme and the ID of the own theme in a stored choice.
const SEPARATOR: char = '/';

/// A choice of an own theme as a setting keeps it: the built-in theme it extends and its ID.
#[derive(Clone, Copy, Debug)]
pub struct OwnChoice {
    /// The built-in theme the own theme extends: what counts when the theme is gone.
    pub extends: &'static BuiltIn,
    /// The own theme.
    pub theme: ThemeId,
}

impl OwnChoice {
    /// The choice stored as `text`, or `None` when the text is no choice of an own theme (a
    /// built-in theme, or anything else).
    #[must_use]
    pub fn read(text: &str) -> Option<Self> {
        let (extends, theme) = text.split_once(SEPARATOR)?;
        Some(Self {
            extends: built_in_theme(extends)?,
            theme: theme.parse().ok()?,
        })
    }

    /// The text a setting keeps for the own theme `theme` that extends `extends`.
    #[must_use]
    pub fn value(extends: &str, theme: ThemeId) -> String {
        format!("{extends}{SEPARATOR}{theme}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SequentialIds;

    #[test]
    fn thm_04_a_choice_of_an_own_theme_keeps_the_built_in_theme_it_extends() {
        let theme = ThemeId::generate(&mut SequentialIds::default()).expect("an ID");
        let text = OwnChoice::value("dusk", theme);
        let read = OwnChoice::read(&text).expect("a choice of an own theme");
        assert_eq!(read.extends.id, "dusk");
        assert_eq!(read.theme, theme);
        for not_one in [
            "dusk",
            "",
            "/",
            "nowhere/x",
            &format!("nowhere/{theme}"),
            "dusk/x",
        ] {
            assert!(OwnChoice::read(not_one).is_none(), "{not_one}");
        }
    }
}
