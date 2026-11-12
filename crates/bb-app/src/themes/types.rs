//! What the own-theme use cases take and give back: inputs that change something are named after
//! what they do; inputs that only read end in `Query` (ARCH-EVENTS). Everything given back is
//! wrapped in `Authorized<T>` by the use case.

use bb_core::id::{LockToken, ThemeId};
use bb_core::theme::{OwnTheme, ThemeFile, ThemeKind};
use bb_policy::Masked;

use super::rules::NamedPair;

/// Ask for the own themes the person may use.
pub struct ThemesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// Ask for one own theme.
pub struct ThemeQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The theme.
    pub theme: ThemeId,
}

/// Ask for the style sheet of an own theme, or of its draft.
pub struct ThemeSheetQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The theme.
    pub theme: ThemeId,
}

/// Make a new own theme. It is listed nowhere until its first save.
pub struct CreateTheme {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What it is called.
    pub name: String,
    /// For everyone or personal.
    pub kind: ThemeKind,
    /// The ID of the built-in theme it extends.
    pub extends: String,
}

/// Save the values of an own theme, under the edit lock the person holds.
pub struct SaveTheme {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The theme.
    pub theme: ThemeId,
    /// What it is called now.
    pub name: String,
    /// The values it changes, as JSON in the shape of the export (`colors`, `shadow`, `radii`,
    /// `fonts`): read and checked here, so a value that is not allowed is named.
    pub values: serde_json::Value,
    /// The version the person started from.
    pub version: i64,
    /// The token the edit lock was taken with.
    pub lock_token: LockToken,
}

/// Keep an own theme as a new one (*Save a copy*).
pub struct CopyTheme {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The theme that is copied.
    pub theme: ThemeId,
    /// What the copy is called.
    pub name: String,
    /// For everyone or personal.
    pub kind: ThemeKind,
}

/// Make an own theme from a file in the shape of the export (`name`, `extends`, `values`).
pub struct UploadTheme {
    /// The secret of the session of the request.
    pub session_token: String,
    /// For everyone or personal.
    pub kind: ThemeKind,
    /// The bytes of the file, at most `bb_core::theme::THEME_UPLOAD_MOST`.
    pub file: Vec<u8>,
}

/// Take an own theme away. A choice of it counts as the built-in theme it extended.
pub struct DeleteTheme {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The theme.
    pub theme: ThemeId,
    /// The version the person looked at.
    pub version: i64,
}

/// An own theme, as a person who may use it sees it.
#[derive(Clone, Debug)]
pub struct ThemeView {
    /// The stored theme.
    pub theme: OwnTheme,
    /// Whether the person may change it (and take it away).
    pub may_change: bool,
    /// The value a setting keeps when the person chooses it (*Light theme* or *Dark theme*).
    pub choice: String,
    /// Whether it is a light theme (it extends a light built-in theme).
    pub light: bool,
}

impl Masked for ThemeView {}

/// The own theme a setting names, when the person may use it: what the page needs to show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChosenTheme {
    /// The theme.
    pub id: ThemeId,
    /// The ID of the built-in theme it extends, which the page shows under it.
    pub extends: &'static str,
    /// Its version, in the address of its style sheet.
    pub version: i64,
}

/// The own themes a person may use, by name.
#[derive(Clone, Debug)]
pub struct ThemeList {
    /// The themes for everyone and the person's own personal themes, saved ones only.
    pub themes: Vec<ThemeView>,
}

impl Masked for ThemeList {}

/// An own theme with what it is like to read: what a read and a save answer.
#[derive(Clone, Debug)]
pub struct ThemeDetail {
    /// The theme as it is now.
    pub view: ThemeView,
    /// The pairs of its saved values that are *Hard to read*: the warning of a personal theme (a
    /// theme for everyone with any is never saved).
    pub hard_to_read: Vec<NamedPair>,
}

impl Masked for ThemeDetail {}

/// An own theme as a file, in the shape an upload takes.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(transparent)]
pub struct ThemeExport(pub ThemeFile);

impl Masked for ThemeExport {}

/// A style sheet of an own theme, with the version it was made from.
#[derive(Clone, Debug)]
pub struct ThemeSheet {
    /// The CSS: custom properties and nothing else.
    pub css: String,
    /// The version of the theme, or of its draft: the `v` of the address that stays the same as
    /// long as the sheet does.
    pub version: i64,
}
