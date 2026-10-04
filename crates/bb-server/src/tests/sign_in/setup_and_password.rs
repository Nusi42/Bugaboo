//! The setup code of the first start (AUTH-09) and signing in with a password (AUTH-01, SEC-14).

use bb_app::auth::{
    AppError, AuthStore as _, AuthenticateQuery, CompleteSetup, IssueSetupCode, StatusQuery,
    issue_setup_code,
};
use bb_core::ErrorCode;
use bb_core::sign_in::SecurityAction;
use bb_testing::each_database;

use super::rig::{Outcome, PASSWORD, Rig, refused};

async fn action_count(rig: &Rig, action: SecurityAction) -> Result<usize, bb_store::StoreError> {
    Ok(rig
        .database
        .security_log_entries(0, 1000)
        .await?
        .iter()
        .filter(|entry| entry.action.as_str() == action.code())
        .count())
}

#[tokio::test]
async fn auth_09_a_new_setup_code_is_made_at_every_start_and_only_the_newest_works() -> Outcome {
    each_database("auth_09_setup_code_per_start", async |db| {
        let rig = Rig::start(db, 10).await?;
        let first = rig.setup_code.clone();
        let second = issue_setup_code(&rig.services.ctx(Rig::info()), IssueSetupCode)
            .await?
            .ok_or("setup is open, so a code is made")?;
        assert_ne!(first, second.as_str());
        let with = |code: &str| CompleteSetup {
            code: code.to_owned(),
            display_name: "Ada".to_owned(),
            email: "ada@example.test".to_owned(),
            password: PASSWORD.to_owned(),
        };
        let old = rig.api().complete_setup(Rig::info(), with(&first)).await;
        assert_eq!(refused(old), Some(ErrorCode::WrongSetupCode));
        rig.api()
            .complete_setup(Rig::info(), with(second.as_str()))
            .await?;
        // Setup is over: the next start makes no code and keeps none.
        let after = issue_setup_code(&rig.services.ctx(Rig::info()), IssueSetupCode).await?;
        assert!(after.is_none());
        assert!(rig.storage.setup_code().await?.is_none());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_09_a_wrong_setup_code_is_refused_counted_and_written_to_the_log() -> Outcome {
    each_database("auth_09_wrong_setup_code", async |db| {
        let rig = Rig::start(db, 3).await?;
        for typed in ["not a code", "0000-0000-0000-0000", ""] {
            let result = rig
                .api()
                .complete_setup(
                    Rig::info(),
                    CompleteSetup {
                        code: typed.to_owned(),
                        display_name: "Ada".to_owned(),
                        email: "ada@example.test".to_owned(),
                        password: PASSWORD.to_owned(),
                    },
                )
                .await;
            assert_eq!(refused(result), Some(ErrorCode::WrongSetupCode), "{typed}");
        }
        // Three wrong tries used up the allowance: even the right code waits (SEC-14).
        let result = rig.admin("ada@example.test").await;
        assert!(matches!(result, Err(AppError::Limited { retry_after }) if retry_after > 0));
        assert_eq!(action_count(&rig, SecurityAction::SetupFailed).await?, 3);
        assert_eq!(action_count(&rig, SecurityAction::SignInLimited).await?, 1);
        assert_eq!(rig.storage.administrators().await?, 0);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_09_the_setup_code_makes_an_administrator_once_and_setup_is_closed_afterwards()
-> Outcome {
    each_database("auth_09_setup_closes", async |db| {
        let rig = Rig::start(db, 10).await?;
        let status = rig.api().status(Rig::info(), StatusQuery).await?;
        assert!(status.setup_needed && !status.sign_up_open && status.passkeys_available);

        // A weak password does not use up the code.
        for (password, code) in [
            ("short", ErrorCode::PasswordTooShort),
            ("123456789012", ErrorCode::PasswordBreached),
        ] {
            let result = rig
                .api()
                .complete_setup(
                    Rig::info(),
                    CompleteSetup {
                        code: rig.setup_code.clone(),
                        display_name: "Ada".to_owned(),
                        email: "ada@example.test".to_owned(),
                        password: password.to_owned(),
                    },
                )
                .await;
            assert_eq!(refused(result), Some(code), "{password}");
        }
        assert!(rig.storage.setup_code().await?.is_some());
        let invalid = rig
            .api()
            .complete_setup(
                Rig::info(),
                CompleteSetup {
                    code: rig.setup_code.clone(),
                    display_name: "  ".to_owned(),
                    email: "ada@example.test".to_owned(),
                    password: PASSWORD.to_owned(),
                },
            )
            .await;
        assert_eq!(refused(invalid), Some(ErrorCode::InvalidRequest));

        let started = rig.admin("Ada@Example.TEST").await?;
        assert!(started.person.administrator);
        assert_eq!(started.person.email, "ada@example.test");
        assert!(started.second_step_needed.is_none());
        let me = rig
            .api()
            .authenticate(
                Rig::info(),
                AuthenticateQuery {
                    session_token: started.session_token.clone(),
                },
            )
            .await?;
        assert_eq!(me.person.id, started.person.id);

        // The code is used up, and setup is closed.
        assert!(rig.storage.setup_code().await?.is_none());
        let again = rig.admin("eve@example.test").await;
        assert_eq!(refused(again), Some(ErrorCode::WrongSetupCode));
        let status = rig.api().status(Rig::info(), StatusQuery).await?;
        assert!(!status.setup_needed);
        assert_eq!(action_count(&rig, SecurityAction::SetupCompleted).await?, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_09_two_requests_with_the_same_setup_code_make_one_administrator() -> Outcome {
    each_database("auth_09_setup_race", async |db| {
        let rig = Rig::start(db, 10).await?;
        let (first, second) =
            tokio::join!(rig.admin("ada@example.test"), rig.admin("eve@example.test"));
        let made = [first.is_ok(), second.is_ok()];
        assert_eq!(made.iter().filter(|made| **made).count(), 1, "{made:?}");
        assert_eq!(rig.storage.administrators().await?, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_01_an_unknown_address_and_a_wrong_password_get_the_same_answer() -> Outcome {
    each_database("auth_01_same_answer", async |db| {
        let rig = Rig::start(db, 10).await?;
        rig.admin("ada@example.test").await?;
        let unknown = rig.sign_in("nobody@example.test", PASSWORD).await;
        let wrong = rig
            .sign_in("ada@example.test", "Wrong-Password-12345")
            .await;
        let not_an_address = rig.sign_in("ada", PASSWORD).await;
        let too_long = rig.sign_in("ada@example.test", &"x".repeat(5000)).await;
        for result in [unknown, wrong, not_an_address, too_long] {
            assert_eq!(
                result.unwrap_err(),
                AppError::Refused(ErrorCode::WrongCredentials)
            );
        }
        // The right password works, whatever the capitals and spaces of the address.
        let started = rig.sign_in("  ADA@example.test ", PASSWORD).await?;
        assert!(started.second_step_needed.is_none());
        assert_eq!(action_count(&rig, SecurityAction::SignInFailed).await?, 4);
        assert_eq!(
            action_count(&rig, SecurityAction::SignInSucceeded).await?,
            1
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_only_failed_tries_count_and_a_success_clears_them() -> Outcome {
    each_database("sec_14_failures_only", async |db| {
        let rig = Rig::start(db, 3).await?;
        rig.admin("ada@example.test").await?;
        // Signing in again and again never uses anything up (M1-F27).
        for _ in 0..8 {
            rig.sign_in("ada@example.test", PASSWORD).await?;
        }
        // Two failures, a success, two failures: the success cleared the first two.
        for _ in 0..2 {
            assert!(
                rig.sign_in("ada@example.test", "Wrong-Password-1")
                    .await
                    .is_err()
            );
        }
        rig.sign_in("ada@example.test", PASSWORD).await?;
        for _ in 0..2 {
            assert_eq!(
                refused(rig.sign_in("ada@example.test", "Wrong-Password-1").await),
                Some(ErrorCode::WrongCredentials)
            );
        }
        // The third failure in a row uses up the allowance, and the next try waits, even with
        // the right password.
        assert_eq!(
            refused(rig.sign_in("ADA@example.test", "Wrong-Password-1").await),
            Some(ErrorCode::WrongCredentials)
        );
        let blocked = rig.sign_in("ada@example.test", PASSWORD).await;
        assert!(matches!(blocked, Err(AppError::Limited { retry_after }) if retry_after > 0));
        // Another account is not affected, and the lock is only the minute, not for good.
        assert_eq!(
            refused(rig.sign_in("bob@example.test", "Wrong-Password-1").await),
            Some(ErrorCode::WrongCredentials)
        );
        assert_eq!(action_count(&rig, SecurityAction::SignInLimited).await?, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_01_a_hash_made_with_other_parameters_is_renewed_at_the_next_sign_in() -> Outcome {
    each_database("auth_01_rehash", async |db| {
        use argon2::password_hash::PasswordHasher as _;
        use argon2::{Algorithm, Argon2, Params, Version};
        let rig = Rig::start(db, 10).await?;
        let started = rig.admin("ada@example.test").await?;
        // An older hash: cheaper parameters, as if the parameters were raised since.
        let weak = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(8, 1, 1, Some(32))?,
        )
        .hash_password_with_salt(PASSWORD.as_bytes(), &[7_u8; 16])?
        .to_string();
        let person = started.person.id;
        let event = {
            use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef};
            use bb_core::id::{EntityId, EventId};
            DomainEvent {
                id: EventId::generate(&mut bb_app::context::SystemIds)?,
                time: jiff::Timestamp::now(),
                actor: Actor::System,
                channel: Channel::System,
                entity: EntityRef {
                    kind: EntityKind::new("person")?,
                    id: EntityId::from(person),
                },
                changes: vec![],
                reason: None,
            }
        };
        let (store, stored) = (&rig.storage, weak.clone());
        bb_app::ports::ChangeStore::record_change(store, &event, move |transaction| {
            Box::pin(async move {
                <bb_store::AuthStorage as bb_app::auth::AuthStore>::set_password(
                    transaction,
                    person,
                    &stored,
                    jiff::Timestamp::now(),
                )
                .await
            })
        })
        .await?;
        let before = rig.storage.account_by_email("ada@example.test").await?;
        assert_eq!(
            before.and_then(|account| account.password_hash),
            Some(weak.clone())
        );

        rig.sign_in("ada@example.test", PASSWORD).await?;
        let after = rig
            .storage
            .account_by_email("ada@example.test")
            .await?
            .and_then(|account| account.password_hash)
            .ok_or("no hash")?;
        assert_ne!(after, weak);
        assert!(after.contains("m=19456,t=2,p=1"), "{after}");
        // The new hash still checks out.
        rig.sign_in("ada@example.test", PASSWORD).await?;
        Ok(())
    })
    .await
}
