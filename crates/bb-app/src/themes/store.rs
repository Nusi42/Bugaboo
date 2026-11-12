//! The port of the own-theme use cases: what they need from storage.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::id::{ThemeId, UserId};
use bb_core::theme::{OwnTheme, OwnValues};
use bb_core::version::Version;
use jiff::Timestamp;

use crate::ports::ChangeStore;
use crate::store_macros::{queries, reads, writes};

/// What the own-theme use cases need from storage.
pub trait ThemeStore: ChangeStore + 'static {
    queries! {
        /// The theme with this ID, also one that was never saved (a draft).
        fn own_theme(id: ThemeId) -> Option<OwnTheme> = own_theme_in;
        /// How many themes for everyone are saved.
        fn count_themes_for_everyone() -> u64 = count_themes_for_everyone_in;
        /// How many personal themes a person has saved.
        fn count_personal_themes_of(maker: UserId) -> u64 = count_personal_themes_of_in;
    }

    reads! {
        /// The saved themes a person may use: those for everyone and their own personal ones, by
        /// name. A theme that was never saved is in no list.
        fn themes_usable_by(person: UserId) -> Vec<OwnTheme>;
    }

    writes! {
        /// Adds a theme (a draft, until its first save).
        fn insert_own_theme(theme: &OwnTheme) -> ();
        /// Writes a save: name, built-in theme and values, clears the draft, counts the version up,
        /// sets the first save moment, and says whether the theme was still at `version`.
        fn save_own_theme(theme: &OwnTheme, version: Version) -> bool;
        /// Keeps the unsaved values of the editor, counts the version of the draft up, and says
        /// whether the theme exists.
        fn keep_draft(id: ThemeId, draft: &OwnValues, now: Timestamp) -> bool;
        /// Takes a theme away, and says whether it was still at `version`.
        fn delete_own_theme(id: ThemeId, version: Version) -> bool;
    }
}
