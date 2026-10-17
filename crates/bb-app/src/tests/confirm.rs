//! Confirming an e-mail address (M1-F38, M4-Q10, ADR-0033 §3): when e-mail is on, an account made by
//! signing up works only after the link in the e-mail, which works for 24 hours.

use bb_core::ErrorCode;
use bb_core::confirmation::HOURS;
use bb_core::settings::SettingValue;
use bb_core::time::SECONDS_PER_DAY;

use super::scenarios::{text, token};
use super::scenarios_notifications::enter_the_mail_server;
use super::world::{Outcome, PASSWORD, PUBLIC_URL, World};
use crate::auth::{
    AppError, ChangeSignUpPolicy, SignIn, SignUp, SignUpPolicy, SignedUp, change_sign_up_policy,
    sign_in, sign_up,
};
use crate::notifications::{ConfirmAddress, confirm_address};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

const EVE: &str = "eve@example.test";

/// A world where people may sign up, and e-mail is on when asked.
async fn world(mail_on: bool) -> Result<World, String> {
    let world = World::new().await?;
    change_sign_up_policy(
        &world.ctx(),
        ChangeSignUpPolicy {
            session_token: token(&world),
            policy: SignUpPolicy {
                enabled: true,
                domains: vec!["example.test".to_owned()],
                version: 0,
            },
        },
    )
    .await
    .map_err(text)?;
    if mail_on {
        enter_the_mail_server(&world).await?;
        change_setting(
            &world.ctx(),
            ChangeSetting {
                session_token: token(&world),
                scope: SettingsScope::Organization,
                key: "mail.enabled".to_owned(),
                value: SettingValue::Bool(true),
            },
        )
        .await
        .map_err(text)?;
    }
    Ok(world)
}

async fn sign_up_as_eve(world: &World) -> Result<SignedUp, AppError> {
    sign_up(
        &world.ctx(),
        SignUp {
            email: EVE.to_owned(),
            display_name: "Eve Evans".to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        },
    )
    .await
}

async fn sign_in_as_eve(world: &World) -> Result<(), AppError> {
    sign_in(
        &world.ctx(),
        SignIn {
            email: EVE.to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        },
    )
    .await
    .map(|_| ())
}

/// Runs the jobs that are due.
async fn run_jobs(world: &World) -> Result<(), String> {
    loop {
        if world.ctx().run_due_jobs().await.map_err(text)? == 0 {
            return Ok(());
        }
    }
}

/// The secret in the link of the last e-mail.
fn secret_in_the_last_e_mail(world: &World) -> Result<String, String> {
    let mails = world.mailer.messages();
    let mail = mails.last().ok_or("no e-mail went")?;
    let (_, secret) = mail
        .text
        .split_once("/confirm-address#")
        .ok_or("no link in the e-mail")?;
    Ok(secret
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned())
}

async fn confirm(world: &World, secret: &str) -> Result<(), AppError> {
    confirm_address(
        &world.ctx(),
        ConfirmAddress {
            secret: secret.to_owned(),
        },
    )
    .await
    .map(|_| ())
}

#[tokio::test]
async fn auth_a_signed_up_account_works_only_after_the_link_when_mail_is_on() -> Outcome {
    let world = world(true).await?;
    assert!(matches!(
        sign_up_as_eve(&world).await?,
        SignedUp::ConfirmationNeeded
    ));
    // The password is right, and still the account does not work yet.
    assert_eq!(
        sign_in_as_eve(&world).await.err(),
        Some(AppError::Refused(ErrorCode::AddressNotConfirmed))
    );
    run_jobs(&world).await?;
    let mails = world.mailer.messages();
    assert_eq!(mails.len(), 1);
    assert_eq!(mails[0].to, EVE);
    assert!(
        mails[0]
            .text
            .contains(&format!("{PUBLIC_URL}/confirm-address#"))
    );
    assert!(mails[0].text.contains("24 hours"));
    let secret = secret_in_the_last_e_mail(&world)?;
    confirm(&world, &secret).await?;
    sign_in_as_eve(&world).await?;
    // The link works once.
    assert_eq!(
        confirm(&world, &secret).await.err(),
        Some(AppError::Refused(ErrorCode::ConfirmationInvalid))
    );
    Ok(())
}

#[tokio::test]
async fn auth_with_mail_off_signing_up_works_as_before() -> Outcome {
    let world = world(false).await?;
    assert!(matches!(
        sign_up_as_eve(&world).await?,
        SignedUp::SignedIn(_)
    ));
    sign_in_as_eve(&world).await?;
    run_jobs(&world).await?;
    assert!(world.mailer.messages().is_empty());
    Ok(())
}

#[tokio::test]
async fn auth_the_link_runs_out_after_24_hours() -> Outcome {
    let world = world(true).await?;
    sign_up_as_eve(&world).await?;
    run_jobs(&world).await?;
    let secret = secret_in_the_last_e_mail(&world)?;
    world.clock.advance(HOURS * 3600 - 1);
    world.clock.advance(2);
    assert_eq!(
        confirm(&world, &secret).await.err(),
        Some(AppError::Refused(ErrorCode::ConfirmationInvalid))
    );
    assert_eq!(
        sign_in_as_eve(&world).await.err(),
        Some(AppError::Refused(ErrorCode::AddressNotConfirmed)),
        "an account that ran out of time is still not an account that works"
    );
    Ok(())
}

#[tokio::test]
async fn auth_a_link_works_for_24_hours_from_the_moment_it_was_sent() -> Outcome {
    let world = world(true).await?;
    sign_up_as_eve(&world).await?;
    // The mail server is slow: the link is made when the e-mail goes, not when the person signs up.
    world.clock.advance(3600);
    run_jobs(&world).await?;
    let secret = secret_in_the_last_e_mail(&world)?;
    world.clock.advance(HOURS * 3600 - 60);
    confirm(&world, &secret).await?;
    Ok(())
}

#[tokio::test]
async fn auth_signing_up_again_replaces_an_account_that_ran_out_of_time_and_not_one_that_waits()
-> Outcome {
    let world = world(true).await?;
    sign_up_as_eve(&world).await?;
    run_jobs(&world).await?;
    assert_eq!(
        sign_up_as_eve(&world).await.err(),
        Some(AppError::Refused(ErrorCode::SignUpRefused)),
        "an account that waits holds its address"
    );
    world.clock.advance(SECONDS_PER_DAY + 1);
    assert!(matches!(
        sign_up_as_eve(&world).await?,
        SignedUp::ConfirmationNeeded
    ));
    run_jobs(&world).await?;
    let secret = secret_in_the_last_e_mail(&world)?;
    confirm(&world, &secret).await?;
    sign_in_as_eve(&world).await?;
    Ok(())
}

#[tokio::test]
async fn auth_the_job_that_clears_sessions_removes_the_accounts_that_ran_out_of_time() -> Outcome {
    let world = world(true).await?;
    sign_up_as_eve(&world).await?;
    run_jobs(&world).await?;
    assert_eq!(world.store.lock().state.confirmations.len(), 1);
    world.clock.advance(SECONDS_PER_DAY + 1);
    world.ctx().purge_dead_sessions().await.map_err(text)?;
    assert!(world.store.lock().state.confirmations.is_empty());
    assert!(
        world
            .store
            .lock()
            .state
            .people
            .iter()
            .all(|(person, _)| person.email != EVE)
    );
    // Safe to run again.
    world.ctx().purge_dead_sessions().await.map_err(text)?;
    Ok(())
}

#[tokio::test]
async fn auth_an_account_that_works_is_never_removed_for_having_no_confirmation() -> Outcome {
    let world = world(false).await?;
    sign_up_as_eve(&world).await?;
    world.clock.advance(10 * SECONDS_PER_DAY);
    world.ctx().purge_dead_sessions().await.map_err(text)?;
    assert!(
        world
            .store
            .lock()
            .state
            .people
            .iter()
            .any(|(person, _)| person.email == EVE)
    );
    Ok(())
}
