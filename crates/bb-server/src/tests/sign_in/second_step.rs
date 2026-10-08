//! Two-step sign-in with a code app and backup codes (AUTH-03).

use bb_app::auth::SecondStepStore as _;
use bb_app::auth::{
    AppError, AuthApi as _, AuthenticateQuery, ConfirmCodeApp, PasskeySecondStep, RemoveCodeApp,
    RenewBackupCodes, SecondStep, SessionsQuery, StartCodeApp, Started, TwoStepQuery,
};
use bb_app::context::Clock as _;
use bb_auth::TotpSecret;
use bb_core::ErrorCode;
use bb_core::id::UserId;
use bb_core::sign_in::SecurityAction;
use bb_testing::{Failure, TestResult, each_database};

use super::http::ADA_EMAIL;
use super::rig::{PASSWORD, Rig, Tries, refused};

/// What the code app of the person shows right now.
async fn code_now(rig: &Rig, person: UserId) -> Result<String, Failure> {
    let secret = rig
        .storage
        .totp_secret(person)
        .await?
        .ok_or("no code app secret")?;
    Ok(TotpSecret::from_bytes(&secret).code_at(rig.clock.now())?)
}

/// Sets the code app up for the person behind `session`, and returns the backup codes.
async fn set_up(rig: &Rig, person: UserId, session: &str) -> Result<Vec<String>, Failure> {
    rig.api()
        .start_code_app(
            Rig::info(),
            StartCodeApp {
                session_token: session.to_owned(),
                password: PASSWORD.to_owned(),
            },
        )
        .await?;
    let codes = rig
        .api()
        .confirm_code_app(
            Rig::info(),
            ConfirmCodeApp {
                session_token: session.to_owned(),
                code: code_now(rig, person).await?,
                password: PASSWORD.to_owned(),
            },
        )
        .await?;
    Ok(codes.0)
}

async fn second_step(rig: &Rig, pending: &Started, code: &str) -> Result<Started, AppError> {
    rig.api()
        .second_step(
            Rig::info(),
            SecondStep {
                session_token: pending.session_token.clone(),
                code: code.to_owned(),
            },
        )
        .await
}

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
async fn auth_03_the_code_app_is_set_up_with_a_qr_picture_and_asked_for_once_confirmed()
-> TestResult {
    each_database("auth_03_set_up", async |db| {
        let rig = Rig::start(db).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        let person = admin.person.id;
        let setup = rig
            .api()
            .start_code_app(
                Rig::info(),
                StartCodeApp {
                    session_token: admin.session_token.clone(),
                    password: PASSWORD.to_owned(),
                },
            )
            .await?;
        assert!(setup.otpauth_url.starts_with("otpauth://totp/"));
        assert!(setup.otpauth_url.contains("issuer=Bugaboo"));
        assert!(!setup.secret.is_empty() && !setup.qr_png_base64.is_empty());
        // The secret is stored encrypted; the database never holds it as it is.
        let stored = rig.storage.totp_secret(person).await?.ok_or("no secret")?;
        assert_eq!(stored.len(), 20);

        // Until the first code is confirmed, nothing asks for it.
        let status = |token: &str| {
            rig.api().two_step(
                Rig::info(),
                TwoStepQuery {
                    session_token: token.to_owned(),
                },
            )
        };
        assert!(!status(&admin.session_token).await?.code_app);
        assert!(
            rig.sign_in(ADA_EMAIL, PASSWORD)
                .await?
                .second_step_needed
                .is_none()
        );

        // A wrong code does not confirm it; the right one does and makes the backup codes.
        let wrong = rig
            .api()
            .confirm_code_app(
                Rig::info(),
                ConfirmCodeApp {
                    session_token: admin.session_token.clone(),
                    code: "000000".to_owned(),
                    password: PASSWORD.to_owned(),
                },
            )
            .await;
        assert_eq!(refused(wrong), Some(ErrorCode::WrongSecondStepCode));
        let codes = rig
            .api()
            .confirm_code_app(
                Rig::info(),
                ConfirmCodeApp {
                    session_token: admin.session_token.clone(),
                    code: code_now(&rig, person).await?,
                    password: PASSWORD.to_owned(),
                },
            )
            .await?;
        assert_eq!(codes.0.len(), 10);
        assert!(
            codes
                .0
                .iter()
                .all(|code| code.len() == 19 && code.matches('-').count() == 3)
        );
        let distinct: std::collections::HashSet<_> = codes.0.iter().collect();
        assert_eq!(distinct.len(), 10);
        let now = status(&admin.session_token).await?;
        assert!(now.code_app);
        assert_eq!(now.backup_codes_left, 10);

        // A code app that is on cannot be set up again before it is switched off.
        let again = rig
            .api()
            .start_code_app(
                Rig::info(),
                StartCodeApp {
                    session_token: admin.session_token.clone(),
                    password: PASSWORD.to_owned(),
                },
            )
            .await;
        assert_eq!(refused(again), Some(ErrorCode::InvalidRequest));
        assert_eq!(action_count(&rig, SecurityAction::TwoStepEnabled).await?, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_03_signing_in_with_a_code_opens_a_new_session_and_a_code_works_once() -> TestResult {
    each_database("auth_03_code", async |db| {
        let rig = Rig::start(db).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        let person = admin.person.id;
        set_up(&rig, person, &admin.session_token).await?;

        let pending = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        let methods = pending
            .second_step_needed
            .ok_or("a second step is needed")?;
        assert!(methods.code_app && methods.backup_code && !methods.passkey);
        // The password alone opens nothing.
        let query = AuthenticateQuery {
            session_token: pending.session_token.clone(),
        };
        assert_eq!(
            refused(rig.api().authenticate(Rig::info(), query).await),
            Some(ErrorCode::NotSignedIn)
        );
        let sessions = rig
            .api()
            .sessions(
                Rig::info(),
                SessionsQuery {
                    session_token: pending.session_token.clone(),
                },
            )
            .await;
        assert_eq!(refused(sessions), Some(ErrorCode::NotSignedIn));

        // The code that confirmed the setup is used up: a code works once (replay).
        let replayed = second_step(&rig, &pending, &code_now(&rig, person).await?).await;
        assert_eq!(refused(replayed), Some(ErrorCode::WrongSecondStepCode));
        let wrong = second_step(&rig, &pending, "123456").await;
        assert_eq!(refused(wrong), Some(ErrorCode::WrongSecondStepCode));

        // The next code (30 seconds later) opens a session with a new secret.
        rig.clock.advance(30);
        let code = code_now(&rig, person).await?;
        let spaced = format!("{} {}", &code[..3], &code[3..]);
        let opened = second_step(&rig, &pending, &spaced).await?;
        assert!(opened.second_step_needed.is_none());
        assert_ne!(opened.session_token, pending.session_token);
        assert_ne!(opened.csrf_token, pending.csrf_token);
        let me = rig
            .api()
            .authenticate(
                Rig::info(),
                AuthenticateQuery {
                    session_token: opened.session_token.clone(),
                },
            )
            .await?;
        assert_eq!(me.person.id, person);
        // The secret of the password step is gone, and the same code cannot be used again.
        let old = second_step(&rig, &pending, &code).await;
        assert_eq!(refused(old), Some(ErrorCode::NotSignedIn));
        let other = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        let twice = second_step(&rig, &other, &code).await;
        assert_eq!(refused(twice), Some(ErrorCode::WrongSecondStepCode));
        assert!(action_count(&rig, SecurityAction::SecondStepSucceeded).await? >= 1);
        assert!(action_count(&rig, SecurityAction::SecondStepFailed).await? >= 3);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_03_a_backup_code_works_once_and_new_ones_replace_the_old() -> TestResult {
    each_database("auth_03_backup_codes", async |db| {
        let rig = Rig::start(db).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        let person = admin.person.id;
        let codes = set_up(&rig, person, &admin.session_token).await?;

        let pending = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        // Typed in capitals and without hyphens, as a person might.
        let typed = codes[0].to_uppercase().replace('-', " ");
        second_step(&rig, &pending, &typed).await?;
        let again = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        let reused = second_step(&rig, &again, &codes[0]).await;
        assert_eq!(refused(reused), Some(ErrorCode::WrongSecondStepCode));
        let left = rig
            .api()
            .two_step(
                Rig::info(),
                TwoStepQuery {
                    session_token: pending_session(&rig).await?,
                },
            )
            .await?;
        assert_eq!(left.backup_codes_left, 9);
        assert_eq!(action_count(&rig, SecurityAction::BackupCodeUsed).await?, 1);

        // New codes: the password is asked again, and the old codes stop working.
        let session = pending_session(&rig).await?;
        let renew = |password: &str| {
            rig.api().renew_backup_codes(
                Rig::info(),
                RenewBackupCodes {
                    session_token: session.clone(),
                    password: password.to_owned(),
                },
            )
        };
        assert_eq!(
            refused(renew("Wrong-Password-1").await),
            Some(ErrorCode::WrongCredentials)
        );
        let fresh = renew(PASSWORD).await?;
        assert_eq!(fresh.0.len(), 10);
        let pending = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        let old = second_step(&rig, &pending, &codes[1]).await;
        assert_eq!(refused(old), Some(ErrorCode::WrongSecondStepCode));
        second_step(&rig, &pending, &fresh.0[1]).await?;
        assert_eq!(
            action_count(&rig, SecurityAction::BackupCodesRenewed).await?,
            1
        );
        Ok(())
    })
    .await
}

/// A fully signed-in session of Ada (password, then a backup code is not needed: the person
/// signs in with the code of the code app one step later).
async fn pending_session(rig: &Rig) -> Result<String, Failure> {
    let person = rig.person_id(ADA_EMAIL).await?;
    rig.clock.advance(30);
    let pending = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
    let opened = second_step(rig, &pending, &code_now(rig, person).await?).await?;
    Ok(opened.session_token)
}

#[tokio::test]
async fn auth_03_the_code_app_is_switched_off_with_the_password() -> TestResult {
    each_database("auth_03_switch_off", async |db| {
        let rig = Rig::start(db).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        let person = admin.person.id;
        set_up(&rig, person, &admin.session_token).await?;
        let remove = |password: &str| {
            rig.api().remove_code_app(
                Rig::info(),
                RemoveCodeApp {
                    session_token: admin.session_token.clone(),
                    password: password.to_owned(),
                },
            )
        };
        assert_eq!(
            refused(remove("Wrong-Password-1").await),
            Some(ErrorCode::WrongCredentials)
        );
        assert!(rig.storage.two_step(person).await?.is_some());
        remove(PASSWORD).await?;
        assert!(rig.storage.two_step(person).await?.is_none());
        assert!(rig.storage.totp_secret(person).await?.is_none());
        assert_eq!(rig.storage.backup_codes_left(person).await?, 0);
        assert!(
            rig.sign_in(ADA_EMAIL, PASSWORD)
                .await?
                .second_step_needed
                .is_none()
        );
        assert_eq!(
            action_count(&rig, SecurityAction::TwoStepDisabled).await?,
            1
        );
        // Without a code app there is nothing to renew.
        let renew = rig
            .api()
            .renew_backup_codes(
                Rig::info(),
                RenewBackupCodes {
                    session_token: admin.session_token.clone(),
                    password: PASSWORD.to_owned(),
                },
            )
            .await;
        assert_eq!(refused(renew), Some(ErrorCode::InvalidRequest));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_wrong_second_step_codes_are_limited_per_person() -> TestResult {
    each_database("sec_14_second_step_limit", async |db| {
        let rig = Rig::start_with(db, Tries::Few).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        let person = admin.person.id;
        set_up(&rig, person, &admin.session_token).await?;
        rig.clock.advance(30);
        let pending = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        for _ in 0..3 {
            let result = second_step(&rig, &pending, "000000").await;
            assert_eq!(refused(result), Some(ErrorCode::WrongSecondStepCode));
        }
        // Three failures: guessing the six digits now has to wait, and so does the right code.
        let blocked = second_step(&rig, &pending, &code_now(&rig, person).await?).await;
        assert!(matches!(blocked, Err(AppError::Limited { retry_after }) if retry_after > 0));
        assert_eq!(action_count(&rig, SecurityAction::SignInLimited).await?, 1);
        Ok(())
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sec_14_parallel_wrong_codes_at_one_half_finished_sign_in_get_no_more_than_the_allowance()
-> TestResult {
    each_database("sec_14_parallel_second_step", async |db| {
        let rig = Rig::start_with(db, Tries::Few).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        set_up(&rig, admin.person.id, &admin.session_token).await?;
        rig.clock.advance(30);
        let pending = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        let mut tasks = Vec::new();
        for _ in 0..20 {
            let api = rig.services.clone();
            let token = pending.session_token.clone();
            tasks.push(tokio::spawn(async move {
                api.second_step(
                    Rig::info(),
                    SecondStep {
                        session_token: token,
                        code: "000000".to_owned(),
                    },
                )
                .await
            }));
        }
        let (mut wrong, mut limited) = (0, 0);
        for task in tasks {
            match task.await? {
                Err(AppError::Refused(ErrorCode::WrongSecondStepCode)) => wrong += 1,
                Err(AppError::Limited { .. }) => limited += 1,
                other => unreachable!("an answer nobody expected: {other:?}"),
            }
        }
        assert_eq!((wrong, limited), (3, 17));
        Ok(())
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sec_14_parallel_wrong_passwords_before_a_sensitive_change_get_no_more_than_the_allowance()
-> TestResult {
    each_database("sec_14_parallel_password_again", async |db| {
        let rig = Rig::start_with(db, Tries::Few).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        let mut tasks = Vec::new();
        for _ in 0..20 {
            let api = rig.services.clone();
            let token = admin.session_token.clone();
            tasks.push(tokio::spawn(async move {
                api.start_code_app(
                    Rig::info(),
                    StartCodeApp {
                        session_token: token,
                        password: "Wrong-Password-1".to_owned(),
                    },
                )
                .await
            }));
        }
        let (mut wrong, mut limited) = (0, 0);
        for task in tasks {
            match task.await? {
                Err(AppError::Refused(ErrorCode::WrongCredentials)) => wrong += 1,
                Err(AppError::Limited { .. }) => limited += 1,
                other => unreachable!("an answer nobody expected: {other:?}"),
            }
        }
        assert_eq!((wrong, limited), (3, 17));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_ten_wrong_codes_end_the_half_finished_sign_in_and_are_written_down() -> TestResult {
    each_database("sec_14_second_step_given_up", async |db| {
        // The window allows many tries here, so only the count of wrong codes ends the sign-in.
        let rig = Rig::start_with(db, Tries::Unreached).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        set_up(&rig, admin.person.id, &admin.session_token).await?;
        rig.clock.advance(30);
        let pending = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        for number in 1..=10 {
            let result = second_step(&rig, &pending, "000000").await;
            assert_eq!(
                refused(result),
                Some(ErrorCode::WrongSecondStepCode),
                "{number}"
            );
        }
        // The sign-in is over: even the right code finds nothing to finish.
        let result = second_step(&rig, &pending, &code_now(&rig, admin.person.id).await?).await;
        assert_eq!(refused(result), Some(ErrorCode::NotSignedIn));
        assert_eq!(
            action_count(&rig, SecurityAction::SecondStepGivenUp).await?,
            1
        );
        // The password starts a new sign-in, whose count starts at zero.
        let again = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        for _ in 0..9 {
            assert!(second_step(&rig, &again, "000000").await.is_err());
        }
        let opened = second_step(&rig, &again, &code_now(&rig, admin.person.id).await?).await;
        assert!(opened.is_ok());
        Ok(())
    })
    .await
}

/// Finishes the setup of the code app with the code it shows now and this password.
async fn confirm_with(
    rig: &Rig,
    person: UserId,
    session: &str,
    password: &str,
) -> Result<(), Failure> {
    rig.api()
        .confirm_code_app(
            Rig::info(),
            ConfirmCodeApp {
                session_token: session.to_owned(),
                code: code_now(rig, person).await?,
                password: password.to_owned(),
            },
        )
        .await?;
    Ok(())
}

#[tokio::test]
async fn sec_12_a_stolen_session_cannot_add_a_code_app_without_the_password() -> TestResult {
    each_database("sec_12_code_app_needs_password", async |db| {
        let rig = Rig::start(db).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        let person = admin.person.id;
        let stolen = admin.session_token.clone();
        // Starting it asks for the password.
        for password in ["", "Wrong-Password-1"] {
            let result = rig
                .api()
                .start_code_app(
                    Rig::info(),
                    StartCodeApp {
                        session_token: stolen.clone(),
                        password: password.to_owned(),
                    },
                )
                .await;
            assert_eq!(refused(result), Some(ErrorCode::WrongCredentials));
        }
        assert!(rig.storage.two_step(person).await?.is_none());
        assert!(rig.storage.totp_secret(person).await?.is_none());
        // The person starts it; a thief with the session, who also sees the secret, still cannot
        // finish it without the password, so no second step is switched on behind their back.
        rig.api()
            .start_code_app(
                Rig::info(),
                StartCodeApp {
                    session_token: stolen.clone(),
                    password: PASSWORD.to_owned(),
                },
            )
            .await?;
        let result = confirm_with(&rig, person, &stolen, "Wrong-Password-1").await;
        assert!(result.is_err());
        assert!(
            !rig.storage
                .two_step(person)
                .await?
                .is_some_and(|step| step.confirmed)
        );
        confirm_with(&rig, person, &stolen, PASSWORD).await?;
        assert!(
            rig.storage
                .two_step(person)
                .await?
                .is_some_and(|step| step.confirmed)
        );
        Ok(())
    })
    .await
}

/// Breaks a table of the database on purpose and puts it right again.
pub(super) async fn broken<T>(
    rig: &Rig,
    table: &str,
    run: impl Future<Output = T>,
) -> Result<T, Failure> {
    let off = format!("{table}_off");
    rig.database.rename_table_for_tests(table, &off).await?;
    let result = run.await;
    rig.database.rename_table_for_tests(&off, table).await?;
    Ok(result)
}

fn is_internal<T>(result: &Result<T, AppError>) -> bool {
    matches!(result, Err(AppError::Internal(_)))
}

#[tokio::test]
async fn sec_14_a_failure_of_the_server_does_not_use_up_the_tries_of_a_person() -> TestResult {
    each_database("sec_14_internal_gives_the_try_back", async |db| {
        let rig = Rig::start_with(db, Tries::Few).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        let person = admin.person.id;
        set_up(&rig, person, &admin.session_token).await?;
        rig.clock.advance(30);
        let pending = rig.sign_in(ADA_EMAIL, PASSWORD).await?;

        // The second step: the code app cannot be read, five times. None of it is a failed try.
        let results = broken(&rig, "two_step", async {
            let mut results = Vec::new();
            for _ in 0..5 {
                results.push(second_step(&rig, &pending, "000000").await);
            }
            results
        })
        .await?;
        assert!(results.iter().all(is_internal), "{results:?}");
        // A passkey as the second step, with the passkeys unreadable, is the same.
        let results = broken(&rig, "passkey", async {
            let mut results = Vec::new();
            for _ in 0..5 {
                results.push(
                    rig.api()
                        .passkey_second_step(
                            Rig::info(),
                            PasskeySecondStep {
                                session_token: pending.session_token.clone(),
                                challenge_id: "x".to_owned(),
                                credential: serde_json::Value::Null,
                            },
                        )
                        .await,
                );
            }
            results
        })
        .await?;
        assert!(results.iter().all(is_internal), "{results:?}");
        // The allowance is untouched: three wrong codes are checked, the fourth waits.
        for _ in 0..3 {
            assert_eq!(
                refused(second_step(&rig, &pending, "000000").await),
                Some(ErrorCode::WrongSecondStepCode)
            );
        }
        let fourth = second_step(&rig, &pending, "000000").await;
        assert!(
            matches!(fourth, Err(AppError::Limited { .. })),
            "{fourth:?}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_a_failure_of_the_server_does_not_use_up_the_tries_at_sign_in_and_before_a_change()
-> TestResult {
    each_database("sec_14_internal_sign_in_and_password_again", async |db| {
        let rig = Rig::start_with(db, Tries::Few).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        // The passwords cannot be read: signing in and typing the password again both fail.
        let results = broken(&rig, "password_login", async {
            let mut results = Vec::new();
            for _ in 0..5 {
                results.push(rig.sign_in(ADA_EMAIL, "Wrong-Password-1").await.map(|_| ()));
                results.push(
                    rig.api()
                        .start_code_app(
                            Rig::info(),
                            StartCodeApp {
                                session_token: admin.session_token.clone(),
                                password: PASSWORD.to_owned(),
                            },
                        )
                        .await
                        .map(|_| ()),
                );
            }
            results
        })
        .await?;
        assert!(results.iter().all(is_internal), "{results:?}");
        for _ in 0..3 {
            assert_eq!(
                refused(rig.sign_in(ADA_EMAIL, "Wrong-Password-1").await),
                Some(ErrorCode::WrongCredentials)
            );
        }
        let fourth = rig.sign_in(ADA_EMAIL, PASSWORD).await;
        assert!(
            matches!(fourth, Err(AppError::Limited { .. })),
            "{fourth:?}"
        );
        // The same for typing the password again: three wrong ones, then the wait.
        for _ in 0..3 {
            let wrong = rig
                .api()
                .start_code_app(
                    Rig::info(),
                    StartCodeApp {
                        session_token: admin.session_token.clone(),
                        password: "Wrong-Password-1".to_owned(),
                    },
                )
                .await;
            assert_eq!(refused(wrong), Some(ErrorCode::WrongCredentials));
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_only_a_finished_second_step_clears_the_wrong_codes_of_a_person() -> TestResult {
    each_database(
        "sec_14_person_key_only_cleared_by_second_step",
        async |db| {
            let rig = Rig::start_with(db, Tries::Few).await?;
            let admin = rig.admin(ADA_EMAIL).await?;
            let person = admin.person.id;
            set_up(&rig, person, &admin.session_token).await?;
            rig.clock.advance(30);
            let pending = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
            for _ in 0..2 {
                assert!(second_step(&rig, &pending, "000000").await.is_err());
            }
            // The person, signed in elsewhere, types the password again and gets it right: that is
            // not a finished second step, so the wrong codes of the other sign-in stay counted.
            let again = rig
                .api()
                .start_code_app(
                    Rig::info(),
                    StartCodeApp {
                        session_token: admin.session_token.clone(),
                        password: PASSWORD.to_owned(),
                    },
                )
                .await;
            assert_eq!(refused(again), Some(ErrorCode::InvalidRequest));
            assert_eq!(
                refused(second_step(&rig, &pending, "000000").await),
                Some(ErrorCode::WrongSecondStepCode)
            );
            let fourth = second_step(&rig, &pending, "000000").await;
            assert!(
                matches!(fourth, Err(AppError::Limited { .. })),
                "{fourth:?}"
            );
            Ok(())
        },
    )
    .await
}
