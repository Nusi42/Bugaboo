//! The port of the own-theme use cases: what they need from storage.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::id::{ThemeId, UserId};
use bb_core::theme::{OwnTheme, ThemeKind};
use bb_core::version::Version;

use crate::ports::ChangeStore;
use crate::store_macros::{queries, reads, writes};

/// What the own-theme use cases need from storage.
pub trait ThemeStore: ChangeStore + 'static {
    queries! {
        /// The theme with this ID, also one that was never saved.
        fn own_theme(id: ThemeId) -> Option<OwnTheme> = own_theme_in;
        /// How many themes for everyone there are, saved or not: a theme that is only made counts
        /// against the limit too, so a flood of new themes is refused like a flood of saves.
        fn count_themes_for_everyone() -> u64 = count_themes_for_everyone_in;
        /// How many personal themes a person has, saved or not.
        fn count_personal_themes_of(maker: UserId) -> u64 = count_personal_themes_of_in;
    }

    reads! {
        /// The themes listed to a person, by name: the saved themes for everyone, every theme the
        /// person made (also one never saved), and, with `unsaved_for_everyone`, the themes for
        /// everyone that were never saved (for who looks after them).
        fn themes_listed_for(person: UserId, unsaved_for_everyone: bool) -> Vec<OwnTheme>;
    }

    writes! {
        /// Holds the themes of a kind (those for everyone, or the personal ones of `maker`) until
        /// the change ends, so two changes at the same moment cannot both pass a limit.
        fn lock_themes_of(kind: ThemeKind, maker: UserId) -> ();
        /// Adds a theme; it is offered as a choice nowhere until its first save.
        fn insert_own_theme(theme: &OwnTheme) -> ();
        /// Writes a save: name and values, counts the version up, sets the first save moment, and
        /// says whether the theme was still at `version`.
        fn save_own_theme(theme: &OwnTheme, version: Version) -> bool;
        /// Takes a theme away, and says whether it was still at `version`.
        fn delete_own_theme(id: ThemeId, version: Version) -> bool;
    }
}
