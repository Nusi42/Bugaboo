//! Signing up on one's own (AUTH-06, M1-F38, ADR-0033 §3): a self-invitation. Nothing is stored for
//! the address but a job; the link that the job mails goes through the invitation page, where the
//! person chooses a name and a password. The answer is the same for every address.

use bb_core::ErrorCode;
use bb_core::settings::SettingValue;
use bb_core::sign_up_invitation::{
    ADDRESSES_PER_HOUR_MAX, HOURS, MAIL_MEMORY, MAILS_CAP_WINDOW, RESEND_AFTER_MINUTES,
};
use bb_core::time::SECONDS_PER_DAY;

use super::scenarios::{text, token};
use super::scenarios_notifications::switch_mail_on;
use super::world::{Outcome, PASSWORD, PUBLIC_URL, World};
use crate::auth::{
    AcceptInvitation, AppError, ChangeSignUpPolicy, SignIn, SignUp, SignUpPolicy,
    accept_invitation, change_sign_up_policy, sign_in, sign_up,
};
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
        switch_mail_on(&world).await?;
    }
    Ok(world)
}

async fn sign_up_for(world: &World, address: &str) -> Result<(), AppError> {
    sign_up(
        &world.ctx(),
        SignUp {
            email: address.to_owned(),
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
        .split_once("/invitation#")
        .ok_or("no link in the e-mail")?;
    Ok(secret
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned())
}

async fn follow(world: &World, secret: &str) -> Result<(), AppError> {
    accept_invitation(
        &world.ctx(),
        AcceptInvitation {
            secret: secret.to_owned(),
            display_name: "Eve Evans".to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        },
    )
    .await
    .map(|_| ())
}

fn people_with(world: &World, address: &str) -> usize {
    world
        .store
        .lock()
        .state
        .people
        .iter()
        .filter(|(person, _)| person.email == address)
        .count()
}

#[tokio::test]
async fn auth_no_password_and_no_person_exist_before_the_link_is_followed() -> Outcome {
    let world = world(true).await?;
    let people = world.store.lock().state.people.len();
    let handles = world.store.lock().state.handles.len();
    sign_up_for(&world, EVE).await?;
    run_jobs(&world).await?;
    assert_eq!(world.store.lock().state.people.len(), people);
    assert_eq!(world.store.lock().state.handles.len(), handles);
    assert_eq!(people_with(&world, EVE), 0);
    // Nobody can sign in with any password yet.
    let signed_in = sign_in(
        &world.ctx(),
        SignIn {
            email: EVE.to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        },
    )
    .await;
    assert_eq!(
        signed_in.err(),
        Some(AppError::Refused(ErrorCode::WrongCredentials))
    );
    let mails = world.mailer.messages();
    assert_eq!(mails.len(), 1);
    assert_eq!(mails[0].to, EVE);
    assert!(mails[0].text.contains(&format!("{PUBLIC_URL}/invitation#")));
    // The link makes the account, with the name and the password the person chooses there.
    follow(&world, &secret_in_the_last_e_mail(&world)?).await?;
    assert_eq!(people_with(&world, EVE), 1);
    Ok(())
}

#[tokio::test]
async fn auth_a_link_works_once() -> Outcome {
    let world = world(true).await?;
    sign_up_for(&world, EVE).await?;
    run_jobs(&world).await?;
    let secret = secret_in_the_last_e_mail(&world)?;
    follow(&world, &secret).await?;
    assert_eq!(
        follow(&world, &secret).await.err(),
        Some(AppError::Refused(ErrorCode::InvitationInvalid))
    );
    assert_eq!(people_with(&world, EVE), 1);
    Ok(())
}

#[tokio::test]
async fn auth_the_link_runs_out_after_24_hours() -> Outcome {
    let world = world(true).await?;
    sign_up_for(&world, EVE).await?;
    run_jobs(&world).await?;
    let secret = secret_in_the_last_e_mail(&world)?;
    world.clock.advance(HOURS * 3600 + 1);
    assert_eq!(
        follow(&world, &secret).await.err(),
        Some(AppError::Refused(ErrorCode::InvitationInvalid))
    );
    assert_eq!(people_with(&world, EVE), 0);
    Ok(())
}

#[tokio::test]
async fn auth_a_link_works_for_24_hours_from_the_moment_it_was_sent() -> Outcome {
    let world = world(true).await?;
    sign_up_for(&world, EVE).await?;
    // The mail server is slow: the link is made when the e-mail goes, not when the person signs up.
    world.clock.advance(3600);
    run_jobs(&world).await?;
    let secret = secret_in_the_last_e_mail(&world)?;
    world.clock.advance(HOURS * 3600 - 60);
    follow(&world, &secret).await?;
    Ok(())
}

#[tokio::test]
async fn auth_a_sign_up_for_an_address_with_an_account_is_answered_like_any_other() -> Outcome {
    let world = world(true).await?;
    let known = "ada@example.test";
    let before = world.store.lock().state.jobs.len();
    let for_new = sign_up_for(&world, EVE).await;
    let for_known = sign_up_for(&world, known).await;
    assert_eq!(format!("{for_new:?}"), format!("{for_known:?}"));
    assert!(for_known.is_ok());
    assert_eq!(
        world.store.lock().state.jobs.len(),
        before + 2,
        "a job for each"
    );
    run_jobs(&world).await?;
    let mails = world.mailer.messages();
    assert_eq!(mails.len(), 2);
    let to_known = mails
        .iter()
        .find(|mail| mail.to == known)
        .ok_or("no e-mail")?;
    // The one who has an account is told so, and gets no link that would make a second one.
    assert!(!to_known.text.contains("/invitation#"));
    assert!(to_known.text.contains(&format!("{PUBLIC_URL}/sign-in")));
    assert_eq!(people_with(&world, known), 1);
    let to_new = mails
        .iter()
        .find(|mail| mail.to == EVE)
        .ok_or("no e-mail")?;
    assert!(to_new.text.contains("/invitation#"));
    Ok(())
}

#[tokio::test]
async fn auth_signing_up_again_soon_sends_no_second_e_mail() -> Outcome {
    let world = world(true).await?;
    sign_up_for(&world, EVE).await?;
    sign_up_for(&world, EVE).await?;
    run_jobs(&world).await?;
    assert_eq!(world.mailer.messages().len(), 1, "the inbox is not filled");
    // After the pause a new link goes.
    world.clock.advance(RESEND_AFTER_MINUTES * 60 + 1);
    sign_up_for(&world, EVE).await?;
    run_jobs(&world).await?;
    assert_eq!(world.mailer.messages().len(), 2);
    Ok(())
}

#[tokio::test]
async fn auth_signing_up_is_refused_while_mail_is_off_for_nothing_could_prove_the_address()
-> Outcome {
    let world = world(false).await?;
    assert_eq!(
        sign_up_for(&world, EVE).await.err(),
        Some(AppError::Refused(ErrorCode::SignUpRefused))
    );
    // E-mail on, but switched off again before the job runs: nothing goes.
    switch_mail_on(&world).await?;
    sign_up_for(&world, EVE).await?;
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(&world),
            scope: SettingsScope::Organization,
            key: "mail.enabled".to_owned(),
            value: SettingValue::Bool(false),
        },
    )
    .await
    .map_err(text)?;
    run_jobs(&world).await?;
    assert!(world.mailer.messages().is_empty());
    Ok(())
}

#[tokio::test]
async fn auth_the_job_that_clears_sessions_removes_the_links_that_are_done() -> Outcome {
    let world = world(true).await?;
    sign_up_for(&world, EVE).await?;
    run_jobs(&world).await?;
    assert_eq!(world.store.lock().state.sign_up_invitations.len(), 1);
    world.ctx().purge_dead_sessions().await.map_err(text)?;
    assert_eq!(
        world.store.lock().state.sign_up_invitations.len(),
        1,
        "an open link stays"
    );
    world.clock.advance(HOURS * 3600 + SECONDS_PER_DAY + 1);
    world.ctx().purge_dead_sessions().await.map_err(text)?;
    assert!(world.store.lock().state.sign_up_invitations.is_empty());
    // Safe to run again.
    world.ctx().purge_dead_sessions().await.map_err(text)?;
    Ok(())
}

#[tokio::test]
async fn sec_the_server_sends_to_only_so_many_addresses_in_an_hour_while_anybody_may_sign_up()
-> Outcome {
    let world = world(true).await?;
    set_the_rule(&world, 1, true, &[]).await?;
    let cap = usize::try_from(ADDRESSES_PER_HOUR_MAX).map_err(text)?;
    let logged = world.store.log_count();
    for number in 0..cap + 3 {
        sign_up_for(&world, &format!("n{number}@example.test")).await?;
    }
    run_jobs(&world).await?;
    assert_eq!(world.mailer.messages().len(), cap);
    // Reaching the cap is written down once, not for every e-mail turned away.
    let capped = world
        .store
        .lock()
        .log
        .iter()
        .skip(logged)
        .filter(|entry| entry.action == bb_core::sign_in::SecurityAction::SignUpMailCapped)
        .count();
    assert_eq!(capped, 1);
    // An hour later the cap counts again from nothing.
    world.clock.advance(MAILS_CAP_WINDOW.as_secs() + 1);
    sign_up_for(&world, "late@example.test").await?;
    run_jobs(&world).await?;
    assert_eq!(world.mailer.messages().len(), cap + 1);
    Ok(())
}

#[tokio::test]
async fn sec_a_list_of_domains_keeps_the_cap_from_turning_people_of_the_organization_away()
-> Outcome {
    let world = world(true).await?;
    let cap = usize::try_from(ADDRESSES_PER_HOUR_MAX).map_err(text)?;
    for number in 0..cap + 3 {
        sign_up_for(&world, &format!("n{number}@example.test")).await?;
    }
    run_jobs(&world).await?;
    assert_eq!(world.mailer.messages().len(), cap + 3);
    Ok(())
}

#[tokio::test]
async fn gdpr_the_memory_of_an_e_mail_goes_when_it_is_no_use_any_more() -> Outcome {
    let world = world(true).await?;
    sign_up_for(&world, EVE).await?;
    run_jobs(&world).await?;
    assert_eq!(world.store.lock().state.sign_up_mail.len(), 1);
    // Within the hour it blocks the next e-mail and counts for the cap, so it stays.
    world.clock.advance(RESEND_AFTER_MINUTES * 60 + 1);
    world.ctx().purge_dead_sessions().await.map_err(text)?;
    assert_eq!(world.store.lock().state.sign_up_mail.len(), 1);
    // After it only keeps what is not needed: the job removes it, long before the link goes.
    world.clock.advance(MAIL_MEMORY.as_secs());
    world.ctx().purge_dead_sessions().await.map_err(text)?;
    assert!(world.store.lock().state.sign_up_mail.is_empty());
    assert_eq!(world.store.lock().state.sign_up_invitations.len(), 1);
    Ok(())
}

#[tokio::test]
async fn gdpr_the_next_sign_up_e_mail_forgets_what_is_no_use_any_more_without_the_daily_job()
-> Outcome {
    let world = world(true).await?;
    sign_up_for(&world, EVE).await?;
    run_jobs(&world).await?;
    world.clock.advance(MAIL_MEMORY.as_secs() + 1);
    sign_up_for(&world, "zed@example.test").await?;
    run_jobs(&world).await?;
    let kept = world.store.lock().state.sign_up_mail.len();
    assert_eq!(kept, 1, "only the memory of the e-mail that went last");
    Ok(())
}

/// Changes the rule of signing up, from the version it has (`world` makes version 1).
async fn set_the_rule(
    world: &World,
    version: i64,
    enabled: bool,
    domains: &[&str],
) -> Result<(), String> {
    change_sign_up_policy(
        &world.ctx(),
        ChangeSignUpPolicy {
            session_token: token(world),
            policy: SignUpPolicy {
                enabled,
                domains: domains.iter().map(|domain| (*domain).to_owned()).collect(),
                version,
            },
        },
    )
    .await
    .map(|_| ())
    .map_err(text)
}

#[tokio::test]
async fn auth_switching_signing_up_off_stops_the_links_that_are_out() -> Outcome {
    let world = world(true).await?;
    sign_up_for(&world, EVE).await?;
    run_jobs(&world).await?;
    let secret = secret_in_the_last_e_mail(&world)?;
    set_the_rule(&world, 1, false, &["example.test"]).await?;
    assert_eq!(
        follow(&world, &secret).await.err(),
        Some(AppError::Refused(ErrorCode::InvitationInvalid))
    );
    assert_eq!(people_with(&world, EVE), 0);
    // So does taking the domain off the list.
    set_the_rule(&world, 2, true, &["other.test"]).await?;
    assert!(follow(&world, &secret).await.is_err());
    let lookup = crate::auth::invitation_lookup(
        &world.ctx(),
        crate::auth::InvitationLookupQuery {
            secret: secret.clone(),
        },
    )
    .await;
    assert!(lookup.is_err(), "the page does not show the address either");
    Ok(())
}

#[tokio::test]
async fn auth_switching_signing_up_off_stops_the_e_mail_that_waits() -> Outcome {
    let world = world(true).await?;
    sign_up_for(&world, EVE).await?;
    sign_up_for(&world, "ada@example.test").await?;
    set_the_rule(&world, 1, false, &["example.test"]).await?;
    run_jobs(&world).await?;
    assert!(
        world.mailer.messages().is_empty(),
        "neither a link nor the word that there is an account"
    );
    assert!(world.store.lock().state.sign_up_invitations.is_empty());
    Ok(())
}

#[tokio::test]
async fn auth_an_address_with_an_account_signed_up_twice_gets_one_e_mail() -> Outcome {
    let world = world(true).await?;
    let known = "ada@example.test";
    sign_up_for(&world, known).await?;
    sign_up_for(&world, known).await?;
    run_jobs(&world).await?;
    assert_eq!(world.mailer.messages().len(), 1);
    // After the pause one more may go.
    world.clock.advance(RESEND_AFTER_MINUTES * 60 + 1);
    sign_up_for(&world, known).await?;
    run_jobs(&world).await?;
    assert_eq!(world.mailer.messages().len(), 2);
    Ok(())
}

#[tokio::test]
async fn auth_a_job_that_is_tried_again_after_the_mail_server_was_down_still_sends() -> Outcome {
    let world = world(true).await?;
    sign_up_for(&world, EVE).await?;
    world
        .mailer
        .fail_next(crate::mailer::MailFailure::Unavailable);
    run_jobs(&world).await?;
    assert!(world.mailer.messages().is_empty());
    world.clock.advance(60);
    run_jobs(&world).await?;
    assert_eq!(
        world.mailer.messages().len(),
        1,
        "the same sign-up may send"
    );
    Ok(())
}
