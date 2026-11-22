//! Corrections of a person's name and e-mail address (PRIV-06, ADR-0044, decision 9), run against
//! the in-memory store: one's own name at once, one's own address after its link, and the
//! corrections of an administrator with a reason, which the person is told of and the security log
//! records.

use bb_core::ErrorCode;
use bb_core::e_mail_change::LINK_HOURS;
use bb_core::id::{EntityId, InvitationId, UserId};
use bb_core::notification::NotificationKind;
use bb_core::security_log::Digest;
use bb_core::sign_in::SecurityAction;
use bb_core::sign_up_invitation::{ADDRESSES_PER_HOUR_MAX, SignUpMail};
use bb_core::text::REASON_MAX;

use super::notifications::{Crew, kinds};
use super::scenarios::{Added, measured, text, token};
use super::scenarios_notifications::switch_mail_on;
use super::world::{OTHER_PASSWORD, Outcome, PASSWORD, World};
use crate::auth::{
    AppError, ChangeSignUpPolicy, SignIn, SignUpPolicy, change_sign_up_policy, sign_in,
};
use crate::mailer::MailMessage;
use crate::people::{
    AskEmailChange, ChangeMyName, ConfirmEmailChange, CorrectEmail, CorrectName, ask_e_mail_change,
    change_my_name, confirm_e_mail_change, correct_e_mail, correct_name,
};

const BOB: &str = "bob@example.test";
const NEW: &str = "bob.baker@example.test";

/// Runs the jobs that are due.
async fn run_jobs(world: &World) -> Result<(), String> {
    while world.ctx().run_due_jobs().await.map_err(text)? > 0 {}
    Ok(())
}

fn refused<T>(answer: &Result<T, AppError>) -> Option<ErrorCode> {
    match answer {
        Err(AppError::Refused(code)) => Some(*code),
        _ => None,
    }
}

/// A crew with e-mail on.
async fn crew() -> Result<Crew, String> {
    let crew = Crew::new().await?;
    switch_mail_on(&crew.world).await?;
    Ok(crew)
}

async fn ask(world: &World, session: &str, email: &str) -> Result<(), AppError> {
    ask_e_mail_change(
        &world.ctx(),
        AskEmailChange {
            session_token: session.to_owned(),
            password: PASSWORD.to_owned(),
            email: email.to_owned(),
        },
    )
    .await
    .map(|_| ())
}

async fn confirm(world: &World, secret: &str) -> Result<(), AppError> {
    confirm_e_mail_change(
        &world.ctx(),
        ConfirmEmailChange {
            secret: secret.to_owned(),
        },
    )
    .await
    .map(|_| ())
}

/// The e-mails that went to `address`.
fn mails_to(world: &World, address: &str) -> Vec<MailMessage> {
    world
        .mailer
        .messages()
        .into_iter()
        .filter(|mail| mail.to == address)
        .collect()
}

/// The secret in the link of the last e-mail to `address`.
fn secret_sent_to(world: &World, address: &str) -> Result<String, String> {
    let mails = mails_to(world, address);
    let mail = mails.last().ok_or("no e-mail went")?;
    let (_, secret) = mail
        .text
        .split_once("/e-mail-change#")
        .ok_or("no link in the e-mail")?;
    Ok(secret
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned())
}

fn address_of(world: &World, person: UserId) -> Option<String> {
    world
        .store
        .lock()
        .state
        .people
        .iter()
        .find(|(known, _)| known.id == person)
        .map(|(known, _)| known.email.clone())
}

fn name_of(world: &World, person: UserId) -> Option<String> {
    world
        .store
        .lock()
        .state
        .people
        .iter()
        .find(|(known, _)| known.id == person)
        .map(|(known, _)| known.display_name.clone())
}

async fn signs_in(world: &World, email: &str) -> bool {
    sign_in(
        &world.ctx(),
        SignIn {
            email: email.to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        },
    )
    .await
    .is_ok()
}

/// How many entries of the security log say `action` by `by` about `person`.
fn noted(world: &World, action: SecurityAction, by: UserId, person: UserId) -> usize {
    world
        .store
        .lock()
        .log
        .iter()
        .filter(|entry| {
            entry.action == action
                && entry.person == Some(by)
                && entry
                    .subject
                    .as_ref()
                    .is_some_and(|subject| subject.id == EntityId::from(person))
        })
        .count()
}

pub(crate) async fn change_my_name_scenario(world: &World) -> Added {
    let ctx = world.ctx();
    let call = change_my_name(
        &ctx,
        ChangeMyName {
            session_token: token(world),
            name: "Ada Admiral".to_owned(),
        },
    );
    measured(&world.store, call).await
}

pub(crate) async fn ask_e_mail_change_scenario(world: &World) -> Added {
    switch_mail_on(world).await?;
    measured(&world.store, ask(world, &token(world), NEW)).await
}

pub(crate) async fn confirm_e_mail_change_scenario(world: &World) -> Added {
    switch_mail_on(world).await?;
    ask(world, &token(world), NEW).await.map_err(text)?;
    run_jobs(world).await?;
    let secret = secret_sent_to(world, NEW)?;
    measured(&world.store, confirm(world, &secret)).await
}

pub(crate) async fn correct_name_scenario(world: &World) -> Added {
    let bob = world.person(BOB).await.map_err(text)?;
    let admin = token(world);
    let call = correct_name_of(world, &admin, bob.person.id, "A typo in the name");
    measured(&world.store, call).await
}

pub(crate) async fn correct_e_mail_scenario(world: &World) -> Added {
    let bob = world.person(BOB).await.map_err(text)?;
    let admin = token(world);
    let call = correct_address_of(world, &admin, bob.person.id, NEW);
    measured(&world.store, call).await
}

#[tokio::test]
async fn priv_06_a_person_corrects_their_own_name_and_it_is_in_the_history() -> Outcome {
    let crew = crew().await?;
    let (world, bob) = (&crew.world, crew.bob.person.id);
    let changed = change_my_name(
        &world.ctx(),
        ChangeMyName {
            session_token: crew.bob.session_token.clone(),
            name: "  Bob Brewer ".to_owned(),
        },
    )
    .await?
    .into_value();
    assert_eq!(changed.name, "Bob Brewer");
    assert_eq!(name_of(world, bob).as_deref(), Some("Bob Brewer"));
    // The history keeps the change of one's own name, with both names.
    let kept = world.store.lock().events.iter().any(|event| {
        event.entity.id == EntityId::from(bob)
            && event.changes.iter().any(|change| {
                change.field == "name"
                    && change.before == Some(serde_json::json!("Bob Baker"))
                    && change.after == Some(serde_json::json!("Bob Brewer"))
            })
    });
    assert!(kept, "the history keeps the change");
    assert_eq!(noted(world, SecurityAction::OwnNameChanged, bob, bob), 1);
    // A name that is no name is refused.
    let empty = change_my_name(
        &world.ctx(),
        ChangeMyName {
            session_token: crew.bob.session_token.clone(),
            name: "   ".to_owned(),
        },
    )
    .await;
    assert_eq!(refused(&empty), Some(ErrorCode::InvalidRequest));
    Ok(())
}

#[tokio::test]
async fn priv_06_a_new_e_mail_address_counts_only_after_its_link_is_opened() -> Outcome {
    let crew = crew().await?;
    let (world, bob) = (&crew.world, crew.bob.person.id);
    ask(world, &crew.bob.session_token, NEW).await?;
    run_jobs(world).await?;
    // The link went to the new address; until it is opened the old one is Bob's.
    let secret = secret_sent_to(world, NEW)?;
    assert_eq!(address_of(world, bob).as_deref(), Some(BOB));
    assert!(signs_in(world, BOB).await);
    assert!(!signs_in(world, NEW).await);
    // The secret is kept only as its keyed digest.
    let stored = world.store.lock().state.e_mail_changes.clone();
    assert_eq!(stored.len(), 1);
    assert_ne!(
        stored
            .first()
            .map(|change| change.token_digest.as_bytes().to_vec()),
        Some(secret.as_bytes().to_vec())
    );
    confirm(world, &secret).await?;
    assert_eq!(address_of(world, bob).as_deref(), Some(NEW));
    assert!(signs_in(world, NEW).await);
    assert!(!signs_in(world, BOB).await);
    assert_eq!(noted(world, SecurityAction::OwnEmailChanged, bob, bob), 1);
    // The change of the address keeps no address in clear in the history of Bob.
    let in_clear = world.store.lock().events.iter().any(|event| {
        event.entity.id == EntityId::from(bob)
            && event.changes.iter().any(|change| {
                [&change.before, &change.after].iter().any(|value| {
                    value
                        .as_ref()
                        .is_some_and(|value| value.to_string().contains("example.test"))
                })
            })
    });
    assert!(!in_clear, "no address in an event");
    Ok(())
}

#[tokio::test]
async fn priv_06_the_old_address_is_told_when_the_change_is_done() -> Outcome {
    let crew = crew().await?;
    let world = &crew.world;
    ask(world, &crew.bob.session_token, NEW).await?;
    run_jobs(world).await?;
    // Asking tells the old address nothing yet.
    assert!(mails_to(world, BOB).is_empty());
    confirm(world, &secret_sent_to(world, NEW)?).await?;
    run_jobs(world).await?;
    let told = mails_to(world, BOB);
    let [note] = told.as_slice() else {
        return Err(format!("one note to the old address, found {}", told.len()).into());
    };
    assert!(note.text.contains("/me/sessions"), "{}", note.text);
    // Other sessions stay: the note offers to end them.
    assert!(signs_in(world, NEW).await);
    Ok(())
}

/// How many events of `person`'s history name the change `field`.
fn events_naming(world: &World, person: UserId, field: &str) -> usize {
    world
        .store
        .lock()
        .events
        .iter()
        .filter(|event| {
            event.entity.id == EntityId::from(person)
                && event.changes.iter().any(|change| change.field == field)
        })
        .count()
}

#[tokio::test]
async fn priv_06_a_taken_address_is_answered_like_a_free_one() -> Outcome {
    let crew = crew().await?;
    let (world, bob) = (&crew.world, crew.bob.person.id);
    let carol = "carol@example.test";
    ask(world, &crew.bob.session_token, NEW).await?;
    run_jobs(world).await?;
    let first = secret_sent_to(world, NEW)?;
    // The same answer for a taken address as for a free one.
    ask(world, &crew.bob.session_token, carol).await?;
    run_jobs(world).await?;
    // The e-mail to it says an account is there, with no link.
    let mails = mails_to(world, carol);
    let [said] = mails.as_slice() else {
        return Err(format!("one e-mail to the taken address, found {}", mails.len()).into());
    };
    assert!(!said.text.contains("/e-mail-change#"));
    // A taken address leaves the trace a free one does: a waiting change in place of the one
    // before, whose link no longer works, and the same event.
    assert_eq!(world.store.lock().state.e_mail_changes.len(), 1);
    assert_eq!(events_naming(world, bob, "e-mail-change-link"), 2);
    assert_eq!(events_naming(world, bob, "e-mail-change-asked"), 2);
    assert_eq!(
        refused(&confirm(world, &first).await),
        Some(ErrorCode::EmailChangeLinkInvalid)
    );
    assert_eq!(address_of(world, bob).as_deref(), Some(BOB));
    Ok(())
}

#[tokio::test]
async fn priv_06_an_address_gets_one_link_e_mail_in_a_while() -> Outcome {
    let crew = crew().await?;
    let world = &crew.world;
    for _ in 0..3 {
        ask(world, &crew.bob.session_token, NEW).await?;
    }
    run_jobs(world).await?;
    // Asking again and again cannot fill the inbox of the address, and the link that went works.
    assert_eq!(mails_to(world, NEW).len(), 1);
    let sent = secret_sent_to(world, NEW)?;
    world
        .clock
        .advance(bb_core::sign_up_invitation::RESEND_AFTER_MINUTES * 60);
    ask(world, &crew.bob.session_token, NEW).await?;
    run_jobs(world).await?;
    assert_eq!(mails_to(world, NEW).len(), 2);
    assert_ne!(
        secret_sent_to(world, NEW)?,
        sent,
        "a new link replaces the old"
    );
    Ok(())
}

#[tokio::test]
async fn priv_06_the_hourly_cap_counts_for_a_new_address_with_approved_domains_too() -> Outcome {
    let crew = crew().await?;
    let world = &crew.world;
    change_sign_up_policy(
        &world.ctx(),
        ChangeSignUpPolicy {
            session_token: token(world),
            policy: SignUpPolicy {
                enabled: true,
                domains: vec!["example.test".to_owned()],
                version: 0,
            },
        },
    )
    .await?;
    // The server wrote to as many addresses this hour as it will.
    let now = world.ctx().now();
    world
        .store
        .lock()
        .state
        .sign_up_mail
        .extend((0..ADDRESSES_PER_HOUR_MAX).map(|at| SignUpMail {
            address: Digest::new([u8::try_from(at).unwrap_or(u8::MAX); 32]),
            invitation: InvitationId::generate(&mut world.ctx().ids()),
            sent_at: now,
        }));
    // A new address can be anywhere, so the list of approved domains does not lift the cap.
    ask(world, &crew.bob.session_token, NEW).await?;
    run_jobs(world).await?;
    assert!(mails_to(world, NEW).is_empty());
    Ok(())
}

#[tokio::test]
async fn priv_06_an_address_taken_after_the_link_went_makes_the_link_fail() -> Outcome {
    let crew = crew().await?;
    let (world, bob) = (&crew.world, crew.bob.person.id);
    ask(world, &crew.bob.session_token, NEW).await?;
    run_jobs(world).await?;
    let secret = secret_sent_to(world, NEW)?;
    // Meanwhile an administrator gives the address to Carol.
    correct_address_of(world, &token(world), crew.carol.person.id, NEW).await?;
    assert_eq!(
        refused(&confirm(world, &secret).await),
        Some(ErrorCode::EmailChangeLinkInvalid)
    );
    assert_eq!(address_of(world, bob).as_deref(), Some(BOB));
    Ok(())
}

#[tokio::test]
async fn priv_06_the_link_runs_out_after_an_hour_and_works_once() -> Outcome {
    let crew = crew().await?;
    let world = &crew.world;
    ask(world, &crew.bob.session_token, NEW).await?;
    run_jobs(world).await?;
    let first = secret_sent_to(world, NEW)?;
    world.clock.advance(LINK_HOURS * 3600);
    assert_eq!(
        refused(&confirm(world, &first).await),
        Some(ErrorCode::EmailChangeLinkInvalid)
    );
    // The daily job deletes what ran out.
    world.ctx().purge_dead_sessions().await.map_err(text)?;
    assert!(world.store.lock().state.e_mail_changes.is_empty());
    // A new link works once (Bob signs in again: the hour ended his session).
    let again = sign_in(
        &world.ctx(),
        SignIn {
            email: BOB.to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        },
    )
    .await?;
    ask(world, &again.session_token, NEW).await?;
    run_jobs(world).await?;
    let second = secret_sent_to(world, NEW)?;
    confirm(world, &second).await?;
    assert_eq!(
        refused(&confirm(world, &second).await),
        Some(ErrorCode::EmailChangeLinkInvalid)
    );
    assert_eq!(
        refused(&confirm(world, "not-a-secret").await),
        Some(ErrorCode::EmailChangeLinkInvalid)
    );
    Ok(())
}

#[tokio::test]
async fn priv_06_a_change_of_address_needs_the_password_and_e_mail() -> Outcome {
    let crew = Crew::new().await?;
    let world = &crew.world;
    // While e-mail is off, nothing could prove the address.
    assert_eq!(
        refused(&ask(world, &crew.bob.session_token, NEW).await),
        Some(ErrorCode::EmailChangeNeedsMail)
    );
    switch_mail_on(world).await?;
    let wrong = ask_e_mail_change(
        &world.ctx(),
        AskEmailChange {
            session_token: crew.bob.session_token.clone(),
            password: OTHER_PASSWORD.to_owned(),
            email: NEW.to_owned(),
        },
    )
    .await;
    assert_eq!(refused(&wrong), Some(ErrorCode::WrongCredentials));
    run_jobs(world).await?;
    assert!(mails_to(world, NEW).is_empty());
    Ok(())
}

async fn correct_name_of(
    world: &World,
    session: &str,
    person: UserId,
    reason: &str,
) -> Result<(), AppError> {
    correct_name(
        &world.ctx(),
        CorrectName {
            session_token: session.to_owned(),
            person,
            name: "Robert Baker".to_owned(),
            reason: Some(reason.to_owned()),
        },
    )
    .await
    .map(|_| ())
}

async fn correct_address_of(
    world: &World,
    session: &str,
    person: UserId,
    email: &str,
) -> Result<(), AppError> {
    correct_e_mail(
        &world.ctx(),
        CorrectEmail {
            session_token: session.to_owned(),
            person,
            email: email.to_owned(),
            reason: Some("Bob married and asked by letter".to_owned()),
        },
    )
    .await
    .map(|_| ())
}

#[tokio::test]
async fn priv_06_only_manage_people_corrects_others_and_needs_a_reason() -> Outcome {
    let crew = crew().await?;
    let (world, bob) = (&crew.world, crew.bob.person.id);
    // Carol holds no *Manage people*.
    assert_eq!(
        refused(&correct_name_of(world, &crew.carol.session_token, bob, "typo").await),
        Some(ErrorCode::Forbidden)
    );
    assert_eq!(
        refused(&correct_address_of(world, &crew.carol.session_token, bob, NEW).await),
        Some(ErrorCode::Forbidden)
    );
    // Ada does, but never without a reason, nor with one that is too long.
    assert_eq!(
        refused(&correct_name_of(world, &token(world), bob, "  ").await),
        Some(ErrorCode::CorrectionReasonNeeded)
    );
    let no_reason = correct_name(
        &world.ctx(),
        CorrectName {
            session_token: token(world),
            person: bob,
            name: "Robert Baker".to_owned(),
            reason: None,
        },
    )
    .await;
    assert_eq!(refused(&no_reason), Some(ErrorCode::InvalidRequest));
    let too_long = "a".repeat(REASON_MAX + 1);
    assert_eq!(
        refused(&correct_name_of(world, &token(world), bob, &too_long).await),
        Some(ErrorCode::InvalidRequest)
    );
    // Nor her own name or address: those she changes in her profile.
    let ada = world.admin.person.id;
    assert_eq!(
        refused(&correct_name_of(world, &token(world), ada, "A typo").await),
        Some(ErrorCode::CorrectionOwnProfile)
    );
    assert_eq!(
        refused(&correct_address_of(world, &token(world), ada, NEW).await),
        Some(ErrorCode::CorrectionOwnProfile)
    );
    assert_eq!(name_of(world, bob).as_deref(), Some("Bob Baker"));
    correct_name_of(world, &token(world), bob, "A typo in the name").await?;
    assert_eq!(name_of(world, bob).as_deref(), Some("Robert Baker"));
    // An address of another account is refused; a free one counts at once.
    assert_eq!(
        refused(&correct_address_of(world, &token(world), bob, "carol@example.test").await),
        Some(ErrorCode::EmailTaken)
    );
    correct_address_of(world, &token(world), bob, NEW).await?;
    assert_eq!(address_of(world, bob).as_deref(), Some(NEW));
    // Both addresses are told.
    run_jobs(world).await?;
    assert_eq!(mails_to(world, BOB).len(), 1);
    assert_eq!(mails_to(world, NEW).len(), 1);
    Ok(())
}

#[tokio::test]
async fn priv_06_the_person_is_told_and_the_security_log_records_the_correction() -> Outcome {
    let crew = crew().await?;
    let (world, bob, ada) = (&crew.world, crew.bob.person.id, crew.world.admin.person.id);
    correct_name_of(world, &token(world), bob, "A typo in the name").await?;
    correct_address_of(world, &token(world), bob, NEW).await?;
    let heard = crew.heard(&crew.bob.session_token).await?;
    assert_eq!(
        kinds(&heard),
        vec![
            NotificationKind::DetailsCorrected,
            NotificationKind::DetailsCorrected
        ]
    );
    // The notification carries the administrator's reason.
    let words = serde_json::to_string(&heard).map_err(text)?;
    assert!(words.contains("A typo in the name"), "{words}");
    assert!(words.contains("Bob married and asked by letter"), "{words}");
    assert_eq!(noted(world, SecurityAction::NameCorrected, ada, bob), 1);
    assert_eq!(noted(world, SecurityAction::EmailCorrected, ada, bob), 1);
    // The security log names who did what to whom, and holds no value.
    for entry in &world.store.lock().log {
        let said = format!("{entry:?}");
        for value in ["Bob Baker", "Robert Baker", BOB, NEW, "A typo in the name"] {
            assert!(!said.contains(value), "{said}");
        }
    }
    // The event of a correction keeps the reason, and of the old values only their keyed digests.
    let events = world.store.lock().events.clone();
    let corrections: Vec<_> = events
        .iter()
        .filter(|event| {
            event.entity.id == EntityId::from(bob)
                && event.changes.iter().any(|change| change.field == "reason")
        })
        .collect();
    assert_eq!(corrections.len(), 2);
    for event in corrections {
        let text = serde_json::to_string(&event.changes).map_err(text)?;
        assert!(!text.contains("Bob Baker") && !text.contains(BOB), "{text}");
    }
    Ok(())
}

#[tokio::test]
async fn priv_04_a_switched_off_or_removed_person_keeps_their_name_and_address() -> Outcome {
    let crew = crew().await?;
    let (world, bob) = (&crew.world, crew.bob.person.id);
    ask(world, &crew.bob.session_token, NEW).await?;
    run_jobs(world).await?;
    let secret = secret_sent_to(world, NEW)?;
    super::switch_off::switched_off(world, bob).await?;
    // Neither the link nor an administrator changes a switched-off person.
    assert_eq!(
        refused(&confirm(world, &secret).await),
        Some(ErrorCode::PersonNotActive)
    );
    let corrected = correct_name_of(world, &token(world), bob, "A typo in the name").await;
    assert_eq!(refused(&corrected), Some(ErrorCode::PersonNotActive));
    assert_eq!(address_of(world, bob).as_deref(), Some(BOB));
    // The removal takes the waiting change with it.
    crate::privacy::removal::remove_person(
        &world.ctx(),
        crate::privacy::removal::RemovePerson {
            session_token: token(world),
            person: bob,
            comments: None,
            password: PASSWORD.to_owned(),
        },
    )
    .await?;
    let waiting = world.store.read(|state| {
        state
            .e_mail_changes
            .iter()
            .filter(|change| change.person == bob)
            .count()
    });
    assert_eq!(waiting, 0);
    Ok(())
}
