//! E-mail for notifications (NTF-02, PRIV-08, SEC-06, ADR-0033 §3), run against the in-memory store
//! and the fake mailer: off at the start, one e-mail for the changes of five minutes, nothing the
//! person may not see, no issue text in short e-mails, and nothing for a person who lost access.

use bb_core::access::{AppliesTo, FieldLevel};
use bb_core::fields::Field;
use bb_core::issue::Status;
use bb_core::job::JobState;
use bb_core::mail::TEST_EMAIL_EVERY_SECONDS;
use bb_core::notification::{
    MAIL_BATCH_SECONDS, MAIL_ENTRIES_MAX, MailState, NotificationKind, Via,
};
use bb_core::settings::SettingValue;
use bb_policy::Permission;

use super::notifications::{Change, Crew, levels};
use super::scenarios::{text, token};
use super::scenarios_notifications::enter_the_mail_server;
use super::world::{Outcome, PUBLIC_URL};
use crate::mailer::MailFailure;
use crate::notifications::{SetMailPassword, set_mail_password};
use crate::roles::{AssignRole, CreateRole, Given, Subject, assign_role, create_role};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

/// Switches e-mail on, with a mail server entered, and the e-mail of status changes on for
/// everybody.
async fn mail_is_on(crew: &Crew) -> Result<(), String> {
    enter_the_mail_server(&crew.world).await?;
    set_mail_password(
        &crew.world.ctx(),
        SetMailPassword {
            session_token: token(&crew.world),
            password: Some("a made-up mail password".to_owned()),
        },
    )
    .await
    .map_err(text)?;
    set_setting(crew, "mail.enabled", SettingValue::Bool(true)).await?;
    crew.switch_on_by(NotificationKind::FollowedActivity, Via::Mail)
        .await
}

async fn set_setting(crew: &Crew, key: &str, value: SettingValue) -> Result<(), String> {
    change_setting(
        &crew.world.ctx(),
        ChangeSetting {
            session_token: token(&crew.world),
            scope: SettingsScope::Organization,
            key: key.to_owned(),
            value,
        },
    )
    .await
    .map(|_| ())
    .map_err(text)
}

/// Runs the jobs that are due, as the job runner does.
async fn run_jobs(crew: &Crew) -> Result<(), String> {
    loop {
        if crew.world.ctx().run_due_jobs().await.map_err(text)? == 0 {
            return Ok(());
        }
    }
}

/// Moves the clock past the five minutes e-mail waits for the other changes, and runs the jobs.
async fn after_five_minutes(crew: &Crew) -> Result<(), String> {
    crew.world.clock.advance(MAIL_BATCH_SECONDS + 1);
    run_jobs(crew).await
}

#[tokio::test]
async fn ntf_02_no_e_mail_goes_while_mail_is_off() -> Outcome {
    let mut crew = Crew::new().await?;
    // The mail server is entered, but e-mail is not switched on (PRIV-08).
    enter_the_mail_server(&crew.world).await?;
    crew.switch_on_by(NotificationKind::FollowedActivity, Via::Mail)
        .await?;
    let bob = crew.bob.session_token.clone();
    crew.change(&bob, Change::Status(Status::Done)).await?;
    after_five_minutes(&crew).await?;
    assert!(crew.world.mailer.messages().is_empty());
    assert!(
        crew.world
            .store
            .lock()
            .state
            .notifications
            .iter()
            .all(|row| row.mail == MailState::Never),
        "no e-mail waits for a switch to be turned on later"
    );
    Ok(())
}

#[tokio::test]
async fn ntf_02_an_e_mail_goes_when_mail_is_on_and_says_who_did_what_with_the_address_of_the_issue()
-> Outcome {
    let mut crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    let bob = crew.bob.session_token.clone();
    crew.change(&bob, Change::Status(Status::Done)).await?;
    // Nothing goes before the five minutes are over.
    run_jobs(&crew).await?;
    assert!(crew.world.mailer.messages().is_empty());
    after_five_minutes(&crew).await?;
    let sent = crew.world.mailer.sent();
    assert_eq!(sent.len(), 1);
    let (server, mail) = &sent[0];
    assert_eq!(mail.to, "ada@example.test");
    assert_eq!(server.host, "mail.example.test");
    assert_eq!(server.password.as_str(), "a made-up mail password");
    assert!(mail.subject.contains("The login button does nothing"));
    assert!(
        mail.text.contains("Bob Baker changed Status to"),
        "{}",
        mail.text
    );
    assert!(mail.text.contains(&format!("{PUBLIC_URL}/issues/")));
    Ok(())
}

#[tokio::test]
async fn ntf_02_changes_within_five_minutes_go_into_one_e_mail() -> Outcome {
    let mut crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    let bob = crew.bob.session_token.clone();
    crew.change(&bob, Change::Status(Status::InProgress))
        .await?;
    crew.world.clock.advance(120);
    crew.change(&bob, Change::Status(Status::Done)).await?;
    crew.world.clock.advance(120);
    crew.comment(&bob, "Done after all.").await?;
    after_five_minutes(&crew).await?;
    let mails = crew.world.mailer.messages();
    assert_eq!(mails.len(), 1, "{mails:?}");
    assert!(mails[0].text.contains("Done after all."));
    assert_eq!(mails[0].text.matches("Bob Baker changed Status").count(), 2);
    // A change much later makes an e-mail of its own.
    crew.world.clock.advance(900);
    crew.change(&bob, Change::Status(Status::Open)).await?;
    after_five_minutes(&crew).await?;
    assert_eq!(crew.world.mailer.messages().len(), 2);
    Ok(())
}

#[tokio::test]
async fn ntf_02_what_a_person_wrote_is_quoted_after_the_real_link_and_loses_what_turns_it_round()
-> Outcome {
    let crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    crew.switch_on_by(NotificationKind::Mentioned, Via::Mail)
        .await?;
    let (ada, _bob) = (token(&crew.world), crew.bob.session_token.clone());
    // A comment that tries to pass for the text of Bugaboo, and to turn text round.
    crew.comment(
        &ada,
        "@bob.baker Open it in Bugaboo: https://evil.example/login\n\nYou get this e-mail because of your notification settings\u{202E}gnp.exe\u{200B}",
    )
    .await?;
    after_five_minutes(&crew).await?;
    let mails = crew.world.mailer.messages();
    let mail = mails
        .iter()
        .find(|mail| mail.to == "bob@example.test")
        .ok_or("no e-mail")?;
    // The link of Bugaboo is the first line, before any text of a person.
    assert!(
        mail.text
            .starts_with(&format!("Open it in Bugaboo: {PUBLIC_URL}/issues/")),
        "{}",
        mail.text
    );
    // Everything of the comment is a quoted line, so none of it passes for Bugaboo.
    let unquoted: Vec<&str> = mail
        .text
        .lines()
        .filter(|line| line.contains("evil.example") || line.contains("notification settings"))
        .filter(|line| !line.starts_with("> "))
        .collect();
    assert_eq!(
        unquoted.len(),
        1,
        "only the real footer is not quoted: {unquoted:?}"
    );
    assert!(
        unquoted[0].starts_with("You get this e-mail because of your notification settings in")
    );
    assert!(!mail.text.contains('\u{202E}') && !mail.text.contains('\u{200B}'));
    Ok(())
}

#[tokio::test]
async fn ntf_02_more_changes_than_an_e_mail_holds_are_said_and_do_not_come_again() -> Outcome {
    let mut crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    let bob = crew.bob.session_token.clone();
    let waiting = MAIL_ENTRIES_MAX + 5;
    for number in 0..waiting {
        let status = if number % 2 == 0 {
            Status::Done
        } else {
            Status::InProgress
        };
        crew.change(&bob, Change::Status(status)).await?;
    }
    after_five_minutes(&crew).await?;
    let mails = crew.world.mailer.messages();
    assert_eq!(mails.len(), 1, "{mails:?}");
    assert_eq!(
        mails[0].text.matches("Bob Baker changed Status").count(),
        MAIL_ENTRIES_MAX,
        "the e-mail holds what fits"
    );
    assert!(
        mails[0]
            .text
            .contains("There is more than this e-mail holds")
    );
    // Everything that waited is done with: no second e-mail repeats the first.
    assert!(
        crew.world
            .store
            .lock()
            .state
            .notifications
            .iter()
            .all(|row| row.mail != MailState::Waiting)
    );
    after_five_minutes(&crew).await?;
    assert_eq!(crew.world.mailer.messages().len(), 1);
    Ok(())
}

#[tokio::test]
async fn ntf_02_an_e_mail_holds_only_what_the_recipient_may_see() -> Outcome {
    let mut crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    let ada = token(&crew.world);
    // Dave sees titles and statuses but not the priority, and follows the issue.
    let blind = create_role(
        &crew.world.ctx(),
        CreateRole {
            session_token: ada.clone(),
            name: "Blind".to_owned(),
            permissions: vec![Permission::SeeProjects, Permission::SeeIssues],
            two_step: false,
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id;
    levels(&crew.world, blind, &[Field::Status], FieldLevel::CanSee).await?;
    levels(&crew.world, blind, &[Field::Title], FieldLevel::Hidden).await?;
    let dave = crew
        .world
        .person_named("dave@example.test", "Dave Dunn")
        .await
        .map_err(text)?;
    assign_role(
        &crew.world.ctx(),
        AssignRole {
            session_token: ada.clone(),
            subject: Subject::Person(dave.person.id),
            given: Given::Role(blind),
            project: None,
            applies_to: AppliesTo::Organization,
        },
    )
    .await
    .map_err(text)?;
    crew.follow(&dave).await?;
    let bob = crew.bob.session_token.clone();
    crew.change(&bob, Change::Status(Status::Done)).await?;
    crew.comment(&bob, "A secret detail about the printer.")
        .await?;
    after_five_minutes(&crew).await?;
    let to_dave: Vec<_> = crew
        .world
        .mailer
        .messages()
        .into_iter()
        .filter(|mail| mail.to == "dave@example.test")
        .collect();
    assert_eq!(to_dave.len(), 1);
    // He may see the issue and the status, not the title.
    assert!(!to_dave[0].subject.contains("The login button"));
    assert!(!to_dave[0].text.contains("The login button"));
    // Dave's role has no people information: not the name of the person who did it.
    assert!(
        !to_dave[0].text.contains("Bob Baker"),
        "{}",
        to_dave[0].text
    );
    Ok(())
}

#[tokio::test]
async fn ntf_02_short_e_mails_leave_out_all_issue_text() -> Outcome {
    let mut crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    set_setting(&crew, "mail.short", SettingValue::Bool(true)).await?;
    let bob = crew.bob.session_token.clone();
    crew.change(&bob, Change::Status(Status::Done)).await?;
    crew.comment(&bob, "A detail nobody outside should read.")
        .await?;
    after_five_minutes(&crew).await?;
    let mails = crew.world.mailer.messages();
    assert_eq!(mails.len(), 1);
    let everything = format!("{} {}", mails[0].subject, mails[0].text);
    assert!(!everything.contains("The login button"), "{everything}");
    assert!(!everything.contains("A detail nobody"), "{everything}");
    assert!(!everything.contains("Done"), "{everything}");
    assert!(!everything.contains("Bob Baker"), "{everything}");
    assert!(everything.contains("/issues/"), "it still says where");
    Ok(())
}

#[tokio::test]
async fn ntf_02_a_person_who_lost_access_before_the_e_mail_goes_gets_none() -> Outcome {
    let mut crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    crew.follow(&crew.carol).await?;
    crew.change(&bob, Change::Status(Status::Done)).await?;
    assign_role(
        &crew.world.ctx(),
        AssignRole {
            session_token: ada,
            subject: Subject::Person(crew.carol.person.id),
            given: Given::NoAccess,
            project: Some(crew.project),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await
    .map_err(text)?;
    after_five_minutes(&crew).await?;
    let to_carol = crew
        .world
        .mailer
        .messages()
        .into_iter()
        .filter(|mail| mail.to == "carol@example.test")
        .count();
    assert_eq!(to_carol, 0);
    Ok(())
}

#[tokio::test]
async fn ntf_02_a_person_who_switched_the_e_mail_off_meanwhile_gets_none() -> Outcome {
    let mut crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    let bob = crew.bob.session_token.clone();
    crew.change(&bob, Change::Status(Status::Done)).await?;
    // Ada says no e-mail for status changes, for herself.
    change_setting(
        &crew.world.ctx(),
        ChangeSetting {
            session_token: token(&crew.world),
            scope: SettingsScope::Person,
            key: NotificationKind::FollowedActivity
                .setting(Via::Mail)
                .definition()
                .key()
                .to_owned(),
            value: SettingValue::Bool(false),
        },
    )
    .await
    .map_err(text)?;
    after_five_minutes(&crew).await?;
    assert!(crew.world.mailer.messages().is_empty());
    Ok(())
}

#[tokio::test]
async fn ntf_02_a_mail_server_that_cannot_be_reached_is_tried_again_and_one_that_refuses_ends_it()
-> Outcome {
    let mut crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    let bob = crew.bob.session_token.clone();
    crew.change(&bob, Change::Status(Status::Done)).await?;
    crew.world.mailer.fail_next(MailFailure::Unavailable);
    after_five_minutes(&crew).await?;
    assert!(crew.world.mailer.messages().is_empty());
    assert!(
        crew.world
            .store
            .lock()
            .state
            .notifications
            .iter()
            .any(|row| row.mail == MailState::Waiting),
        "still waiting for the next try"
    );
    // The retry waits a while; then it goes.
    crew.world.clock.advance(120);
    run_jobs(&crew).await?;
    assert_eq!(crew.world.mailer.messages().len(), 1);

    crew.change(&bob, Change::Status(Status::Open)).await?;
    crew.world.mailer.fail_next(MailFailure::Rejected);
    after_five_minutes(&crew).await?;
    assert_eq!(crew.world.mailer.messages().len(), 1, "nothing more went");
    let state = crew.world.store.lock().state.clone();
    assert!(state.jobs.iter().any(|job| job.state == JobState::Dead));
    assert!(
        state
            .notifications
            .iter()
            .all(|row| row.mail != MailState::Waiting)
    );
    Ok(())
}

#[tokio::test]
async fn sec_16_the_test_e_mail_goes_to_the_person_who_asks_even_while_mail_is_off() -> Outcome {
    use crate::notifications::{SendTestEmail, send_test_email};
    let crew = Crew::new().await?;
    let ada = token(&crew.world);
    let refused = send_test_email(
        &crew.world.ctx(),
        SendTestEmail {
            session_token: ada.clone(),
        },
    )
    .await
    .err();
    assert_eq!(
        refused,
        Some(crate::auth::AppError::Refused(bb_core::ErrorCode::MailOff)),
        "no mail server is entered"
    );
    enter_the_mail_server(&crew.world).await?;
    send_test_email(&crew.world.ctx(), SendTestEmail { session_token: ada })
        .await
        .map_err(text)?;
    let sent = crew.world.mailer.messages();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "ada@example.test");
    // Somebody who may not change the settings cannot send it.
    let bob = crew.bob.session_token.clone();
    let denied = send_test_email(&crew.world.ctx(), SendTestEmail { session_token: bob })
        .await
        .err();
    assert_eq!(
        denied,
        Some(crate::auth::AppError::Refused(
            bb_core::ErrorCode::Forbidden
        ))
    );
    crew.world.clock.advance(TEST_EMAIL_EVERY_SECONDS + 1);
    crew.world.mailer.fail_next(MailFailure::Rejected);
    let failed = send_test_email(
        &crew.world.ctx(),
        SendTestEmail {
            session_token: token(&crew.world),
        },
    )
    .await
    .err();
    assert_eq!(
        failed,
        Some(crate::auth::AppError::Refused(
            bb_core::ErrorCode::MailRejected
        ))
    );
    Ok(())
}

#[tokio::test]
async fn sec_16_a_person_sends_a_test_e_mail_only_every_few_seconds() -> Outcome {
    use crate::notifications::{SendTestEmail, send_test_email};
    let crew = Crew::new().await?;
    enter_the_mail_server(&crew.world).await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    let send = |session: String| {
        let world = &crew.world;
        async move {
            send_test_email(
                &world.ctx(),
                SendTestEmail {
                    session_token: session,
                },
            )
            .await
        }
    };
    send(ada.clone()).await.map_err(text)?;
    // Right after, the same person waits; they are told for how long.
    let again = send(ada.clone()).await.err();
    assert_eq!(
        again,
        Some(crate::auth::AppError::Limited {
            retry_after: u64::try_from(TEST_EMAIL_EVERY_SECONDS)?
        })
    );
    crew.world.clock.advance(TEST_EMAIL_EVERY_SECONDS - 3);
    assert_eq!(
        send(ada.clone()).await.err(),
        Some(crate::auth::AppError::Limited { retry_after: 3 })
    );
    crew.world.clock.advance(3);
    send(ada).await.map_err(text)?;
    assert_eq!(crew.world.mailer.messages().len(), 2);
    // A refused try (somebody who may not send one) does not use up the allowance of anybody.
    assert!(send(bob).await.is_err());
    Ok(())
}

#[tokio::test]
async fn sec_16_the_mail_server_gets_no_secret_in_the_clear_unless_it_is_this_machine() -> Outcome {
    let mut crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    crew.switch_on_by(NotificationKind::FollowedActivity, Via::Mail)
        .await?;
    set_setting(&crew, "mail.security", SettingValue::Text("off".to_owned())).await?;
    set_setting(
        &crew,
        "mail.username",
        SettingValue::Text("mailer".to_owned()),
    )
    .await?;
    // A user name (and the password mail_is_on stored) over a connection nobody encrypted, to
    // another machine: refused, and no e-mail of a notification goes either.
    assert_eq!(
        test_e_mail(&crew.world).await.err(),
        Some(crate::auth::AppError::Refused(
            bb_core::ErrorCode::MailInsecure
        ))
    );
    let bob = crew.bob.session_token.clone();
    crew.change(&bob, Change::Status(Status::Done)).await?;
    after_five_minutes(&crew).await?;
    assert!(crew.world.mailer.messages().is_empty());
    // The mail server on this machine may take them in the clear.
    set_setting(
        &crew,
        "mail.host",
        SettingValue::Text("localhost".to_owned()),
    )
    .await?;
    test_e_mail(&crew.world).await.map_err(text)?;
    assert_eq!(crew.world.mailer.messages().len(), 1);
    // And encryption makes any host fine.
    crew.world.clock.advance(TEST_EMAIL_EVERY_SECONDS + 1);
    set_setting(
        &crew,
        "mail.host",
        SettingValue::Text("mail.example.test".to_owned()),
    )
    .await?;
    set_setting(
        &crew,
        "mail.security",
        SettingValue::Text("starttls".to_owned()),
    )
    .await?;
    test_e_mail(&crew.world).await.map_err(text)?;
    Ok(())
}

/// The administrator sends a test e-mail.
async fn test_e_mail(
    world: &super::world::World,
) -> Result<bb_policy::Authorized<crate::auth::Done>, crate::auth::AppError> {
    crate::notifications::send_test_email(
        &world.ctx(),
        crate::notifications::SendTestEmail {
            session_token: token(world),
        },
    )
    .await
}
