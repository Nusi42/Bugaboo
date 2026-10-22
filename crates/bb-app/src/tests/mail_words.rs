//! The words of an e-mail, end to end (NTF-02, WP-13): the subject and the links of the e-mail of
//! a renewal of issue IDs that gave up, what a short e-mail leaves out, the footer, and the time
//! zone of the person who gets an e-mail with a moment in it.

use std::collections::BTreeMap;

use bb_core::access::FieldLevel;
use bb_core::fields::FieldKey;
use bb_core::message::{self, Message};
use bb_core::notification::{NotificationKind, Via};
use bb_core::own_fields::FieldType;
use bb_core::settings::SettingValue;
use serde_json::json;

use super::mail::{after_five_minutes, mail_is_on, set_setting};
use super::notifications::Crew;
use super::scenarios::{text, token};
use super::scenarios_fields::a_field;
use super::world::{Outcome, PUBLIC_URL};
use crate::issues::{ChangeIssue, change_issue};
use crate::roles::{SetFieldLevel, set_level};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

/// Carol is told that the new issue IDs of the project could not be given out, and wants it by
/// e-mail.
async fn a_renewal_gave_up(crew: &Crew) -> Result<(), String> {
    crew.switch_on_by(NotificationKind::RunEnded, Via::Mail)
        .await?;
    let payload =
        json!({"project": crew.project.to_string(), "by": crew.carol.person.id.to_string()});
    crew.world
        .ctx()
        .renewal_gave_up(&payload)
        .await
        .map_err(text)
}

#[tokio::test]
async fn ntf_02_the_e_mail_of_a_failed_renewal_has_its_subject_its_links_and_the_footer() -> Outcome
{
    let crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    a_renewal_gave_up(&crew).await?;
    after_five_minutes(&crew).await?;
    let mails = crew.world.mailer.messages();
    assert_eq!(mails.len(), 1, "{mails:?}");
    let mail = &mails[0];
    assert_eq!(mail.to, "carol@example.test");
    assert_eq!(
        mail.subject,
        Message::from(message::MAIL_SUBJECT_IDS_FAILED).render()
    );
    // Where the result is, and where to try again (a project that the person can see).
    let notifications = message::mail_open_notifications(format!("{PUBLIC_URL}/notifications"));
    assert!(
        mail.text.starts_with(&notifications.render()),
        "{}",
        mail.text
    );
    let settings = message::mail_open_project_settings(format!("{PUBLIC_URL}/p/ACME/settings"));
    assert!(mail.text.contains(&settings.render()), "{}", mail.text);
    assert!(mail.text.contains("A project"), "{}", mail.text);
    // The footer names the page where the person changes what they get.
    assert!(
        mail.text
            .contains("under My settings › Notification settings."),
        "{}",
        mail.text
    );
    Ok(())
}

#[tokio::test]
async fn ntf_02_a_short_e_mail_of_a_failed_renewal_names_no_project() -> Outcome {
    let crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    set_setting(&crew, "mail.short", SettingValue::Bool(true)).await?;
    a_renewal_gave_up(&crew).await?;
    after_five_minutes(&crew).await?;
    let mails = crew.world.mailer.messages();
    assert_eq!(mails.len(), 1, "{mails:?}");
    let text = &mails[0].text;
    assert!(!text.contains("A project"), "{text}");
    assert!(!text.contains("ACME") && !text.contains("/p/"), "{text}");
    assert!(
        text.contains(&Message::from(message::NOTIFICATIONS_ISSUE_IDS_FAILED_SHORT).render()),
        "{text}"
    );
    assert!(
        text.contains(&format!("{PUBLIC_URL}/notifications")),
        "{text}"
    );
    Ok(())
}

#[tokio::test]
async fn ntf_02_a_moment_in_an_e_mail_is_written_in_the_time_zone_of_the_recipient() -> Outcome {
    let crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    crew.switch_on_by(NotificationKind::FollowedFields, Via::Mail)
        .await?;
    let starts = a_field(&crew.world, "Starts", FieldType::DateTime, &[]).await?;
    set_level(
        &crew.world.ctx(),
        SetFieldLevel {
            session_token: token(&crew.world),
            role: crew.role,
            project: None,
            issue_type: None,
            field: FieldKey::Own(starts.id),
            level: Some(FieldLevel::CanChange),
        },
    )
    .await
    .map_err(text)?;
    // Ada, who gets the e-mail, chose a time zone; the moment is 07:30 in UTC.
    change_setting(
        &crew.world.ctx(),
        ChangeSetting {
            session_token: token(&crew.world),
            scope: SettingsScope::Person,
            key: "region.time-zone".to_owned(),
            value: SettingValue::Text("Asia/Tokyo".to_owned()),
        },
    )
    .await
    .map_err(text)?;
    change_issue(
        &crew.world.ctx(),
        ChangeIssue {
            session_token: crew.bob.session_token.clone(),
            issue: crew.issue.to_string(),
            version: crew.version,
            issue_type: None,
            title: None,
            status: None,
            priority: None,
            assigned_to: None,
            fields: BTreeMap::from([(starts.id.to_string(), json!("2026-10-05T07:30:00Z"))]),
        },
    )
    .await
    .map_err(text)?;
    after_five_minutes(&crew).await?;
    let mails = crew.world.mailer.messages();
    let ada = mails
        .iter()
        .find(|mail| mail.to == "ada@example.test")
        .ok_or("Ada gets an e-mail")?;
    let said = message::mail_date_time("2026-10-05, 4:30 PM", "Asia/Tokyo").render();
    assert!(ada.text.contains(&said), "{}", ada.text);
    assert!(!ada.text.contains("7:30 AM"), "{}", ada.text);
    Ok(())
}
