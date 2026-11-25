//! *E-mail me this search* over HTTP on SQLite and PostgreSQL (QRY-07): one hour never brings two
//! e-mails of one e-mailed search (saved again or changed while its e-mail waits, or an hour missed
//! while the server did not run), *Switch off* makes them wait for the person even when they are
//! switched on again before the next one is due, and what the person loses between setting it and
//! its e-mail, or may not read, is not in the e-mail.

use bb_core::fields::Field;
use bb_core::search_mail::MailPause;
use bb_policy::StartingRole;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::http::{ADA_EMAIL, Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, key_of, ok, version_of};
use super::removal_http::switch_off;
use super::search_mail_http::{
    EVERY_DAY, HOUR, SEARCHES, bob_mails_his_search, mails_to, pass_hours, reader_seeing, save,
    set, set_at, start, stored,
};

/// A few minutes, less than a session waits without a request.
const MINUTES_20: i64 = 20 * 60;
const BOB: &str = "bob@example.test";

#[tokio::test]
async fn qry_07_saving_again_while_the_e_mail_waits_sends_one_e_mail() -> TestResult {
    each_database("qry_07_saved_again", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let search = save(&mut ada, "Open bugs", "").await?;
        ok(&set(&mut ada, &search, EVERY_DAY, false).await?)?;
        // 8:00: the round queues the e-mail of 9:00 and moves the next one to tomorrow.
        start(&rig).await?;
        rig.clock.advance(MINUTES_20);
        // Saved again unchanged: the next e-mail is today at 9:00 again.
        ok(&set(&mut ada, &search, EVERY_DAY, false).await?)?;
        pass_hours(&rig, 3).await?;
        assert_eq!(mails_to(&rig, ADA_EMAIL).len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_changing_the_hour_while_the_e_mail_waits_sends_one_e_mail() -> TestResult {
    each_database("qry_07_hour_changed", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let search = save(&mut ada, "Open bugs", "").await?;
        ok(&set(&mut ada, &search, EVERY_DAY, false).await?)?;
        start(&rig).await?;
        rig.clock.advance(MINUTES_20);
        // From 9:00 to 11:00 today: the e-mail of 9:00 that waits does not go.
        ok(&set_at(&mut ada, &search, (EVERY_DAY, HOUR + 2), false).await?)?;
        pass_hours(&rig, 2).await?;
        assert!(mails_to(&rig, ADA_EMAIL).is_empty());
        pass_hours(&rig, 2).await?;
        assert_eq!(mails_to(&rig, ADA_EMAIL).len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_an_hour_missed_while_the_server_was_down_sends_one_e_mail() -> TestResult {
    each_database("qry_07_missed_hour", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let search = save(&mut ada, "Open bugs", "").await?;
        ok(&set(&mut ada, &search, EVERY_DAY, false).await?)?;
        // Friday 8:00: the server stops before the e-mail of 9:00 and starts again on Saturday at
        // 8:30. The late one goes at once and stands for Saturday's 9:00 too.
        rig.clock.advance(24 * 3600 + 30 * 60);
        start(&rig).await?;
        assert_eq!(mails_to(&rig, ADA_EMAIL).len(), 1);
        pass_hours(&rig, 3).await?;
        assert_eq!(mails_to(&rig, ADA_EMAIL).len(), 1);
        // Sunday's goes as usual.
        pass_hours(&rig, 24).await?;
        assert_eq!(mails_to(&rig, ADA_EMAIL).len(), 2);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_switched_off_and_on_again_between_two_e_mails_waits_for_the_person() -> TestResult {
    each_database("qry_07_off_and_on", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        start(&rig).await?;
        let (_, bob_id, search) = bob_mails_his_search(&site, &mut ada).await?;
        switch_off(&mut ada, bob_id).await?;
        let on = ada
            .send("POST", &format!("/api/v1/people/{bob_id}/switch-on"), None)
            .await?;
        assert!(on.status.is_success(), "{}", on.body);
        assert_eq!(
            stored(&rig, bob_id, &search).await?.paused,
            Some(MailPause::SwitchedOff)
        );
        pass_hours(&rig, 25).await?;
        assert!(mails_to(&rig, BOB).is_empty());
        // Bob starts it again himself.
        let mut bob = Browser::signed_in_as(&app, BOB).await?;
        ok(&set(&mut bob, &search, EVERY_DAY, false).await?)?;
        pass_hours(&rig, 25).await?;
        assert_eq!(mails_to(&rig, BOB).len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_what_the_person_loses_before_the_e_mail_is_not_in_it() -> TestResult {
    each_database("qry_07_lost_before_sending", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        start(&rig).await?;
        let (mut bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let web = ok(&ada
            .send(
                "POST",
                "/api/v1/projects",
                Some(json!({"short_name": "WEB", "name": "Website"})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("a project without ID")?
            .to_owned();
        let kept = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let trashed = key_of(&site.create(&mut ada, "Scanner stuck").await?)?;
        let elsewhere = key_of(
            &ada.send(
                "POST",
                &format!("/api/v1/projects/{web}/issues"),
                Some(site.new_issue("Website down")),
            )
            .await?,
        )?;
        let search = save(&mut bob, "Everything", "").await?;
        ok(&set(&mut bob, &search, EVERY_DAY, false).await?)?;
        // Between setting and sending, Bob gets *No access* on WEB and an issue goes to the trash.
        ok(&ada
            .send(
                "POST",
                &format!("/api/v1/projects/{web}/assignments"),
                Some(json!({"who": {"person": bob_id}, "applies_to": "project-and-sub-projects"})),
            )
            .await?)?;
        let deleted = ada
            .send("DELETE", &format!("{ISSUES}/{trashed}"), None)
            .await?;
        assert!(deleted.status.is_success(), "{}", deleted.body);
        pass_hours(&rig, 3).await?;
        let mails = mails_to(&rig, BOB);
        let text = &mails.first().ok_or("no e-mail for Bob")?.text;
        assert!(text.contains(kept.as_str()), "{text}");
        assert!(!text.contains(trashed.as_str()), "{text}");
        assert!(!text.contains(elsewhere.as_str()), "{text}");
        assert!(!text.contains("Website"), "{text}");
        assert!(text.contains("finds 1 issue now"), "{text}");
        Ok(())
    })
    .await
}

/// *Only when something is new or changed* goes by when an issue last changed, which the person may
/// see on every list: a change of a field they may not read sends the e-mail, which holds nothing
/// of that field.
#[tokio::test]
async fn qry_07_a_change_of_a_field_the_person_may_not_read_sends_the_e_mail_without_it()
-> TestResult {
    each_database("qry_07_hidden_change", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        start(&rig).await?;
        let mut reader = reader_seeing(&site, &rig, &[Field::Title, Field::Status]).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let search = save(&mut reader, "Mine", "").await?;
        ok(&set(&mut reader, &search, EVERY_DAY, true).await?)?;
        pass_hours(&rig, 3).await?;
        assert_eq!(mails_to(&rig, "reader@example.test").len(), 1);
        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let version = version_of(&mut ada, &issue).await?;
        ok(&ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({"version": version, "priority": "critical"})),
            )
            .await?)?;
        pass_hours(&rig, 24).await?;
        let mails = mails_to(&rig, "reader@example.test");
        assert_eq!(mails.len(), 2);
        let text = mails[1].text.to_lowercase();
        assert!(
            !text.contains("critical") && !text.contains("priority"),
            "{text}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_the_private_search_of_somebody_else_is_not_found() -> TestResult {
    each_database("qry_07_others_search", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let adas = save(&mut ada, "Ada's own", "").await?;
        let refused = set(&mut bob, &adas, EVERY_DAY, true).await?;
        assert_eq!(refused.code(), "resource.not-found", "{}", refused.body);
        let unknown = bob
            .send("DELETE", &format!("{SEARCHES}/{adas}/mail"), None)
            .await?;
        assert_eq!(unknown.code(), "resource.not-found", "{}", unknown.body);
        Ok(())
    })
    .await
}
