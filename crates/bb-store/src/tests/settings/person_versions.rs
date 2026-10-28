//! A person's own settings have a version: two writers, one of them stale (M5-F97, ADR-0019).

use super::*;

const SIZES: &str = "region.time-zone";
const BERLIN: &str = "Europe/Berlin";
const PARIS: &str = "Europe/Paris";
const LONDON: &str = "Europe/London";
const ROME: &str = "Europe/Rome";

/// The version of the person's own value of `key`, `0` while there is none.
async fn version_of(storage: &AuthStorage, person: UserId, key: &str) -> Result<i64, StoreError> {
    let layers = storage
        .database()
        .setting_layers(&Scopes {
            project: None,
            person: Some(person),
        })
        .await?;
    Ok(layers.person_versions.get(key).copied().unwrap_or(0))
}

#[tokio::test]
async fn m5_f97_a_stale_writer_is_refused_and_the_other_value_stays() -> TestResult {
    each_database("m5_f97_stale_writer", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let ada = create_person(storage, &mut source, "ada@example.test").await?;
        let own = Scope::Person(ada);
        assert_eq!(version_of(storage, ada, SIZES).await?, 0);

        // Two tabs read the setting while it is not there (version 0); the first one writes.
        assert!(put_at(storage, &mut source, own, SIZES, text(BERLIN), Some(0)).await?);
        let first = version_of(storage, ada, SIZES).await?;
        assert!(first > 0);
        // The second one is stale, whatever it writes.
        assert!(!put_at(storage, &mut source, own, SIZES, text(PARIS), Some(0)).await?);
        assert_eq!(version_of(storage, ada, SIZES).await?, first);

        // Both read that version; the first one wins, the second one is stale.
        assert!(put_at(storage, &mut source, own, SIZES, text(LONDON), Some(first)).await?);
        assert!(!put_at(storage, &mut source, own, SIZES, text(ROME), Some(first)).await?);
        let layers = storage
            .database()
            .setting_layers(&Scopes {
                project: None,
                person: Some(ada),
            })
            .await?;
        assert_eq!(layers.person[SIZES], text(LONDON));
        assert!(layers.person_versions[SIZES] > first);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn m5_f97_without_a_version_the_last_write_wins_and_counts_up() -> TestResult {
    each_database("m5_f97_unversioned", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let ada = create_person(storage, &mut source, "ada@example.test").await?;
        let own = Scope::Person(ada);
        put(storage, &mut source, own, SIZES, text(BERLIN)).await?;
        put(storage, &mut source, own, SIZES, text(PARIS)).await?;
        let sizes = version_of(storage, ada, SIZES).await?;
        assert!(sizes > 1, "the second write counted the version up");
        // One counter serves all the settings of a person: the next write has the next number.
        put(storage, &mut source, own, "appearance.mode", text("dark")).await?;
        assert_eq!(
            version_of(storage, ada, "appearance.mode").await?,
            sizes + 1
        );
        // The value of another person is another record.
        let ben = create_person(storage, &mut source, "ben@example.test").await?;
        assert_eq!(version_of(storage, ben, SIZES).await?, 0);
        assert!(
            !put_at(storage, &mut source, own, SIZES, text(LONDON), Some(700)).await?,
            "a version that was never there is stale"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn m5_f97_a_version_never_goes_back_after_a_reset() -> TestResult {
    each_database("m5_f97_reset_version", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let ada = create_person(storage, &mut source, "ada@example.test").await?;
        let own = Scope::Person(ada);
        // A tab reads the value at its version.
        put(storage, &mut source, own, SIZES, text(BERLIN)).await?;
        let read = version_of(storage, ada, SIZES).await?;
        // Another tab resets the setting and sets it again.
        assert!(clear(storage, &mut source, own, SIZES).await?);
        assert_eq!(version_of(storage, ada, SIZES).await?, 0);
        put(storage, &mut source, own, SIZES, text(PARIS)).await?;
        let again = version_of(storage, ada, SIZES).await?;
        assert!(again > read, "{again} is not after {read}");
        // The first tab writes from the version it read: refused, and the new value stays.
        assert!(!put_at(storage, &mut source, own, SIZES, text(ROME), Some(read)).await?);
        assert_eq!(version_of(storage, ada, SIZES).await?, again);
        // Reset and set again with nothing written in between also never repeats a number.
        assert!(clear(storage, &mut source, own, SIZES).await?);
        assert!(put_at(storage, &mut source, own, SIZES, text(LONDON), Some(0)).await?);
        assert!(version_of(storage, ada, SIZES).await? > again);
        Ok(())
    })
    .await
}
