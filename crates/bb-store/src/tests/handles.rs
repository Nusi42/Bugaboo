//! The handles on both databases: one name for one person for good, the one a person gave up stays
//! theirs, and the people without one are found (M3-F70).

use bb_app::auth::AccountStore;
use bb_app::notifications::HandleStore;
use bb_app::ports::ChangeStore;
use bb_core::id::UserId;
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, event, fixture, ids, person};

async fn stored_person(
    storage: &AuthStorage,
    source: &mut bb_core::id::SequentialIds,
    email: &str,
) -> Result<UserId, StoreError> {
    let who = person(source, email);
    let id = who.id;
    let change = event(source, 1);
    ChangeStore::record_change(storage, &change, move |tx| {
        Box::pin(async move { AuthStorage::insert_person(tx, &who, "hash", at(1)).await })
    })
    .await?;
    Ok(id)
}

async fn give(
    storage: &AuthStorage,
    source: &mut bb_core::id::SequentialIds,
    who: UserId,
    handle: &'static str,
) -> Result<(), StoreError> {
    let change = event(source, 2);
    ChangeStore::record_change(storage, &change, move |tx| {
        Box::pin(async move { AuthStorage::put_handle(tx, who, handle, at(2)).await })
    })
    .await
}

/// When the person last changed their handle, as a change sees it.
async fn changed_at(
    storage: &AuthStorage,
    source: &mut bb_core::id::SequentialIds,
    person: UserId,
) -> Result<Option<Timestamp>, StoreError> {
    let change = event(source, 3);
    ChangeStore::record_change(storage, &change, move |tx| {
        Box::pin(async move { AuthStorage::handle_changed_in(tx, person).await })
    })
    .await
}

#[tokio::test]
async fn m3_f70_a_handle_names_one_person_for_good_and_a_former_one_stays_with_them() -> TestResult
{
    each_database("m3_f70_handles", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ana = stored_person(&storage, &mut source, "ana@example.test").await?;
        let ben = stored_person(&storage, &mut source, "ben@example.test").await?;
        assert_eq!(storage.people_without_handle(10).await?.len(), 2);
        give(&storage, &mut source, ana, "ana.brandt").await?;
        give(&storage, &mut source, ben, "ben.baker").await?;
        assert!(storage.people_without_handle(10).await?.is_empty());
        assert_eq!(
            storage.handle_of_person(ana).await?.as_deref(),
            Some("ana.brandt")
        );
        assert_eq!(storage.person_with_handle("ana.brandt").await?, Some(ana));

        // Ana changes her handle: the old one is still hers and nobody else can have it.
        give(&storage, &mut source, ana, "ana").await?;
        assert_eq!(storage.handle_of_person(ana).await?.as_deref(), Some("ana"));
        assert_eq!(storage.person_with_handle("ana.brandt").await?, Some(ana));
        let taken = give(&storage, &mut source, ben, "ana.brandt").await;
        assert!(taken.is_err(), "a former handle is not free");
        assert_eq!(
            storage.handle_of_person(ben).await?.as_deref(),
            Some("ben.baker")
        );

        // Only the handle a person has now names them in a new text; the one given up is reserved.
        assert_eq!(storage.person_with_current_handle("ana").await?, Some(ana));
        assert_eq!(
            storage.person_with_current_handle("ana.brandt").await?,
            None
        );
        // The handle the account was given is no change; the second one is.
        assert_eq!(changed_at(&storage, &mut source, ben).await?, None);
        assert_eq!(changed_at(&storage, &mut source, ana).await?, Some(at(2)));

        // She takes the old one back.
        give(&storage, &mut source, ana, "ana.brandt").await?;
        assert_eq!(
            storage.handle_of_person(ana).await?.as_deref(),
            Some("ana.brandt")
        );

        // Only the handles people have now are offered, by their start.
        let offered: Vec<String> = storage
            .handle_choices("a", 10)
            .await?
            .into_iter()
            .map(|(_, handle)| handle)
            .collect();
        assert_eq!(offered, ["ana.brandt"]);
        let everyone = storage.handle_choices("", 10).await?;
        assert_eq!(everyone.len(), 2);
        // The underscore is no wildcard.
        assert!(storage.handle_choices("a_a", 10).await?.is_empty());
        Ok(())
    })
    .await
}
