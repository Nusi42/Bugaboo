//! Two-step sign-in with a code app and backup codes (AUTH-03).

use bb_app::auth::{
    AppError, AuthStore as _, AuthenticateQuery, ConfirmCodeApp, RemoveCodeApp, RenewBackupCodes,
    SecondStep, SessionsQuery, StartCodeApp, Started, TwoStepQuery,
};
use bb_app::context::Clock as _;
use bb_auth::TotpSecret;
use bb_core::ErrorCode;
use bb_core::id::UserId;
use bb_core::sign_in::SecurityAction;
use bb_testing::each_database;

use super::rig::{Outcome, PASSWORD, Rig, refused};

/// What the code app of the person shows right now.
async fn code_now(
    rig: &Rig,
    person: UserId,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let secret = rig
        .storage
        .totp_secret(person)
        .await?
        .ok_or("no code app secret")?;
    Ok(TotpSecret::from_bytes(&secret).code_at(rig.clock.now())?)
}

/// Sets the code app up for the person behind `session`, and returns the backup codes.
async fn set_up(
    rig: &Rig,
    person: UserId,
    session: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    rig.api()
        .start_code_app(
            Rig::info(),
            StartCodeApp {
                session_token: session.to_owned(),
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
async fn auth_03_the_code_app_is_set_up_with_a_qr_picture_and_asked_for_once_confirmed() -> Outcome
{
    each_database("auth_03_set_up", async |db| {
        let rig = Rig::start(db, 10).await?;
        let admin = rig.admin("ada@example.test").await?;
        let person = admin.person.id;
        let setup = rig
            .api()
            .start_code_app(
                Rig::info(),
                StartCodeApp {
                    session_token: admin.session_token.clone(),
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
            rig.sign_in("ada@example.test", PASSWORD)
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
async fn auth_03_signing_in_with_a_code_opens_a_new_session_and_a_code_works_once() -> Outcome {
    each_database("auth_03_code", async |db| {
        let rig = Rig::start(db, 10).await?;
        let admin = rig.admin("ada@example.test").await?;
        let person = admin.person.id;
        set_up(&rig, person, &admin.session_token).await?;

        let pending = rig.sign_in("ada@example.test", PASSWORD).await?;
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
        let other = rig.sign_in("ada@example.test", PASSWORD).await?;
        let twice = second_step(&rig, &other, &code).await;
        assert_eq!(refused(twice), Some(ErrorCode::WrongSecondStepCode));
        assert!(action_count(&rig, SecurityAction::SecondStepSucceeded).await? >= 1);
        assert!(action_count(&rig, SecurityAction::SecondStepFailed).await? >= 3);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_03_a_backup_code_works_once_and_new_ones_replace_the_old() -> Outcome {
    each_database("auth_03_backup_codes", async |db| {
        let rig = Rig::start(db, 10).await?;
        let admin = rig.admin("ada@example.test").await?;
        let person = admin.person.id;
        let codes = set_up(&rig, person, &admin.session_token).await?;

        let pending = rig.sign_in("ada@example.test", PASSWORD).await?;
        // Typed in capitals and without hyphens, as a person might.
        let typed = codes[0].to_uppercase().replace('-', " ");
        second_step(&rig, &pending, &typed).await?;
        let again = rig.sign_in("ada@example.test", PASSWORD).await?;
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
        let pending = rig.sign_in("ada@example.test", PASSWORD).await?;
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
async fn pending_session(rig: &Rig) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let person = rig.person_id("ada@example.test").await?;
    rig.clock.advance(30);
    let pending = rig.sign_in("ada@example.test", PASSWORD).await?;
    let opened = second_step(rig, &pending, &code_now(rig, person).await?).await?;
    Ok(opened.session_token)
}

#[tokio::test]
async fn auth_03_the_code_app_is_switched_off_with_the_password() -> Outcome {
    each_database("auth_03_switch_off", async |db| {
        let rig = Rig::start(db, 10).await?;
        let admin = rig.admin("ada@example.test").await?;
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
            rig.sign_in("ada@example.test", PASSWORD)
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
async fn sec_14_wrong_second_step_codes_are_limited_per_person() -> Outcome {
    each_database("sec_14_second_step_limit", async |db| {
        let rig = Rig::start(db, 3).await?;
        let admin = rig.admin("ada@example.test").await?;
        let person = admin.person.id;
        set_up(&rig, person, &admin.session_token).await?;
        rig.clock.advance(30);
        let pending = rig.sign_in("ada@example.test", PASSWORD).await?;
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
