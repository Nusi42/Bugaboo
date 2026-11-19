//! What people are told of *Download my data* (PRIV-03, ADR-0044, decisions 2 and 4), run against
//! the in-memory store and the fake mailer: whoever asked hears that the file is ready, and the
//! person whose data an administrator took hears of it in the app and by e-mail, with the reason,
//! whatever they chose, and sees the administrator only as *Who sees people* lets them.

use bb_core::access::PeopleLevel;
use bb_core::message::{self, Message};
use bb_core::notification::{NotificationKind, Via};
use bb_core::people_kinds::PeopleKind;
use bb_core::settings::SettingValue;
use bb_policy::PersonRef;

use super::data_exports::{ask_for, ask_mine, run_jobs};
use super::mail::{after_five_minutes, mail_is_on, set_setting};
use super::notifications::{Crew, kinds};
use super::scenarios::{text, token};
use super::world::Outcome;
use crate::notifications::NotificationWhat;
use crate::roles::{SetPeopleLevel, set_level};

const REASON: &str = "Carol asked by letter";

/// The organization switches every notification it can off, in the app and by e-mail, and locks
/// nothing: a person who wants to hear of nothing hears of nothing that can be switched off.
async fn everything_switched_off(crew: &Crew) -> Result<(), String> {
    for kind in NotificationKind::ALL {
        for via in [Via::App, Via::Mail] {
            if let Some(setting) = kind.setting(via) {
                set_setting(crew, setting.definition().key(), SettingValue::Bool(false)).await?;
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn priv_03_whoever_asked_hears_that_the_file_is_ready() -> Outcome {
    let crew = Crew::new().await?;
    let (world, bob) = (&crew.world, crew.bob.session_token.as_str());
    ask_mine(world, bob).await?;
    // Nothing is said before the file is written.
    assert!(crew.heard(bob).await?.is_empty());
    run_jobs(world).await?;
    let heard = crew.heard(bob).await?;
    assert_eq!(kinds(&heard), vec![NotificationKind::ExportReady]);
    // Bugaboo wrote the file: nobody is named.
    assert_eq!(heard[0].by, None);
    assert_eq!(heard[0].issue, None);
    // The file Ada asked for Carol is Ada's to download: Ada hears it is ready, Carol does not.
    ask_for(world, &token(world), crew.carol.person.id, REASON).await?;
    run_jobs(world).await?;
    assert_eq!(
        kinds(&crew.heard(&token(world)).await?),
        vec![NotificationKind::ExportReady]
    );
    assert!(
        !kinds(&crew.heard(&crew.carol.session_token).await?)
            .contains(&NotificationKind::ExportReady)
    );
    Ok(())
}

#[tokio::test]
async fn priv_03_the_person_is_told_of_an_administrators_export_with_the_reason_whatever_they_chose()
-> Outcome {
    let crew = Crew::new().await?;
    let world = &crew.world;
    everything_switched_off(&crew).await?;
    ask_for(world, &token(world), crew.carol.person.id, REASON).await?;
    let heard = crew.heard(&crew.carol.session_token).await?;
    assert_eq!(kinds(&heard), vec![NotificationKind::ExportByAdministrator]);
    assert_eq!(
        heard[0].what,
        NotificationWhat::ByAdministrator {
            reason: Some(REASON.to_owned())
        }
    );
    // Who did it is named as *Who sees people* lets Carol see authors (the next test).
    assert!(heard[0].by.is_some());
    // Nobody else hears of it, and a person's own export tells nobody that an administrator took it.
    assert!(crew.heard(&token(world)).await?.is_empty());
    ask_mine(world, &crew.bob.session_token).await?;
    assert!(crew.heard(&crew.bob.session_token).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn priv_03_the_administrator_is_named_only_as_who_sees_people_lets_the_person() -> Outcome {
    let crew = Crew::new().await?;
    let world = &crew.world;
    let mut shown = Vec::new();
    for level in [PeopleLevel::Names, PeopleLevel::Hidden] {
        set_level(
            &world.ctx(),
            SetPeopleLevel {
                session_token: token(world),
                role: crew.role,
                project: None,
                kind: PeopleKind::Authors,
                level: Some(level),
            },
        )
        .await
        .map_err(text)?;
        ask_for(world, &token(world), crew.carol.person.id, REASON).await?;
        run_jobs(world).await?;
        let heard = crew.heard(&crew.carol.session_token).await?;
        shown.push(heard.first().and_then(|item| item.by.clone()));
    }
    assert!(
        matches!(&shown[0], Some(PersonRef::Name { name, .. }) if name == "Ada Admin"),
        "{shown:?}"
    );
    assert_eq!(shown[1], Some(PersonRef::Someone));
    Ok(())
}

#[tokio::test]
async fn priv_03_the_person_gets_an_e_mail_of_an_administrators_export_at_the_address_on_record()
-> Outcome {
    let crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    everything_switched_off(&crew).await?;
    let world = &crew.world;
    ask_for(world, &token(world), crew.carol.person.id, REASON).await?;
    after_five_minutes(&crew).await?;
    let sent = world.mailer.messages();
    let to_carol: Vec<_> = sent
        .iter()
        .filter(|mail| mail.to == "carol@example.test")
        .collect();
    assert_eq!(to_carol.len(), 1, "{sent:?}");
    let mail = to_carol[0];
    let kind = Message::from(NotificationKind::ExportByAdministrator.label()).render();
    assert_eq!(mail.subject, message::mail_subject_notice(&kind).render());
    assert!(mail.text.contains(REASON), "{}", mail.text);
    // Ada switched every e-mail off and asked for no file of her own: she gets none.
    assert!(sent.iter().all(|mail| mail.to != "ada@example.test"));
    Ok(())
}

#[tokio::test]
async fn priv_03_a_switched_off_person_is_told_by_e_mail_of_an_administrators_export() -> Outcome {
    let crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    let world = &crew.world;
    super::switch_off::switched_off(world, crew.carol.person.id).await?;
    ask_for(world, &token(world), crew.carol.person.id, REASON).await?;
    after_five_minutes(&crew).await?;
    let sent = world.mailer.messages();
    let to_carol: Vec<_> = sent
        .iter()
        .filter(|mail| mail.to == "carol@example.test")
        .collect();
    // The one message a switched-off person still gets, with the reason (ADR-0044, decision 4).
    assert_eq!(to_carol.len(), 1, "{sent:?}");
    assert!(to_carol[0].text.contains(REASON), "{}", to_carol[0].text);
    Ok(())
}

#[tokio::test]
async fn priv_03_an_e_mail_says_the_file_is_ready_under_its_own_subject() -> Outcome {
    let crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    let world = &crew.world;
    ask_mine(world, &crew.bob.session_token).await?;
    run_jobs(world).await?;
    after_five_minutes(&crew).await?;
    let sent = world.mailer.messages();
    let mail = sent
        .iter()
        .find(|mail| mail.to == "bob@example.test")
        .ok_or("an e-mail to Bob")?;
    let kind = Message::from(NotificationKind::ExportReady.label()).render();
    assert_eq!(mail.subject, message::mail_subject_notice(&kind).render());
    assert!(
        mail.text
            .contains(&Message::from(message::NOTIFICATIONS_EXPORT_READY).render()),
        "{}",
        mail.text
    );
    Ok(())
}
