//! The setup code of the first start (AUTH-09) and signing in with a password (AUTH-01, SEC-14).

use bb_app::access::Permission;
use bb_app::auth::{AccountStore as _, InvitationStore as _};
use bb_app::auth::{
    AppError, AuthApi as _, AuthenticateQuery, CompleteSetup, IssueSetupCode, RequestInfo, SignIn,
    StatusQuery, issue_setup_code,
};
use bb_core::ErrorCode;
use bb_core::sign_in::SecurityAction;
use bb_testing::{Failure, TestResult, each_database};

use super::http::ADA_EMAIL;
use super::rig::{PASSWORD, Rig, Tries, from, refused};

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
async fn auth_09_a_new_setup_code_is_made_at_every_start_and_only_the_newest_works() -> TestResult {
    each_database("auth_09_setup_code_per_start", async |db| {
        let rig = Rig::start(db).await?;
        let first = rig.setup_code.clone();
        let second = issue_setup_code(&rig.services.ctx(Rig::info()), IssueSetupCode)
            .await?
            .ok_or("setup is open, so a code is made")?;
        assert_ne!(first, second.as_str());
        let with = |code: &str| CompleteSetup {
            code: code.to_owned(),
            display_name: "Ada".to_owned(),
            email: ADA_EMAIL.to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
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
async fn auth_09_a_wrong_setup_code_is_refused_counted_and_written_to_the_log() -> TestResult {
    each_database("auth_09_wrong_setup_code", async |db| {
        let rig = Rig::start_with(db, Tries::Few).await?;
        for typed in ["not a code", "0000-0000-0000-0000", ""] {
            let result = rig
                .api()
                .complete_setup(
                    Rig::info(),
                    CompleteSetup {
                        code: typed.to_owned(),
                        display_name: "Ada".to_owned(),
                        email: ADA_EMAIL.to_owned(),
                        password: PASSWORD.to_owned(),
                        previous_session: None,
                    },
                )
                .await;
            assert_eq!(refused(result), Some(ErrorCode::WrongSetupCode), "{typed}");
        }
        // Three wrong tries used up the allowance: even the right code waits (SEC-14).
        let result = rig.admin(ADA_EMAIL).await;
        assert!(matches!(result, Err(AppError::Limited { retry_after }) if retry_after > 0));
        assert_eq!(action_count(&rig, SecurityAction::SetupFailed).await?, 3);
        assert_eq!(action_count(&rig, SecurityAction::SignInLimited).await?, 1);
        assert_eq!(rig.role_managers().await?, 0);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_09_the_setup_code_makes_an_administrator_once_and_setup_is_closed_afterwards()
-> TestResult {
    each_database("auth_09_setup_closes", async |db| {
        let rig = Rig::start(db).await?;
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
                        email: ADA_EMAIL.to_owned(),
                        password: password.to_owned(),
                        previous_session: None,
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
                    email: ADA_EMAIL.to_owned(),
                    password: PASSWORD.to_owned(),
                    previous_session: None,
                },
            )
            .await;
        assert_eq!(refused(invalid), Some(ErrorCode::InvalidRequest));

        let started = rig.admin("Ada@Example.TEST").await?;
        let permissions = rig.permissions(&started.session_token).await?;
        assert!(permissions.contains(&Permission::ManageRoles));
        assert!(permissions.contains(&Permission::DecideRoles));
        assert_eq!(started.person.email, ADA_EMAIL);
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
async fn auth_09_two_requests_with_the_same_setup_code_create_one_administrator() -> TestResult {
    each_database("auth_09_setup_race", async |db| {
        let rig = Rig::start(db).await?;
        let (first, second) = tokio::join!(rig.admin(ADA_EMAIL), rig.admin("eve@example.test"));
        let made = [first.is_ok(), second.is_ok()];
        assert_eq!(made.iter().filter(|made| **made).count(), 1, "{made:?}");
        assert_eq!(rig.role_managers().await?, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_01_an_unknown_address_and_a_wrong_password_get_the_same_answer() -> TestResult {
    each_database("auth_01_same_answer", async |db| {
        let rig = Rig::start(db).await?;
        rig.admin(ADA_EMAIL).await?;
        let unknown = rig.sign_in("nobody@example.test", PASSWORD).await;
        let wrong = rig.sign_in(ADA_EMAIL, "Wrong-Password-12345").await;
        let not_an_address = rig.sign_in("ada", PASSWORD).await;
        let too_long = rig.sign_in(ADA_EMAIL, &"x".repeat(5000)).await;
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
async fn sec_14_only_failed_tries_count_and_a_success_clears_them() -> TestResult {
    each_database("sec_14_failures_only", async |db| {
        let rig = Rig::start_with(db, Tries::Few).await?;
        rig.admin(ADA_EMAIL).await?;
        // Signing in again and again never uses anything up (M1-F27).
        for _ in 0..8 {
            rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        }
        // Two failures, a success, two failures: the success cleared the first two.
        for _ in 0..2 {
            assert!(rig.sign_in(ADA_EMAIL, "Wrong-Password-1").await.is_err());
        }
        rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        for _ in 0..2 {
            assert_eq!(
                refused(rig.sign_in(ADA_EMAIL, "Wrong-Password-1").await),
                Some(ErrorCode::WrongCredentials)
            );
        }
        // The third failure in a row uses up the allowance, and the next try waits, even with
        // the right password.
        assert_eq!(
            refused(rig.sign_in("ADA@example.test", "Wrong-Password-1").await),
            Some(ErrorCode::WrongCredentials)
        );
        let blocked = rig.sign_in(ADA_EMAIL, PASSWORD).await;
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
async fn sec_14_tries_from_many_networks_never_lock_the_person_out() -> TestResult {
    each_database("sec_14_person_not_locked_out", async |db| {
        let rig = Rig::start_with(db, Tries::Few).await?;
        rig.admin(ADA_EMAIL).await?;
        let sign_in = async |info: RequestInfo, password: &str| {
            rig.api()
                .sign_in(
                    info,
                    SignIn {
                        email: ADA_EMAIL.to_owned(),
                        password: password.to_owned(),
                        previous_session: None,
                    },
                )
                .await
        };
        // Somebody tries Ada's address from several networks, each up to its allowance and one more.
        for network in 1..=2 {
            let attacker = format!("198.51.{network}.7");
            for _ in 0..3 {
                assert_eq!(
                    refused(sign_in(from(&attacker)?, "Wrong-Password-1").await),
                    Some(ErrorCode::WrongCredentials)
                );
            }
            let over = sign_in(from(&attacker)?, PASSWORD).await;
            assert!(matches!(over, Err(AppError::Limited { .. })), "{over:?}");
        }
        // The account is under attack, yet Ada with her password from a network of hers gets in:
        // a try waits its turn, it is not refused.
        sign_in(from("203.0.113.9")?, PASSWORD).await?;
        // A new network waits its turn too, and its wrong try is checked as wrong.
        assert_eq!(
            refused(sign_in(from("192.0.2.1")?, "Wrong-Password-1").await),
            Some(ErrorCode::WrongCredentials)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_guesses_from_many_networks_at_once_are_checked_one_at_a_time() -> TestResult {
    each_database("sec_14_guesses_one_at_a_time", async |db| {
        let rig = Rig::start_with(db, Tries::Few).await?;
        rig.admin(ADA_EMAIL).await?;
        let wrong = async |network: u32| {
            let info = from(&format!("198.51.{network}.7"))?;
            let reply = rig
                .api()
                .sign_in(
                    info,
                    SignIn {
                        email: ADA_EMAIL.to_owned(),
                        password: "Wrong-Password-1".to_owned(),
                        previous_session: None,
                    },
                )
                .await;
            Ok::<_, Failure>(refused(reply))
        };
        // The account alone uses up its allowance.
        for network in 0..3 {
            assert_eq!(wrong(network).await?, Some(ErrorCode::WrongCredentials));
        }
        // Six networks guess at the same moment: one a second is checked, so the last waits about
        // five seconds, and every guess is still answered as wrong.
        let started = std::time::Instant::now();
        let guesses = tokio::join!(
            wrong(10),
            wrong(11),
            wrong(12),
            wrong(13),
            wrong(14),
            wrong(15)
        );
        let guesses = [
            guesses.0, guesses.1, guesses.2, guesses.3, guesses.4, guesses.5,
        ];
        for guess in guesses {
            assert_eq!(guess?, Some(ErrorCode::WrongCredentials));
        }
        let took = started.elapsed();
        assert!(took >= std::time::Duration::from_secs(5), "{took:?}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_the_tries_of_a_day_run_out_for_strange_networks_and_the_person_still_gets_in()
-> TestResult {
    each_database("sec_14_day_of_an_account", async |db| {
        let rig = Rig::start_with(db, Tries::FewADay).await?;
        rig.admin(ADA_EMAIL).await?;
        let sign_in = async |info: RequestInfo, password: &str| {
            rig.api()
                .sign_in(
                    info,
                    SignIn {
                        email: ADA_EMAIL.to_owned(),
                        password: password.to_owned(),
                        previous_session: None,
                    },
                )
                .await
        };
        // Ada signs in from her own network once.
        sign_in(from("203.0.113.9")?, PASSWORD).await?;
        // Five networks she never signed in from use up the tries of the day.
        for network in 0..Tries::FewADay.a_day() {
            assert_eq!(
                refused(sign_in(from(&format!("198.51.{network}.7"))?, "Wrong-Password-1").await),
                Some(ErrorCode::WrongCredentials)
            );
        }
        // Then every network she never signed in from is refused, even with her password, ...
        for network in [100, 101] {
            let over = sign_in(from(&format!("198.51.{network}.7"))?, PASSWORD).await;
            assert!(matches!(over, Err(AppError::Limited { .. })), "{over:?}");
        }
        // ... written down once, ...
        assert_eq!(action_count(&rig, SecurityAction::SignInCapped).await?, 1);
        // ... while from her own network she gets in, and is told.
        let started = sign_in(from("203.0.113.9")?, PASSWORD).await?;
        let told = bb_app::notifications::notifications(
            &rig.services.ctx(Rig::info()),
            bb_app::notifications::NotificationsQuery {
                session_token: started.session_token,
                filter: bb_core::notification::NotificationFilter::All,
                cursor: None,
                limit: 20,
            },
        )
        .await?
        .into_value();
        let kinds: Vec<_> = told.items.iter().map(|item| item.kind).collect();
        assert_eq!(
            kinds,
            [bb_core::notification::NotificationKind::SignInCapped]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_01_a_hash_made_with_other_parameters_is_renewed_at_the_next_sign_in() -> TestResult {
    each_database("auth_01_rehash", async |db| {
        use argon2::password_hash::PasswordHasher as _;
        use argon2::{Algorithm, Argon2, Params, Version};
        let rig = Rig::start(db).await?;
        let started = rig.admin(ADA_EMAIL).await?;
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
                run: None,
            }
        };
        let (store, stored) = (&rig.storage, weak.clone());
        bb_app::ports::ChangeStore::record_change(store, &event, move |transaction| {
            Box::pin(async move {
                <bb_store::AuthStorage as bb_app::auth::AccountStore>::set_password(
                    transaction,
                    person,
                    &stored,
                    jiff::Timestamp::now(),
                )
                .await
            })
        })
        .await?;
        let before = rig.storage.account_by_email(ADA_EMAIL).await?;
        assert_eq!(
            before.and_then(|account| account.password_hash),
            Some(weak.clone())
        );

        rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        let after = rig
            .storage
            .account_by_email(ADA_EMAIL)
            .await?
            .and_then(|account| account.password_hash)
            .ok_or("no hash")?;
        assert_ne!(after, weak);
        assert!(after.contains("m=19456,t=2,p=1"), "{after}");
        // The new hash still checks out.
        rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        Ok(())
    })
    .await
}

/// How many of `tries` parallel requests got an answer of a kind, and how many were limited.
fn tally(
    results: Vec<Result<bb_app::auth::Started, AppError>>,
    wrong: ErrorCode,
) -> (usize, usize) {
    let mut refused_with = 0;
    let mut limited = 0;
    for result in results {
        match result {
            Err(AppError::Refused(code)) if code == wrong => refused_with += 1,
            Err(AppError::Limited { .. }) => limited += 1,
            other => unreachable!("an answer nobody expected: {other:?}"),
        }
    }
    (refused_with, limited)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sec_14_parallel_wrong_tries_at_one_account_get_no_more_than_the_allowance() -> TestResult {
    each_database("sec_14_parallel_account", async |db| {
        let rig = Rig::start_with(db, Tries::Few).await?;
        rig.admin(ADA_EMAIL).await?;
        let mut tasks = Vec::new();
        for _ in 0..20 {
            let api = rig.services.clone();
            tasks.push(tokio::spawn(async move {
                api.sign_in(
                    Rig::info(),
                    SignIn {
                        email: ADA_EMAIL.to_owned(),
                        password: "Wrong-Password-1".to_owned(),
                        previous_session: None,
                    },
                )
                .await
            }));
        }
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await?);
        }
        // The tries are reserved before they are checked, so twenty at once are not twenty
        // guesses: three get through to be checked, the rest wait.
        assert_eq!(tally(results, ErrorCode::WrongCredentials), (3, 17));
        Ok(())
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sec_14_a_success_gives_the_try_back_also_when_tries_run_at_the_same_time() -> TestResult {
    each_database("sec_14_parallel_success", async |db| {
        let rig = Rig::start_with(db, Tries::Few).await?;
        rig.admin(ADA_EMAIL).await?;
        // Right passwords at the same time: none of them counts as a failure afterwards.
        let mut tasks = Vec::new();
        for _ in 0..3 {
            let api = rig.services.clone();
            tasks.push(tokio::spawn(async move {
                api.sign_in(
                    Rig::info(),
                    SignIn {
                        email: ADA_EMAIL.to_owned(),
                        password: PASSWORD.to_owned(),
                        previous_session: None,
                    },
                )
                .await
            }));
        }
        for task in tasks {
            task.await??;
        }
        for _ in 0..3 {
            assert_eq!(
                refused(rig.sign_in(ADA_EMAIL, "Wrong-Password-1").await),
                Some(ErrorCode::WrongCredentials)
            );
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_signing_in_again_ends_the_session_the_browser_still_had() -> TestResult {
    each_database("sec_12_previous_session", async |db| {
        let rig = Rig::start(db).await?;
        let first = rig.admin(ADA_EMAIL).await?;
        let second = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        // A browser that still has the cookie of the first one signs in again.
        let third = rig
            .api()
            .sign_in(
                Rig::info(),
                SignIn {
                    email: ADA_EMAIL.to_owned(),
                    password: PASSWORD.to_owned(),
                    previous_session: Some(first.session_token.clone()),
                },
            )
            .await?;
        let signed_in = |token: &str| {
            rig.api().authenticate(
                Rig::info(),
                AuthenticateQuery {
                    session_token: token.to_owned(),
                },
            )
        };
        assert!(signed_in(&first.session_token).await.is_err());
        assert!(signed_in(&second.session_token).await.is_ok());
        assert!(signed_in(&third.session_token).await.is_ok());
        // A cookie that opens nothing, or text that is no secret, changes nothing.
        for previous in ["garbage".to_owned(), first.session_token.clone()] {
            rig.api()
                .sign_in(
                    Rig::info(),
                    SignIn {
                        email: ADA_EMAIL.to_owned(),
                        password: PASSWORD.to_owned(),
                        previous_session: Some(previous),
                    },
                )
                .await?;
        }
        assert!(signed_in(&second.session_token).await.is_ok());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_09_wrong_setup_codes_are_counted_per_network_not_for_everybody() -> TestResult {
    each_database("auth_09_setup_per_network", async |db| {
        let rig = Rig::start_with(db, Tries::Few).await?;
        let setup = |code: String| CompleteSetup {
            code,
            display_name: "Ada".to_owned(),
            email: ADA_EMAIL.to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        };
        // Somebody far away uses up the allowance of their network ...
        for _ in 0..3 {
            let result = rig
                .api()
                .complete_setup(
                    from("198.51.100.7")?,
                    setup("0000-0000-0000-0000".to_owned()),
                )
                .await;
            assert_eq!(refused(result), Some(ErrorCode::WrongSetupCode));
        }
        let blocked = rig
            .api()
            .complete_setup(from("198.51.100.8")?, setup(rig.setup_code.clone()))
            .await;
        assert!(matches!(blocked, Err(AppError::Limited { .. })));
        // ... and the person at the console, on another network, can still set up.
        rig.api()
            .complete_setup(from("203.0.113.5")?, setup(rig.setup_code.clone()))
            .await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_a_failure_of_the_server_does_not_use_up_the_tries_at_setup() -> TestResult {
    each_database("sec_14_internal_setup", async |db| {
        let rig = Rig::start_with(db, Tries::Few).await?;
        let setup = |code: &str| CompleteSetup {
            code: code.to_owned(),
            display_name: "Ada".to_owned(),
            email: ADA_EMAIL.to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        };
        rig.database
            .rename_table_for_tests("setup_code", "setup_code_off")
            .await?;
        for _ in 0..5 {
            let result = rig
                .api()
                .complete_setup(Rig::info(), setup("0000-0000-0000-0000"))
                .await;
            assert!(matches!(result, Err(AppError::Internal(_))), "{result:?}");
        }
        rig.database
            .rename_table_for_tests("setup_code_off", "setup_code")
            .await?;
        for _ in 0..3 {
            let result = rig
                .api()
                .complete_setup(Rig::info(), setup("0000-0000-0000-0000"))
                .await;
            assert_eq!(refused(result), Some(ErrorCode::WrongSetupCode));
        }
        let result = rig
            .api()
            .complete_setup(Rig::info(), setup(&rig.setup_code))
            .await;
        assert!(matches!(result, Err(AppError::Limited { .. })));
        Ok(())
    })
    .await
}
