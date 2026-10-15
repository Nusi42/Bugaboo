//! Counting the version of an issue up, on both databases (ADR-0017): every way of doing it adds
//! one to the version and sets the date the issue changed, and the guards say what they promise.

use bb_app::issues::IssueStore as _;
use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::id::IdSource;
use bb_core::issue_key::IssueCode;
use bb_testing::{TestResult, each_database};
use jiff::Timestamp;

use super::*;
use crate::test_support::{at, event, fixture, ids, world};

async fn changed<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    ChangeStore::record_change(storage, &event(source, 5), write).await
}

async fn version_and_date(
    storage: &AuthStorage,
    issue: IssueId,
) -> Result<(i64, Timestamp), StoreError> {
    let found = storage.issue(issue).await?.expect("the issue is there");
    Ok((found.version, found.updated_at))
}

#[tokio::test]
async fn adr_0017_every_count_up_adds_one_and_sets_the_date() -> TestResult {
    each_database("adr_0017_count_up", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 1).await;
        let issue = world.issues[0].id;
        assert_eq!(version_and_date(storage, issue).await?, (1, at(1)));

        // With the version the person looked at, and then with a stale one.
        let counted = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::count_issue_up(tx, issue, Some(1), at(10)).await })
        })
        .await?;
        assert_eq!(counted, Some(2));
        let stale = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::count_issue_up(tx, issue, Some(1), at(11)).await })
        })
        .await?;
        assert_eq!(stale, None);
        assert_eq!(version_and_date(storage, issue).await?, (2, at(10)));

        // Without a version: whatever it is.
        let counted = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::count_issue_up(tx, issue, None, at(20)).await })
        })
        .await?;
        assert_eq!(counted, Some(3));

        // A new code, the trash and the way out of it count too.
        let code = IssueCode::counting(77);
        changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::set_issue_code(tx, issue, &code, at(30)).await })
        })
        .await?;
        assert_eq!(version_and_date(storage, issue).await?, (4, at(30)));
        for (deleted_at, now, version) in [(Some(at(40)), at(40), 5), (None, at(50), 6)] {
            changed(storage, &mut source, move |tx| {
                Box::pin(
                    async move { AuthStorage::set_issue_deleted(tx, issue, deleted_at, now).await },
                )
            })
            .await?;
            assert_eq!(version_and_date(storage, issue).await?, (version, now));
            if deleted_at.is_some() {
                // An issue in the trash is not counted up by the other kind of change.
                let none = changed(storage, &mut source, move |tx| {
                    Box::pin(
                        async move { AuthStorage::count_issue_up(tx, issue, None, at(45)).await },
                    )
                })
                .await?;
                assert_eq!(none, None);
            }
        }
        Ok(())
    })
    .await
}
