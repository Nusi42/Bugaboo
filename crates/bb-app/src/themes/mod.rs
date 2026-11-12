//! Own themes (THM-03, THM-04, ADR-0026 update M8): a theme for everyone or a personal one is
//! stored as the token values it changes, checked on every save, and served as a style sheet.
//! A theme is the look of a person in a browser: only a session reaches it, never an access key or
//! an agent.
//!
//! - `rules.rs`: who may use and change a theme, and the checks of a save (the values, the
//!   built-in theme, the name, the pairs that are *Hard to read*).
//! - `read.rs`: the list, one theme, the file of a download, the style sheets of a theme and of
//!   the draft of its editor.
//! - `write.rs`: make, save (under the edit lock, ADR-0019), *Save a copy*, upload, take away.
//! - `records.rs`: a theme as a record the edit locks and drafts work on.

mod read;
mod records;
mod rules;
mod store;
mod types;
mod write;

pub use bb_core::theme::NamedPair;
pub use read::{download_theme, theme, theme_draft_sheet, theme_sheet, themes};
#[cfg(test)]
pub(crate) use rules::editing_what;
pub(crate) use rules::{ChoiceInForce, known_to};
pub use store::ThemeStore;
pub use types::*;
pub use write::{copy_theme, create_theme, delete_theme, save_theme, upload_theme};
