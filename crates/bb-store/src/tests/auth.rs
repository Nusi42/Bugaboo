//! The sign-in storage on both databases: what the use cases rely on when they write inside the
//! transaction of a change (unique addresses, purging sessions, one-time codes and invitations,
//! the encrypted code app secret).

use bb_app::auth::{AccountStore, InvitationStore, SecondStepStore, SessionStore};
use bb_core::id::IdSource;
use bb_core::sign_in::{SetupCode, TwoStep};
use bb_testing::{TestDatabase, TestResult, each_database};

use super::*;
use crate::test_support::{at, event, ids, open};
use bb_core::id::{InvitationId, SessionId, UserId};
use bb_core::network::NetworkPrefix;
use bb_core::person::Person;
use bb_core::sign_in::{Invitation, SessionStage, SignInSession};

struct Fixture {
    storage: AuthStorage,
    _folder: tempfile::TempDir,
}

async fn fixture(db: &TestDatabase) -> Result<Fixture, Box<dyn std::error::Error + Send + Sync>> {
    let database = open(db).await?;
    let folder = tempfile::tempdir()?;
    std::fs::create_dir_all(folder.path().join("logs"))?;
    let keys = Arc::new(database.open_keys(&folder.path().join("keys")).await?);
    let seal = LogSeal::new(&keys, folder.path().join("logs").join("head.log"));
    Ok(Fixture {
        storage: AuthStorage::new(database, keys, seal),
        _folder: folder,
    })
}

fn person(source: &mut impl IdSource, email: &str) -> Person {
    Person {
        id: UserId::generate(source),
        display_name: "Test Person".to_owned(),
        email: email.to_owned(),
        created_at: at(0),
    }
}

fn session(
    source: &mut impl IdSource,
    person: UserId,
    stage: SessionStage,
    tag: u8,
    started: i64,
    used: i64,
) -> SignInSession {
    SignInSession {
        id: SessionId::generate(source),
        person,
        token_hash: Digest::new([tag; 32]),
        stage,
        created_at: at(started),
        last_seen_at: at(used),
        network: NetworkPrefix::from_stored("203.0.113.0"),
        device: "Firefox|Windows".to_owned(),
    }
}

/// Runs `write` as a change with its event, the way a use case does.
async fn change<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    let event = event(source, 5);
    ChangeStore::record_change(storage, &event, write).await
}

#[tokio::test]
async fn auth_01_one_address_is_one_account_and_the_password_is_read_with_it() -> TestResult {
    each_database("auth_01_unique_email", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ada = person(&mut source, "ada@example.test");
        let first = ada.clone();
        change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::insert_person(tx, &first, "hash-1", at(1)).await })
        })
        .await?;
        let account = storage
            .account_by_email("ada@example.test")
            .await?
            .ok_or("no account")?;
        assert_eq!(account.person, ada);
        assert_eq!(account.password_hash.as_deref(), Some("hash-1"));
        assert!(
            storage
                .account_by_email("nobody@example.test")
                .await?
                .is_none()
        );

        let twin = person(&mut source, "ada@example.test");
        let result = change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::insert_person(tx, &twin, "hash-2", at(2)).await })
        })
        .await;
        assert!(AuthStorage::is_duplicate(
            &result.err().ok_or("the twin was accepted")?
        ));
        // Nothing of the failed change stays: not the person, not their password.
        let kept = storage
            .account_by_email("ada@example.test")
            .await?
            .ok_or("no account")?;
        assert_eq!(kept.person, ada);
        assert_eq!(kept.password_hash.as_deref(), Some("hash-1"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_only_sessions_nobody_can_use_are_purged() -> TestResult {
    each_database("sec_12_purge_sessions", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ada = person(&mut source, "ada@example.test");
        let id = ada.id;
        let sessions = vec![
            session(&mut source, id, SessionStage::Active, 1, 900, 990), // fresh
            session(&mut source, id, SessionStage::Active, 2, 900, 100), // idle too long
            session(&mut source, id, SessionStage::Active, 3, 10, 990),  // too old
            session(
                &mut source,
                id,
                SessionStage::SecondStepPending,
                4,
                100,
                100,
            ), // stale
            session(
                &mut source,
                id,
                SessionStage::SecondStepPending,
                5,
                990,
                990,
            ), // fresh
        ];
        change(&storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::insert_person(tx, &ada, "hash", at(1)).await?;
                for session in &sessions {
                    AuthStorage::insert_session(tx, session).await?;
                }
                Ok(())
            })
        })
        .await?;
        let purged = change(&storage, &mut source, |tx| {
            Box::pin(
                async move { AuthStorage::purge_sessions(tx, at(500), at(200), at(500)).await },
            )
        })
        .await?;
        assert_eq!(purged, 3);
        let left: Vec<u8> = storage
            .sessions_of(id)
            .await?
            .iter()
            .map(|session| session.token_hash.as_bytes()[0])
            .collect();
        assert_eq!(left.len(), 2);
        assert!(left.contains(&1) && left.contains(&5));
        // A session is found by its hash with its person, and only by the hash.
        let found = storage
            .session_by_hash(&Digest::new([1; 32]))
            .await?
            .ok_or("no session")?;
        assert_eq!(found.person.id, id);
        assert!(
            storage
                .session_by_hash(&Digest::new([9; 32]))
                .await?
                .is_none()
        );

        // Ending all sessions of a person can keep one.
        let keep = found.session.id;
        let ended = change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_sessions_of(tx, id, Some(keep)).await })
        })
        .await?;
        assert_eq!(ended, 1);
        assert_eq!(storage.sessions_of(id).await?.len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn auth_03_a_code_step_is_accepted_once_even_when_requests_race() -> TestResult {
    each_database("auth_03_step_race", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ada = person(&mut source, "ada@example.test");
        let id = ada.id;
        let step = TwoStep {
            person: id,
            confirmed: true,
            last_step: 0,
            created_at: at(0),
        };
        change(&storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::insert_person(tx, &ada, "hash", at(1)).await?;
                AuthStorage::upsert_two_step(tx, &step).await
            })
        })
        .await?;
        let mut tasks = Vec::new();
        for number in 0..6_i64 {
            let storage = storage.clone();
            let event = event(&mut source, 10 + number);
            tasks.push(tokio::spawn(async move {
                ChangeStore::record_change(&storage, &event, move |tx| {
                    Box::pin(async move { AuthStorage::accept_totp_step(tx, id, 7).await })
                })
                .await
            }));
        }
        let mut accepted = 0;
        for task in tasks {
            accepted += usize::from(task.await??);
        }
        assert_eq!(accepted, 1, "the same step is accepted by one request only");
        // An older step is never accepted, a newer one is.
        let older = change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::accept_totp_step(tx, id, 6).await })
        })
        .await?;
        let newer = change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::accept_totp_step(tx, id, 8).await })
        })
        .await?;
        assert!(!older && newer);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_03_a_backup_code_and_an_invitation_work_once() -> TestResult {
    each_database("auth_03_use_once", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ada = person(&mut source, "ada@example.test");
        let id = ada.id;
        let codes = [Digest::new([1; 32]), Digest::new([2; 32])];
        let invitation = Invitation {
            id: InvitationId::generate(&mut source),
            token_hash: Digest::new([3; 32]),
            email: "bob@example.test".to_owned(),
            invited_by: id,
            created_at: at(0),
            expires_at: at(100),
            used_at: None,
        };
        let stored_invitation = invitation.clone();
        change(&storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::insert_person(tx, &ada, "hash", at(1)).await?;
                AuthStorage::replace_backup_codes(tx, id, &codes).await?;
                AuthStorage::insert_invitation(tx, &stored_invitation).await
            })
        })
        .await?;
        assert_eq!(storage.backup_codes_left(id).await?, 2);
        let first = change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::use_backup_code(tx, id, &codes[0], at(5)).await })
        })
        .await?;
        let again = change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::use_backup_code(tx, id, &codes[0], at(6)).await })
        })
        .await?;
        let unknown = change(&storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::use_backup_code(tx, id, &Digest::new([9; 32]), at(6)).await
            })
        })
        .await?;
        assert!(first && !again && !unknown);
        assert_eq!(storage.backup_codes_left(id).await?, 1);

        // An invitation: not after it ran out, and once before.
        let invitation_id = invitation.id;
        let late = change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::use_invitation(tx, invitation_id, at(100)).await })
        })
        .await?;
        let used = change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::use_invitation(tx, invitation_id, at(50)).await })
        })
        .await?;
        let twice = change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::use_invitation(tx, invitation_id, at(51)).await })
        })
        .await?;
        assert!(!late && used && !twice);
        let found = storage
            .invitation_by_hash(&invitation.token_hash)
            .await?
            .ok_or("no invitation")?;
        assert_eq!(found.used_at, Some(at(50)));
        assert!(storage.open_invitations(at(10), None, 10).await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_09_a_new_setup_code_replaces_the_old_and_is_taken_once() -> TestResult {
    each_database("auth_09_setup_code_store", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        assert!(storage.setup_code().await?.is_none());
        for seed in [1_u8, 2] {
            let code = SetupCode {
                code_hash: Digest::new([seed; 32]),
                created_at: at(i64::from(seed)),
            };
            change(&storage, &mut source, move |tx| {
                Box::pin(async move { AuthStorage::put_setup_code(tx, &code).await })
            })
            .await?;
        }
        let waiting = storage.setup_code().await?.ok_or("no setup code")?;
        assert_eq!(waiting.code_hash, Digest::new([2; 32]));
        // The hash of an older code does not use up the newer one.
        let old = change(&storage, &mut source, |tx| {
            Box::pin(async move { AuthStorage::take_setup_code(tx, &Digest::new([1; 32])).await })
        })
        .await?;
        assert!(!old);
        let first = change(&storage, &mut source, |tx| {
            Box::pin(async move { AuthStorage::take_setup_code(tx, &Digest::new([2; 32])).await })
        })
        .await?;
        let second = change(&storage, &mut source, |tx| {
            Box::pin(async move { AuthStorage::take_setup_code(tx, &Digest::new([2; 32])).await })
        })
        .await?;
        assert!(first && !second);
        assert!(storage.setup_code().await?.is_none());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_03_the_code_app_secret_is_stored_encrypted_and_goes_away_with_the_code_app()
-> TestResult {
    each_database("auth_03_secret_store", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ada = person(&mut source, "ada@example.test");
        let id = ada.id;
        change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::insert_person(tx, &ada, "hash", at(1)).await })
        })
        .await?;
        assert!(storage.totp_secret(id).await?.is_none());
        storage
            .put_totp_secret(id, b"12345678901234567890", at(2))
            .await?;
        let secret = storage.totp_secret(id).await?.ok_or("no secret")?;
        assert_eq!(&**secret, b"12345678901234567890");
        let names = storage.database().secret_names().await?;
        assert_eq!(names.len(), 1);
        assert!(names[0].as_str().starts_with("totp."));
        storage.delete_totp_secret(id).await?;
        assert!(storage.totp_secret(id).await?.is_none());
        Ok(())
    })
    .await
}
