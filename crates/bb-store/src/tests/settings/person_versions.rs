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
        assert_eq!(version_of(storage, ada, SIZES).await?, 1);
        // The second one is stale, whatever it writes.
        assert!(!put_at(storage, &mut source, own, SIZES, text(PARIS), Some(0)).await?);
        assert_eq!(version_of(storage, ada, SIZES).await?, 1);

        // Both read version 1; the first one wins, the second one is stale.
        assert!(put_at(storage, &mut source, own, SIZES, text(LONDON), Some(1)).await?);
        assert!(!put_at(storage, &mut source, own, SIZES, text(ROME), Some(1)).await?);
        let layers = storage
            .database()
            .setting_layers(&Scopes {
                project: None,
                person: Some(ada),
            })
            .await?;
        assert_eq!(layers.person[SIZES], text(LONDON));
        assert_eq!(layers.person_versions[SIZES], 2);
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
        assert_eq!(version_of(storage, ada, SIZES).await?, 2);
        // A setting of the same person is counted on its own.
        put(storage, &mut source, own, "appearance.mode", text("dark")).await?;
        assert_eq!(version_of(storage, ada, "appearance.mode").await?, 1);
        // The value of another person is another record.
        let ben = create_person(storage, &mut source, "ben@example.test").await?;
        assert_eq!(version_of(storage, ben, SIZES).await?, 0);
        assert!(
            !put_at(storage, &mut source, own, SIZES, text(LONDON), Some(7)).await?,
            "a version that was never there is stale"
        );
        Ok(())
    })
    .await
}
