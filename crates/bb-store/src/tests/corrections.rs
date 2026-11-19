//! The corrections of a person's name and address on both databases (PRIV-06): the name with its
//! key, an address another account has refused, and the change of an address that waits for its
//! link, which is used once, replaced by a newer one, dropped, and deleted once it ran out.

use bb_app::auth::AccountStore;
use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::id::{SequentialIds, UserId};
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, event, fixture, ids, person};

async fn write<T: Send>(
    storage: &AuthStorage,
    source: &mut SequentialIds,
    change: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    let event = event(source, 1);
    ChangeStore::record_change(storage, &event, change).await
}

async fn stored_person(
    storage: &AuthStorage,
    source: &mut SequentialIds,
    email: &str,
) -> Result<UserId, StoreError> {
    let who = person(source, email);
    let id = who.id;
    write(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::insert_person(tx, &who, "hash", at(1)).await })
    })
    .await?;
    Ok(id)
}

/// Uses up the link whose secret has the digest `[tag; 32]`.
async fn take(
    storage: &AuthStorage,
    source: &mut SequentialIds,
    tag: u8,
) -> Result<bool, StoreError> {
    write(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::take_e_mail_change(tx, &Digest::new([tag; 32])).await })
    })
    .await
}

fn change(person: UserId, tag: u8, email: &str, expires: i64) -> EmailChange {
    EmailChange {
        person,
        token_digest: Digest::new([tag; 32]),
        new_email: email.to_owned(),
        asked_at: at(1),
        expires_at: at(expires),
    }
}

#[tokio::test]
async fn priv_06_a_name_and_an_address_are_corrected_and_a_taken_address_is_refused() -> TestResult
{
    each_database("priv_06_corrections", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let bob = stored_person(&storage, &mut source, "bob@example.test").await?;
        stored_person(&storage, &mut source, "carol@example.test").await?;
        write(&storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::set_person_name(tx, bob, "Robert Baker").await?;
                AuthStorage::set_person_email(tx, bob, "robert@example.test").await
            })
        })
        .await?;
        let found = storage.person(bob).await?.ok_or("no person")?;
        assert_eq!(found.display_name, "Robert Baker");
        assert_eq!(found.email, "robert@example.test");
        assert!(
            storage
                .account_by_email("robert@example.test")
                .await?
                .is_some()
        );
        // An address of another account is a key used twice.
        let taken = write(&storage, &mut source, move |tx| {
            Box::pin(
                async move { AuthStorage::set_person_email(tx, bob, "carol@example.test").await },
            )
        })
        .await;
        assert!(
            taken
                .as_ref()
                .is_err_and(<AuthStorage as bb_app::auth::AuthStore>::is_duplicate)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_06_a_waiting_change_is_used_once_replaced_and_deleted_when_it_ran_out() -> TestResult
{
    each_database("priv_06_e_mail_changes", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let bob = stored_person(&storage, &mut source, "bob@example.test").await?;
        let carol = stored_person(&storage, &mut source, "carol@example.test").await?;
        let first = change(bob, 1, "bob.new@example.test", 100);
        let put = first.clone();
        write(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::put_e_mail_change(tx, &put).await })
        })
        .await?;
        let found = storage
            .e_mail_change_by_digest(&Digest::new([1; 32]))
            .await?;
        assert_eq!(found, Some(first));
        // A newer change of the same person replaces the one that waited.
        let newer = change(bob, 2, "bob.other@example.test", 100);
        let others = change(carol, 3, "carol.new@example.test", 50);
        write(&storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::put_e_mail_change(tx, &newer).await?;
                AuthStorage::put_e_mail_change(tx, &others).await
            })
        })
        .await?;
        assert!(
            storage
                .e_mail_change_by_digest(&Digest::new([1; 32]))
                .await?
                .is_none()
        );
        // A link is used once.
        assert!(take(&storage, &mut source, 2).await?);
        assert!(!take(&storage, &mut source, 2).await?);
        // What ran out is deleted, what still works stays.
        let works = change(bob, 4, "bob.last@example.test", 100);
        let put = works.clone();
        write(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::put_e_mail_change(tx, &put).await })
        })
        .await?;
        let deleted = write(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_old_e_mail_changes(tx, at(50), 10).await })
        })
        .await?;
        assert_eq!(deleted, 1);
        assert!(
            storage
                .e_mail_change_by_digest(&Digest::new([3; 32]))
                .await?
                .is_none()
        );
        assert_eq!(
            storage
                .e_mail_change_by_digest(&Digest::new([4; 32]))
                .await?,
            Some(works)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_06_dropping_a_persons_waiting_change_leaves_the_others() -> TestResult {
    each_database("priv_06_drop_e_mail_change", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let bob = stored_person(&storage, &mut source, "bob@example.test").await?;
        let carol = stored_person(&storage, &mut source, "carol@example.test").await?;
        let (bobs, carols) = (
            change(bob, 1, "bob.new@example.test", 100),
            change(carol, 2, "carol.new@example.test", 100),
        );
        let kept = carols.clone();
        write(&storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::put_e_mail_change(tx, &bobs).await?;
                AuthStorage::put_e_mail_change(tx, &carols).await
            })
        })
        .await?;
        write(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::drop_e_mail_change(tx, bob).await })
        })
        .await?;
        assert!(
            storage
                .e_mail_change_by_digest(&Digest::new([1; 32]))
                .await?
                .is_none()
        );
        assert_eq!(
            storage
                .e_mail_change_by_digest(&Digest::new([2; 32]))
                .await?,
            Some(kept)
        );
        // Dropping when nothing waits is no error.
        write(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::drop_e_mail_change(tx, bob).await })
        })
        .await?;
        Ok(())
    })
    .await
}
