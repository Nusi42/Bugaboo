//! The links of sign-ups on both databases (M1-F38): a link is found like an invitation, is used
//! once and only in time, and the old finished ones are deleted while the open ones stay.

use bb_app::auth::InvitationStore;
use bb_app::ports::ChangeStore;
use bb_core::id::{InvitationId, SequentialIds};
use bb_core::security_log::Digest;
use bb_core::sign_up_invitation::SignUpInvitation;
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, event, fixture, ids};

fn link(
    source: &mut SequentialIds,
    email: &str,
    tag: u8,
    made: i64,
    expires: i64,
) -> SignUpInvitation {
    SignUpInvitation {
        id: InvitationId::generate(source),
        token_hash: Digest::new([tag; 32]),
        email: email.to_owned(),
        created_at: at(made),
        expires_at: at(expires),
        used_at: None,
    }
}

async fn put(
    storage: &AuthStorage,
    source: &mut SequentialIds,
    link: &SignUpInvitation,
) -> Result<(), StoreError> {
    let change = event(source, 1);
    let link = link.clone();
    ChangeStore::record_change(storage, &change, move |tx| {
        Box::pin(async move { AuthStorage::put_sign_up_invitation(tx, &link).await })
    })
    .await
}

#[tokio::test]
async fn auth_a_link_is_read_as_an_invitation_with_nobody_who_invited() -> TestResult {
    each_database("auth_link_as_invitation", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let eve = link(&mut source, "eve@example.test", 1, 10, 100);
        put(&storage, &mut source, &eve).await?;
        let found = storage
            .invitation_by_hash(&eve.token_hash)
            .await?
            .ok_or("no invitation")?;
        assert_eq!((found.id, found.invited_by), (eve.id, None));
        assert_eq!(found.email, "eve@example.test");
        assert!(
            storage
                .invitation_by_hash(&Digest::new([9; 32]))
                .await?
                .is_none()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_a_link_is_used_once_and_only_in_time() -> TestResult {
    each_database("auth_link_once", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let eve = link(&mut source, "eve@example.test", 1, 10, 100);
        put(&storage, &mut source, &eve).await?;
        let id = eve.id;
        let mut tries = Vec::new();
        for now in [101, 50, 60] {
            let change = event(&mut source, 5);
            tries.push(
                ChangeStore::record_change(&storage, &change, move |tx| {
                    Box::pin(async move { AuthStorage::use_invitation(tx, id, at(now)).await })
                })
                .await?,
            );
        }
        assert_eq!(
            tries,
            [false, true, false],
            "late, in time, then never again"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_the_newest_open_link_of_an_address_is_found_and_a_used_one_is_not() -> TestResult {
    each_database("auth_open_link", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let older = link(&mut source, "eve@example.test", 1, 10, 500);
        let newer = link(&mut source, "eve@example.test", 2, 20, 500);
        let other = link(&mut source, "zed@example.test", 3, 30, 500);
        for each in [&older, &newer, &other] {
            put(&storage, &mut source, each).await?;
        }
        let open = storage.open_sign_up_for("eve@example.test", at(40)).await?;
        assert_eq!(open.map(|found| found.id), Some(newer.id));
        assert!(
            storage
                .open_sign_up_for("eve@example.test", at(501))
                .await?
                .is_none()
        );
        assert!(
            storage
                .open_sign_up_for("nobody@example.test", at(40))
                .await?
                .is_none()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_old_finished_links_are_deleted_and_open_ones_stay() -> TestResult {
    each_database("auth_delete_old_links", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ran_out = link(&mut source, "a@example.test", 1, 10, 50);
        let open = link(&mut source, "b@example.test", 2, 10, 900);
        let young = link(&mut source, "c@example.test", 3, 400, 450);
        for each in [&ran_out, &open, &young] {
            put(&storage, &mut source, each).await?;
        }
        // Made before 300, and used up or run out by 500.
        let change = event(&mut source, 7);
        let gone = ChangeStore::record_change(&storage, &change, move |tx| {
            Box::pin(async move {
                AuthStorage::delete_old_sign_up_invitations(tx, at(300), at(500)).await
            })
        })
        .await?;
        assert_eq!(gone, 1);
        assert!(
            storage
                .invitation_by_hash(&ran_out.token_hash)
                .await?
                .is_none()
        );
        assert!(
            storage
                .invitation_by_hash(&open.token_hash)
                .await?
                .is_some()
        );
        assert!(
            storage
                .invitation_by_hash(&young.token_hash)
                .await?
                .is_some()
        );
        Ok(())
    })
    .await
}
