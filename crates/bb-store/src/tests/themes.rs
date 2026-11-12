//! Own themes on both databases (THM-04): a theme that was never saved is listed nowhere, a
//! personal theme is listed for its maker only, a save counts the version up and clears the draft,
//! the draft of the editor counts its own version up, and a stale version changes nothing.

use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::id::{IdSource, ThemeId, UserId};
use bb_core::technical::TechnicalName;
use bb_core::theme::{ColorChoice, OwnTheme, OwnValues, ThemeKind, ThemeName};
use bb_core::version::{FIRST_VERSION, Version};
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, event, fixture, ids};

async fn changed<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    ChangeStore::record_change(storage, &event(source, 5), write).await
}

fn accent(color: &str) -> OwnValues {
    OwnValues {
        colors: [(
            "accent".to_owned(),
            ColorChoice {
                value: color.to_owned(),
                more: None,
            },
        )]
        .into(),
        ..OwnValues::default()
    }
}

fn theme(source: &mut impl IdSource, kind: ThemeKind, maker: UserId, name: &str) -> OwnTheme {
    OwnTheme {
        id: ThemeId::generate(source).unwrap(),
        kind,
        maker,
        name: ThemeName::new(name).unwrap(),
        extends: TechnicalName::new("dusk").unwrap(),
        values: accent("#aabbcc"),
        draft: None,
        draft_version: 0,
        saved_at: None,
        created_at: at(1),
        updated_at: at(1),
        version: FIRST_VERSION,
    }
}

async fn insert(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    theme: &OwnTheme,
) -> Result<(), StoreError> {
    let stored = theme.clone();
    changed(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::insert_own_theme(tx, &stored).await })
    })
    .await
}

async fn save(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    theme: &OwnTheme,
    version: i64,
) -> Result<bool, StoreError> {
    let stored = theme.clone();
    changed(storage, source, move |tx| {
        Box::pin(
            async move { AuthStorage::save_own_theme(tx, &stored, Version::new(version)).await },
        )
    })
    .await
}

#[tokio::test]
async fn thm_04_a_theme_is_a_draft_until_its_first_save_and_listed_nowhere_before() -> TestResult {
    each_database("thm_04_store_draft_and_save", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let maker = UserId::generate(&mut source);
        let mut made = theme(&mut source, ThemeKind::Personal, maker, "Evening");
        insert(storage, &mut source, &made).await?;
        assert_eq!(storage.own_theme(made.id).await?, Some(made.clone()));
        assert!(storage.themes_usable_by(maker).await?.is_empty());
        assert_eq!(storage.count_personal_themes_of(maker).await?, 0);

        // The editor keeps unsaved values; each change of the draft counts its version up.
        let id = made.id;
        let draft = accent("#112233");
        let kept = draft.clone();
        assert!(
            changed(storage, &mut source, move |tx| {
                Box::pin(async move { AuthStorage::keep_draft(tx, id, &kept, at(2)).await })
            })
            .await?
        );
        let found = storage.own_theme(id).await?.unwrap();
        assert_eq!(found.draft, Some(draft));
        assert_eq!(found.draft_version, 1);
        assert_eq!(found.values, made.values, "the saved values do not move");
        assert!(storage.themes_usable_by(maker).await?.is_empty());

        // Save makes the draft the theme, clears it and counts the version up.
        made.values = accent("#112233");
        made.saved_at = Some(at(3));
        made.updated_at = at(3);
        assert!(save(storage, &mut source, &made, FIRST_VERSION).await?);
        let saved = storage.own_theme(id).await?.unwrap();
        assert_eq!(saved.draft, None);
        assert_eq!(saved.version, FIRST_VERSION + 1);
        assert_eq!(saved.values, made.values);
        assert_eq!(storage.themes_usable_by(maker).await?.len(), 1);
        assert_eq!(storage.count_personal_themes_of(maker).await?, 1);
        // A save from a version that is gone changes nothing.
        assert!(!save(storage, &mut source, &made, FIRST_VERSION).await?);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn thm_04_a_personal_theme_is_listed_for_its_maker_only() -> TestResult {
    each_database("thm_04_store_who_may_use", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let (maker, other) = (UserId::generate(&mut source), UserId::generate(&mut source));
        for (kind, who, name) in [
            (ThemeKind::Personal, maker, "Mine"),
            (ThemeKind::ForEveryone, maker, "Company look"),
            (ThemeKind::Personal, other, "Theirs"),
        ] {
            let mut made = theme(&mut source, kind, who, name);
            insert(storage, &mut source, &made).await?;
            made.saved_at = Some(at(2));
            assert!(save(storage, &mut source, &made, FIRST_VERSION).await?);
        }
        let names = |themes: Vec<OwnTheme>| -> Vec<String> {
            themes
                .iter()
                .map(|found| found.name.as_str().to_owned())
                .collect()
        };
        assert_eq!(
            names(storage.themes_usable_by(maker).await?),
            ["Company look", "Mine"]
        );
        assert_eq!(
            names(storage.themes_usable_by(other).await?),
            ["Company look", "Theirs"]
        );
        assert_eq!(storage.count_themes_for_everyone().await?, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn thm_04_a_theme_is_deleted_from_its_version_only() -> TestResult {
    each_database("thm_04_store_delete", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let maker = UserId::generate(&mut source);
        let made = theme(&mut source, ThemeKind::ForEveryone, maker, "Company look");
        insert(storage, &mut source, &made).await?;
        let id = made.id;
        for (version, deleted) in [(FIRST_VERSION + 1, false), (FIRST_VERSION, true)] {
            let applied = changed(storage, &mut source, move |tx| {
                Box::pin(async move {
                    AuthStorage::delete_own_theme(tx, id, Version::new(version)).await
                })
            })
            .await?;
            assert_eq!(applied, deleted);
            assert_eq!(storage.own_theme(id).await?.is_none(), deleted);
        }
        assert_eq!(storage.own_theme(id).await?, None);
        Ok(())
    })
    .await
}
