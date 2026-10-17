//! Accounts that wait for their address on both databases (M1-F38): the link is used once and only
//! in time, and an account that ran out of time goes with everything that belongs to it, while an
//! account that works is never removed this way.

use bb_app::auth::AccountStore;
use bb_app::notifications::{ConfirmationStore, HandleStore};
use bb_app::ports::ChangeStore;
use bb_core::confirmation::AddressConfirmation;
use bb_core::id::{SequentialIds, UserId};
use bb_core::security_log::Digest;
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, event, fixture, ids, person};

/// A person with a password and the handle `someone`, who waits for a link made from `tag`.
async fn waiting(
    storage: &AuthStorage,
    source: &mut SequentialIds,
    email: &str,
    tag: u8,
    expires: i64,
) -> Result<(UserId, Digest), StoreError> {
    waiting_with_handle(storage, source, email, tag, expires, "someone").await
}

#[tokio::test]
async fn auth_a_link_is_used_once_and_only_in_time() -> TestResult {
    each_database("auth_link_once", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let (eve, hash) = waiting(&storage, &mut source, "eve@example.test", 1, 100).await?;
        assert_eq!(
            storage
                .unconfirmed(eve)
                .await?
                .map(|found| found.token_hash),
            Some(hash)
        );
        assert_eq!(
            storage
                .address_confirmation_by_hash(&hash)
                .await?
                .map(|found| found.person),
            Some(eve)
        );
        // A new link replaces the hash and the time.
        let renewed = Digest::new([2; 32]);
        assert!(storage.renew_confirmation(eve, &renewed, at(200)).await?);
        assert!(storage.address_confirmation_by_hash(&hash).await?.is_none());
        let late = at(201);
        let change = event(&mut source, 5);
        let too_late = ChangeStore::record_change(&storage, &change, move |tx| {
            Box::pin(async move { AuthStorage::use_address_confirmation(tx, &renewed, late).await })
        })
        .await?;
        assert!(!too_late, "the time ran out");
        let in_time = at(150);
        let change = event(&mut source, 6);
        let used = ChangeStore::record_change(&storage, &change, move |tx| {
            Box::pin(
                async move { AuthStorage::use_address_confirmation(tx, &renewed, in_time).await },
            )
        })
        .await?;
        assert!(used);
        assert!(storage.unconfirmed(eve).await?.is_none());
        let change = event(&mut source, 7);
        let again = ChangeStore::record_change(&storage, &change, move |tx| {
            Box::pin(
                async move { AuthStorage::use_address_confirmation(tx, &renewed, in_time).await },
            )
        })
        .await?;
        assert!(!again, "once");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_an_account_that_ran_out_of_time_goes_with_what_belongs_to_it() -> TestResult {
    each_database("auth_stale_account", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let (stale, _) = waiting(&storage, &mut source, "stale@example.test", 1, 100).await?;
        let (fresh, _) =
            waiting_with_handle(&storage, &mut source, "fresh@example.test", 2, 900, "fresh")
                .await?;
        assert_eq!(storage.stale_sign_ups(at(500), 10).await?, [stale]);
        let change = event(&mut source, 9);
        assert!(
            ChangeStore::record_change(&storage, &change, move |tx| {
                Box::pin(async move { AuthStorage::delete_unconfirmed_person(tx, stale).await })
            })
            .await?
        );
        assert!(
            storage
                .account_by_email("stale@example.test")
                .await?
                .is_none()
        );
        assert!(storage.person_with_handle("someone").await?.is_none());
        assert!(storage.stale_sign_ups(at(500), 10).await?.is_empty());
        // The one that still has time, and a person with no confirmation, are never removed here.
        let change = event(&mut source, 10);
        let works = person(&mut source, "works@example.test");
        let works_id = works.id;
        ChangeStore::record_change(&storage, &change, move |tx| {
            Box::pin(async move { AuthStorage::insert_person(tx, &works, "hash", at(1)).await })
        })
        .await?;
        let change = event(&mut source, 11);
        assert!(
            !ChangeStore::record_change(&storage, &change, move |tx| {
                Box::pin(async move { AuthStorage::delete_unconfirmed_person(tx, works_id).await })
            })
            .await?
        );
        assert!(
            storage
                .account_by_email("works@example.test")
                .await?
                .is_some()
        );
        assert!(
            storage
                .account_by_email("fresh@example.test")
                .await?
                .is_some()
        );
        assert!(storage.unconfirmed(fresh).await?.is_some());
        Ok(())
    })
    .await
}

/// The same with a handle of its own, so two waiting accounts do not share one.
async fn waiting_with_handle(
    storage: &AuthStorage,
    source: &mut SequentialIds,
    email: &str,
    tag: u8,
    expires: i64,
    handle: &'static str,
) -> Result<(UserId, Digest), StoreError> {
    let who = person(source, email);
    let id = who.id;
    let hash = Digest::new([tag; 32]);
    let confirmation = AddressConfirmation {
        person: id,
        token_hash: hash,
        created_at: at(1),
        expires_at: at(expires),
    };
    let change = event(source, 1);
    ChangeStore::record_change(storage, &change, move |tx| {
        Box::pin(async move {
            AuthStorage::insert_person(tx, &who, "hash", at(1)).await?;
            AuthStorage::put_handle(tx, id, handle, at(1)).await?;
            AuthStorage::put_address_confirmation(tx, &confirmation).await
        })
    })
    .await?;
    Ok((id, hash))
}
