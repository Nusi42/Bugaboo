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

async fn claim_mail(
    storage: &AuthStorage,
    source: &mut SequentialIds,
    address: &Digest,
    invitation: InvitationId,
    since: i64,
    now: i64,
) -> Result<bool, StoreError> {
    let change = event(source, 3);
    let address = *address;
    ChangeStore::record_change(storage, &change, move |tx| {
        Box::pin(async move {
            AuthStorage::claim_sign_up_mail(tx, &address, invitation, at(since), at(now)).await
        })
    })
    .await
}

#[tokio::test]
async fn auth_one_e_mail_goes_to_an_address_in_a_period_and_a_second_try_of_the_job_may_send()
-> TestResult {
    each_database("auth_claim_mail", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let (first, second) = (
            InvitationId::generate(&mut source),
            InvitationId::generate(&mut source),
        );
        let eve = Digest::new([1; 32]);
        let zed = Digest::new([2; 32]);
        // The first e-mail to an address goes; the next soon after does not, from another sign-up.
        assert!(claim_mail(&storage, &mut source, &eve, first, 0, 100).await?);
        assert!(!claim_mail(&storage, &mut source, &eve, second, 50, 120).await?);
        // The same sign-up tries again (the mail server was down): it may send.
        assert!(claim_mail(&storage, &mut source, &eve, first, 50, 130).await?);
        // Another address is no concern of it.
        assert!(claim_mail(&storage, &mut source, &zed, second, 50, 130).await?);
        // After the period a new one goes.
        assert!(claim_mail(&storage, &mut source, &eve, second, 700, 800).await?);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_two_jobs_for_one_address_at_the_same_moment_send_one_e_mail() -> TestResult {
    each_database("auth_claim_mail_parallel", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let eve = Digest::new([1; 32]);
        for round in 0_i64..10 {
            let (since, now) = (round * 1000, round * 1000 + 900);
            let (one, two) = (
                InvitationId::generate(&mut source),
                InvitationId::generate(&mut source),
            );
            let (change_one, change_two) = (event(&mut source, 3), event(&mut source, 3));
            let claim_for = |storage: AuthStorage, change, invitation| async move {
                ChangeStore::record_change(&storage, &change, move |tx| {
                    Box::pin(async move {
                        AuthStorage::claim_sign_up_mail(tx, &eve, invitation, at(since), at(now))
                            .await
                    })
                })
                .await
            };
            let (a, b) = tokio::join!(
                claim_for(storage.clone(), change_one, one),
                claim_for(storage.clone(), change_two, two)
            );
            let won = [a?, b?].iter().filter(|won| **won).count();
            assert_eq!(won, 1, "round {round}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn gdpr_the_memory_of_an_e_mail_is_forgotten_after_the_time_it_is_needed() -> TestResult {
    each_database("gdpr_forget_sign_up_mail", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let (old, young) = (Digest::new([1; 32]), Digest::new([2; 32]));
        let (one, two) = (
            InvitationId::generate(&mut source),
            InvitationId::generate(&mut source),
        );
        assert!(claim_mail(&storage, &mut source, &old, one, 0, 100).await?);
        assert!(claim_mail(&storage, &mut source, &young, one, 0, 200).await?);
        // Both are remembered: another e-mail to them waits.
        assert!(!claim_mail(&storage, &mut source, &old, two, 0, 250).await?);
        assert!(!claim_mail(&storage, &mut source, &young, two, 0, 250).await?);
        assert_eq!(storage.sign_up_mail_since(at(0)).await?, 2);
        let change = event(&mut source, 9);
        let gone = ChangeStore::record_change(&storage, &change, move |tx| {
            Box::pin(async move { AuthStorage::forget_old_sign_up_mail(tx, at(150)).await })
        })
        .await?;
        assert_eq!(gone, 1);
        // The read for the cap counts what went since a moment, and what is forgotten no more.
        assert_eq!(storage.sign_up_mail_since(at(0)).await?, 1);
        assert_eq!(storage.sign_up_mail_since(at(200)).await?, 0);
        // The old one is forgotten (an e-mail may go), the young one is not.
        assert!(claim_mail(&storage, &mut source, &old, two, 0, 300).await?);
        assert!(!claim_mail(&storage, &mut source, &young, two, 0, 300).await?);
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
