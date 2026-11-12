//! The stored own theme (THM-04, ADR-0026 update M8): its name, whether it is for everyone or
//! personal, the built-in theme it extends and the values it changes. The unsaved values of the
//! editor are the editor's draft of the theme (ADR-0019), kept like the draft of any other record.

use jiff::Timestamp;

use super::THEME_NAME_MOST;
use super::values::OwnValues;
use crate::id::{ThemeId, UserId};
use crate::personal::{Entity, NotPersonal};
use crate::technical::TechnicalName;

words! {
    /// Whether an own theme is for everyone or personal.
    pub enum ThemeKind {
        /// *For everyone*: every signed-in person of the organization can use it.
        ForEveryone => "for-everyone",
        /// *Personal*: only the person who made it can use it.
        Personal => "personal",
    }
}

words_in_api!(
    ThemeKind,
    "Whether an own theme is for everyone in the organization or personal to the person who made it."
);

/// The name of an own theme: trimmed, not empty, at most [`THEME_NAME_MOST`] characters and without
/// control characters. It names a look, not a person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeName(String);

/// The text is no name of a theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeNameError;

impl ThemeName {
    /// The name `text`, trimmed.
    ///
    /// # Errors
    ///
    /// When it is empty, longer than [`THEME_NAME_MOST`] characters or holds a control character.
    pub fn new(text: &str) -> Result<Self, ThemeNameError> {
        let name = text.trim();
        if name.is_empty()
            || name.chars().count() > THEME_NAME_MOST
            || name.chars().any(char::is_control)
        {
            return Err(ThemeNameError);
        }
        Ok(Self(name.to_owned()))
    }

    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// A theme is named for its look; the one personal fact about it is the person who made it.
impl NotPersonal for ThemeName {}
impl NotPersonal for OwnValues {}

/// An own theme (THM-04).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct OwnTheme {
    /// The ID of the theme, in the address of its style sheet.
    #[not_personal]
    pub id: ThemeId,
    /// Whether it is for everyone or personal.
    #[not_personal]
    pub kind: ThemeKind,
    /// The person of a personal theme, or who made a theme for everyone.
    #[personal(category = "identity", basis = "contract")]
    pub maker: UserId,
    /// What the theme is called.
    #[not_personal]
    pub name: ThemeName,
    /// The built-in theme it extends.
    #[not_personal]
    pub extends: TechnicalName,
    /// The values it changes.
    #[not_personal]
    pub values: OwnValues,
    /// When it was first saved; `None` while it is only a draft, which is listed nowhere.
    #[not_personal]
    pub saved_at: Option<Timestamp>,
    /// When it was made.
    #[not_personal]
    pub created_at: Timestamp,
    /// When it was last changed.
    #[not_personal]
    pub updated_at: Timestamp,
    /// Counts up with every save, for the check that nobody changed it meanwhile (ADR-0019) and
    /// for the address of its style sheet.
    #[not_personal]
    pub version: i64,
}
