//! The own-theme part of the in-memory store (THM-04).

use bb_core::id::{ThemeId, UserId};
use bb_core::theme::{OwnTheme, ThemeKind};
use bb_core::version::Version;

use super::fake::{FakeError, FakeStore, Tx};
use crate::themes::ThemeStore;

/// How many themes of `kind` there are, of `maker` when one is named.
fn count(tx: &Tx, kind: ThemeKind, maker: Option<UserId>) -> u64 {
    let found = tx
        .state
        .own_themes
        .iter()
        .filter(|theme| theme.kind == kind && maker.is_none_or(|maker| theme.maker == maker))
        .count();
    u64::try_from(found).unwrap_or(u64::MAX)
}

impl ThemeStore for FakeStore {
    async fn own_theme_in(tx: &mut Tx, id: ThemeId) -> Result<Option<OwnTheme>, FakeError> {
        Ok(tx
            .state
            .own_themes
            .iter()
            .find(|theme| theme.id == id)
            .cloned())
    }

    async fn count_themes_for_everyone_in(tx: &mut Tx) -> Result<u64, FakeError> {
        Ok(count(tx, ThemeKind::ForEveryone, None))
    }

    async fn count_personal_themes_of_in(tx: &mut Tx, maker: UserId) -> Result<u64, FakeError> {
        Ok(count(tx, ThemeKind::Personal, Some(maker)))
    }

    async fn themes_usable_by(&self, person: UserId) -> Result<Vec<OwnTheme>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<OwnTheme> = state
                .own_themes
                .iter()
                .filter(|theme| {
                    theme.saved_at.is_some()
                        && (theme.kind == ThemeKind::ForEveryone || theme.maker == person)
                })
                .cloned()
                .collect();
            found.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()).then(a.id.cmp(&b.id)));
            found
        }))
    }

    async fn insert_own_theme(tx: &mut Tx, theme: &OwnTheme) -> Result<(), FakeError> {
        tx.state.own_themes.push(theme.clone());
        Ok(())
    }

    async fn save_own_theme(
        tx: &mut Tx,
        theme: &OwnTheme,
        version: Version,
    ) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .own_themes
            .iter_mut()
            .find(|found| found.id == theme.id && found.version == version.get())
        else {
            return Ok(false);
        };
        found.name = theme.name.clone();
        found.values = theme.values.clone();
        found.saved_at = theme.saved_at;
        found.updated_at = theme.updated_at;
        found.version += 1;
        Ok(true)
    }

    async fn delete_own_theme(
        tx: &mut Tx,
        id: ThemeId,
        version: Version,
    ) -> Result<bool, FakeError> {
        let before = tx.state.own_themes.len();
        tx.state
            .own_themes
            .retain(|theme| !(theme.id == id && theme.version == version.get()));
        Ok(tx.state.own_themes.len() != before)
    }
}
