//! Notifications, the mail server and the jobs that keep and delete, over HTTP on both databases
//! (NTF-01, NTF-02, SEC-16, PRIV-05, ADR-0033): who is told, a reader that resumes after a restart
//! without doubles, the mail password that is stored encrypted and never sent back, and the jobs
//! that remove what outlived its keep period.

use axum::http::StatusCode;
use bb_api::{LimitsConfig, router};
use bb_app::access::StartingRole;
use bb_app::auth::RequestInfo;
use bb_app::notifications::NotificationStore as _;
use bb_core::job::JobState;
use bb_core::notification::KEEP_DAYS;
use bb_core::time::SECONDS_PER_DAY;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, Reply, options, rig_and_app};
use super::issues_site::{ISSUES, Site, key_of, version_of};
use super::rig::Rig;

const NOTIFICATIONS: &str = "/api/v1/notifications";

/// The items of a list.
fn items(list: &Reply) -> Vec<Value> {
    list.body["items"].as_array().cloned().unwrap_or_default()
}

/// Changes the status of an issue as `browser`.
async fn finish(browser: &mut Browser, issue: &str) -> Result<(), Failure> {
    let version = version_of(browser, issue).await?;
    let changed = browser
        .send(
            "PATCH",
            &format!("{ISSUES}/{issue}"),
            Some(json!({"version": version, "status": "done"})),
        )
        .await?;
    assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
    Ok(())
}

#[tokio::test]
async fn ntf_01_a_follower_hears_over_http_and_marks_what_was_read() -> TestResult {
    each_database("ntf_01_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        rig.clock.advance(5);
        finish(&mut bob, &issue).await?;
        assert!(items(&bob.send("GET", NOTIFICATIONS, None).await?).is_empty());

        let listed = ada.send("GET", NOTIFICATIONS, None).await?;
        assert_eq!(listed.status, StatusCode::OK, "{}", listed.body);
        let entries = items(&listed);
        assert_eq!(entries.len(), 1, "{}", listed.body);
        let entry = &entries[0];
        assert_eq!(entry["kind"], "followed-activity");
        assert_eq!(entry["read"], false);
        assert_eq!(entry["change"]["field"], "status");
        assert_eq!(entry["change"]["after"]["text"], "done");
        assert_eq!(entry["issue"]["readable_id"], issue.as_str());
        assert_eq!(entry["by"]["name"], "Bob Baker");
        assert!(
            listed.headers.get("etag").is_none(),
            "personal data is never stored"
        );

        let unread = ada
            .send("GET", &format!("{NOTIFICATIONS}/unread"), None)
            .await?;
        assert_eq!(unread.body["unread"], 1);
        let read = ada
            .send(
                "POST",
                &format!("{NOTIFICATIONS}/read"),
                Some(json!({"ids": [entry["id"]]})),
            )
            .await?;
        assert_eq!(read.status, StatusCode::NO_CONTENT, "{}", read.body);
        let unread = ada
            .send("GET", &format!("{NOTIFICATIONS}/unread"), None)
            .await?;
        assert_eq!(unread.body["unread"], 0);
        let unread_only = ada
            .send("GET", &format!("{NOTIFICATIONS}?tab=unread"), None)
            .await?;
        assert!(items(&unread_only).is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ntf_01_the_people_of_a_change_stay_people_as_the_reader_may_see_them() -> TestResult {
    each_database("ntf_01_people_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        rig.set_organization_switch("notifications.followed-fields-in-app", true)
            .await?;
        // A guest sees people of the kind *Assigned to* by their role only.
        let (mut gus, _) = site.person(StartingRole::Guest, "gus").await?;
        let issue = key_of(&site.create(&mut gus, "Printer jams").await?)?;
        let followed = gus
            .send("PUT", &format!("{ISSUES}/{issue}/follow"), None)
            .await?;
        assert!(followed.status.is_success(), "{}", followed.body);
        let (_bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        rig.clock.advance(5);
        let version = version_of(&mut ada, &issue).await?;
        let assigned = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({"version": version, "assigned_to": [bob_id.to_string()]})),
            )
            .await?;
        assert_eq!(assigned.status, StatusCode::OK, "{}", assigned.body);

        let heard = items(&gus.send("GET", NOTIFICATIONS, None).await?);
        let change = heard
            .iter()
            .find(|item| item["change"]["field"] == "assigned-to")
            .ok_or("no change of the assignee")?;
        assert!(change.pointer("/change/after/text").is_none(), "{change}");
        let people = change["change"]["after"]["people"]
            .as_array()
            .ok_or("no people")?;
        assert_eq!(people.len(), 1);
        let shown = people[0].to_string();
        assert!(
            !shown.contains("Bob Baker"),
            "a guest is not told the name: {shown}"
        );
        assert!(shown.contains("Team member"), "the role is: {shown}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_a_notification_and_an_e_mail_leave_out_a_field_the_role_of_the_reader_hides()
-> TestResult {
    use bb_app::access::Permission;
    use bb_core::access::FieldLevel;
    use bb_core::fields::{Field, FieldKey};

    use super::exit_masks::person_with_levels;

    each_database("sec_06_delivery_hidden_field_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        rig.turn_mail_on().await?;
        for key in [
            "notifications.followed-fields-in-app",
            "notifications.followed-fields-by-mail",
            "notifications.followed-activity-by-mail",
        ] {
            rig.set_organization_switch(key, true).await?;
        }
        // A reader with a role of their own: the title and the status are shown, the priority is
        // hidden.
        let (mut reader, _) = person_with_levels(
            &site,
            &rig,
            "reader",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[
                (FieldKey::BuiltIn(Field::Title), None, FieldLevel::CanSee),
                (FieldKey::BuiltIn(Field::Status), None, FieldLevel::CanSee),
            ],
        )
        .await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let followed = reader
            .send("PUT", &format!("{ISSUES}/{issue}/follow"), None)
            .await?;
        assert!(followed.status.is_success(), "{}", followed.body);
        rig.clock.advance(5);
        let version = version_of(&mut ada, &issue).await?;
        let changed = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({
                    "version": version,
                    "title": "Printer jams twice",
                    "status": "in-progress",
                    "priority": "critical"
                })),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);

        // In the app: the title and the status, and nothing of the priority.
        let heard = items(&reader.send("GET", NOTIFICATIONS, None).await?);
        let fields: Vec<&str> = heard
            .iter()
            .filter_map(|item| item["change"]["field"].as_str())
            .collect();
        assert!(
            fields.contains(&"title") || fields.contains(&"status"),
            "{fields:?}"
        );
        assert!(!fields.contains(&"priority"), "{fields:?}");
        assert!(
            !Value::Array(heard.clone())
                .to_string()
                .to_lowercase()
                .contains("critical")
        );

        // By e-mail: the same.
        rig.clock
            .advance(bb_core::notification::MAIL_BATCH_SECONDS + 1);
        rig.run_jobs().await?;
        let mails = rig.mailer.messages();
        let mail = mails
            .iter()
            .find(|mail| mail.to == "reader@example.test")
            .ok_or("no e-mail for the reader")?;
        let text = mail.text.to_lowercase();
        assert!(
            text.contains("printer jams twice") || text.contains("in progress"),
            "{text}"
        );
        assert!(
            !text.contains("priority") && !text.contains("critical"),
            "{text}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn m3_f70_of_two_changes_of_a_handle_at_the_same_moment_one_is_kept_on_both_databases()
-> TestResult {
    each_database("m3_f70_handle_parallel", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        site.person(StartingRole::TeamMember, "bob").await?;
        for round in 0..5 {
            // Two sessions of one person change the handle at the same moment, once in each
            // period: one change is made, the other is told it came too soon.
            let mut one = Browser::signed_in_as(&app, "bob@example.test").await?;
            let mut two = Browser::signed_in_as(&app, "bob@example.test").await?;
            let (first, second) = tokio::join!(
                one.send(
                    "PUT",
                    "/api/v1/me/handle",
                    Some(json!({"handle": format!("first.{round}")}))
                ),
                two.send(
                    "PUT",
                    "/api/v1/me/handle",
                    Some(json!({"handle": format!("second.{round}")}))
                )
            );
            let (first, second) = (first?, second?);
            let codes = [first.code().to_owned(), second.code().to_owned()];
            let made = [first.status, second.status]
                .iter()
                .filter(|status| **status == StatusCode::OK)
                .count();
            assert_eq!(made, 1, "round {round}: {codes:?}");
            assert!(
                codes.contains(&"handle.changed-lately".to_owned()),
                "round {round}: {codes:?}"
            );
            rig.clock
                .advance(bb_core::handle::CHANGE_EVERY_DAYS * SECONDS_PER_DAY + 1);
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ntf_01_the_reader_resumes_after_a_restart_without_doubles() -> TestResult {
    each_database("ntf_01_restart", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        rig.clock.advance(5);
        finish(&mut bob, &issue).await?;
        let before = items(&ada.send("GET", NOTIFICATIONS, None).await?);
        assert_eq!(before.len(), 1);

        // The server stops and starts again on the same database: a new reader, its place read
        // from the database.
        let second = rig.restarted(db).await?;
        let again = router(options(&second, LimitsConfig::default()));
        let mut ada_again = Browser::signed_in_as(&again, super::http::ADA_EMAIL).await?;
        let after = items(&ada_again.send("GET", NOTIFICATIONS, None).await?);
        assert_eq!(after, before, "nothing is made twice");

        // And the new reader goes on where the old one stopped.
        second.clock.advance(10);
        let mut bob_again = Browser::signed_in_as(&again, "bob@example.test").await?;
        let version = version_of(&mut bob_again, &issue).await?;
        let changed = bob_again
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({"version": version, "status": "in-progress"})),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        let last = items(&ada_again.send("GET", NOTIFICATIONS, None).await?);
        assert_eq!(last.len(), 2, "the old one once, and the new one");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_16_the_mail_password_is_stored_encrypted_and_never_sent_back() -> TestResult {
    each_database("sec_16_mail_password_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let password = "a made-up mail password 7";
        let denied = bob
            .send(
                "PUT",
                "/api/v1/mail/password",
                Some(json!({"password": password})),
            )
            .await?;
        assert_eq!(denied.code(), "access.forbidden");
        let unknown = bob.send("GET", "/api/v1/mail", None).await?;
        assert_eq!(unknown.code(), "access.forbidden");
        let before = ada.send("GET", "/api/v1/mail", None).await?;
        assert_eq!(
            before.body,
            json!({"credential_set": false, "readiness": "incomplete"})
        );
        let set = ada
            .send(
                "PUT",
                "/api/v1/mail/password",
                Some(json!({"password": password})),
            )
            .await?;
        assert_eq!(set.status, StatusCode::NO_CONTENT, "{}", set.body);
        // The page learns that a password is entered, never what it is.
        let after = ada.send("GET", "/api/v1/mail", None).await?;
        assert_eq!(
            after.body,
            json!({"credential_set": true, "readiness": "incomplete"})
        );
        for path in ["/api/v1/settings", "/api/v1/admin/status"] {
            let shown = ada.send("GET", path, None).await?;
            assert!(!shown.body.to_string().contains(password), "{path}");
        }
        let stored = rig.storage.mail_password().await?.ok_or("no password")?;
        assert_eq!(stored.as_slice(), password.as_bytes());
        let removed = ada
            .send("PUT", "/api/v1/mail/password", Some(json!({})))
            .await?;
        assert_eq!(removed.status, StatusCode::NO_CONTENT);
        assert!(rig.storage.mail_password().await?.is_none());
        let gone = ada.send("GET", "/api/v1/mail", None).await?;
        assert_eq!(
            gone.body,
            json!({"credential_set": false, "readiness": "incomplete"})
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ntf_02_the_test_e_mail_goes_to_the_person_who_asks_through_the_mail_server() -> TestResult
{
    each_database("ntf_02_test_email_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let off = ada.send("POST", "/api/v1/mail/test", None).await?;
        assert_eq!(off.code(), "mail.off", "no mail server is entered");
        for (key, value) in [
            ("mail.host", json!("mail.example.test")),
            ("mail.from", json!("bugaboo@example.test")),
        ] {
            let put = ada
                .send(
                    "PUT",
                    &format!("/api/v1/settings/{key}"),
                    Some(json!({"value": value})),
                )
                .await?;
            assert_eq!(put.status, StatusCode::OK, "{key}: {}", put.body);
        }
        let sent = ada.send("POST", "/api/v1/mail/test", None).await?;
        assert_eq!(sent.status, StatusCode::NO_CONTENT, "{}", sent.body);
        let mails = rig.mailer.messages();
        assert_eq!(mails.len(), 1);
        assert_eq!(mails[0].to, super::http::ADA_EMAIL);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_05_notifications_go_after_90_days() -> TestResult {
    each_database("priv_05_ninety_days", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        rig.clock.advance(5);
        finish(&mut bob, &issue).await?;
        assert_eq!(items(&ada.send("GET", NOTIFICATIONS, None).await?).len(), 1);

        // The daily jobs run at the start and then every day.
        let ctx = rig.services.ctx(RequestInfo::default());
        ctx.start_daily_jobs()
            .await
            .map_err(|error| error.to_string())?;
        rig.run_jobs().await?;
        assert_eq!(
            items(&ada.send("GET", NOTIFICATIONS, None).await?).len(),
            1,
            "it is not old yet"
        );
        rig.clock.advance((KEEP_DAYS + 1) * SECONDS_PER_DAY);
        // A session lasts a day at most: sign in again.
        let mut ada = Browser::signed_in_as(&app, super::http::ADA_EMAIL).await?;
        rig.run_jobs().await?;
        assert!(items(&ada.send("GET", NOTIFICATIONS, None).await?).is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0017_every_keep_and_delete_job_runs_again_with_nothing_more_to_delete() -> TestResult {
    each_database("adr_0017_keep_jobs", async |db| {
        let (rig, _app) = rig_and_app(db).await?;
        let ctx = rig.services.ctx(RequestInfo::default());
        ctx.start_daily_jobs()
            .await
            .map_err(|error| error.to_string())?;
        let first = rig.run_jobs().await?;
        assert_eq!(first, bb_app::jobs::JobKind::daily().count());
        // A day later every kind runs once more, and none of them fails or leaves a dead job.
        rig.clock.advance(SECONDS_PER_DAY + 1);
        assert_eq!(
            rig.run_jobs().await?,
            bb_app::jobs::JobKind::daily().count()
        );
        let dead = rig.database.dead_jobs(10).await?;
        assert!(
            dead.iter().all(|job| job.state != JobState::Dead),
            "no job died"
        );
        Ok(())
    })
    .await
}

/// Switches e-mail on with a mail server, and signing up on.
async fn mail_and_sign_up_on(rig: &Rig) -> Result<(), Failure> {
    rig.turn_mail_on().await?;
    rig.set_organization_switch("auth.sign-up-enabled", true)
        .await
}

const EVE: &str = "eve@example.test";
const SIGN_UP: &str = "/api/v1/auth/sign-up";
const ACCEPT: &str = "/api/v1/auth/invitations/accept";

fn sign_up_body(address: &str) -> Value {
    json!({"email": address})
}

/// The secret in the link of the last e-mail.
fn secret_of_the_last_e_mail(rig: &Rig) -> Result<String, Failure> {
    let mails = rig.mailer.messages();
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

fn accept_body(secret: &str) -> Value {
    json!({"secret": secret, "name": "Eve Evans", "password": super::rig::PASSWORD})
}

#[tokio::test]
async fn auth_a_signed_up_account_works_only_after_the_link_when_mail_is_on() -> TestResult {
    each_database("auth_sign_up_link_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, _ada) = Site::start(&rig, &app).await?;
        mail_and_sign_up_on(&rig).await?;
        let mut eve = Browser::new(&app, &super::http::called());
        let asked = eve.send("POST", SIGN_UP, Some(sign_up_body(EVE))).await?;
        assert_eq!(asked.status, StatusCode::ACCEPTED, "{}", asked.body);
        assert_eq!(asked.body["check_your_mail"], true);
        assert!(asked.headers.get("set-cookie").is_none(), "no session");
        // Nothing is made yet: no password, no person.
        let refused = eve.sign_in(EVE, super::rig::PASSWORD).await?;
        assert_eq!(refused.code(), "auth.wrong-credentials");
        assert!(rig.account_by_email(EVE).await?.is_none());
        rig.run_jobs().await?;
        let secret = secret_of_the_last_e_mail(&rig)?;
        assert_eq!(rig.mailer.messages()[0].to, EVE);
        // The link goes through the invitation page: the person chooses a name and a password.
        let made = eve.send("POST", ACCEPT, Some(accept_body(&secret))).await?;
        assert_eq!(made.status, StatusCode::OK, "{}", made.body);
        assert!(rig.account_by_email(EVE).await?.is_some());
        let again = eve.send("POST", ACCEPT, Some(accept_body(&secret))).await?;
        assert_eq!(again.code(), "auth.invitation-invalid", "once");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_a_sign_up_for_an_address_with_an_account_answers_like_one_for_a_new_address()
-> TestResult {
    each_database("auth_sign_up_same_answer_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, _ada) = Site::start(&rig, &app).await?;
        mail_and_sign_up_on(&rig).await?;
        let mut visitor = Browser::new(&app, &super::http::called());
        let new = visitor
            .send("POST", SIGN_UP, Some(sign_up_body(EVE)))
            .await?;
        let known = visitor
            .send("POST", SIGN_UP, Some(sign_up_body(super::http::ADA_EMAIL)))
            .await?;
        assert_eq!(new.status, known.status);
        assert_eq!(new.body, known.body);
        // The same headers, whatever the request carried (the moment and the request ID differ).
        let kept = |reply: &Reply| {
            let mut headers: Vec<(String, String)> = reply
                .headers
                .iter()
                .filter(|(name, _)| !matches!(name.as_str(), "date" | "x-request-id"))
                .map(|(name, value)| {
                    (
                        name.as_str().to_owned(),
                        value.to_str().unwrap_or_default().to_owned(),
                    )
                })
                .collect();
            headers.sort();
            headers
        };
        assert_eq!(kept(&new), kept(&known));
        assert_eq!(
            new.body.to_string().as_bytes(),
            known.body.to_string().as_bytes(),
            "byte for byte"
        );
        // Only the e-mail differs: a link for the new one, the word that it has an account for
        // the other, and the person who has one gets no second account.
        rig.run_jobs().await?;
        let mails = rig.mailer.messages();
        assert_eq!(mails.len(), 2);
        let to_known = mails
            .iter()
            .find(|mail| mail.to == super::http::ADA_EMAIL)
            .ok_or("no e-mail")?;
        assert!(!to_known.text.contains("/invitation#"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_signing_up_has_a_small_limit_per_network_of_its_own() -> TestResult {
    each_database("sec_14_sign_up_limit_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, _ada) = Site::start(&rig, &app).await?;
        mail_and_sign_up_on(&rig).await?;
        let mut visitor = Browser::new(&app, &super::http::called());
        for number in 0..bb_api::limits::SIGN_UP_REQUESTS_PER_MINUTE {
            let reply = visitor
                .send(
                    "POST",
                    SIGN_UP,
                    Some(sign_up_body(&format!("n{number}@example.test"))),
                )
                .await?;
            assert_eq!(reply.status, StatusCode::ACCEPTED, "{}", reply.body);
        }
        let over = visitor
            .send("POST", SIGN_UP, Some(sign_up_body("late@example.test")))
            .await?;
        assert_eq!(over.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(over.code(), "rate-limit.exceeded");
        // The sign-in family keeps its own, larger cap.
        let refused = visitor.sign_in(EVE, super::rig::PASSWORD).await?;
        assert_eq!(refused.code(), "auth.wrong-credentials");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_the_link_runs_out_after_24_hours() -> TestResult {
    each_database("auth_link_expiry_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, _ada) = Site::start(&rig, &app).await?;
        mail_and_sign_up_on(&rig).await?;
        let mut eve = Browser::new(&app, &super::http::called());
        eve.send("POST", SIGN_UP, Some(sign_up_body(EVE))).await?;
        rig.run_jobs().await?;
        let secret = secret_of_the_last_e_mail(&rig)?;
        rig.clock.advance(24 * 3600 + 1);
        let late = eve.send("POST", ACCEPT, Some(accept_body(&secret))).await?;
        assert_eq!(late.code(), "auth.invitation-invalid");
        assert!(rig.account_by_email(EVE).await?.is_none());
        // The job that clears sessions removes the link, and signing up again sends a new one.
        rig.services
            .ctx(RequestInfo::default())
            .start_daily_jobs()
            .await
            .map_err(|error| error.to_string())?;
        rig.clock.advance(SECONDS_PER_DAY);
        rig.run_jobs().await?;
        let again = eve.send("POST", SIGN_UP, Some(sign_up_body(EVE))).await?;
        assert_eq!(again.status, StatusCode::ACCEPTED, "{}", again.body);
        rig.run_jobs().await?;
        let fresh = secret_of_the_last_e_mail(&rig)?;
        let made = eve.send("POST", ACCEPT, Some(accept_body(&fresh))).await?;
        assert_eq!(made.status, StatusCode::OK, "{}", made.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_over_http_the_background_header_counts_as_use_only_on_a_post() -> TestResult {
    each_database("sec_12_background_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        for person in [&mut ada, &mut bob] {
            person.set_header(bb_api::cookies::BACKGROUND_HEADER, "1");
        }
        // The idle time is 30 minutes. Twenty minutes in, both ask with the header.
        rig.clock.advance(20 * 60);
        let count = ada
            .send("GET", "/api/v1/notifications/unread", None)
            .await?;
        assert_eq!(count.status, StatusCode::OK, "{}", count.body);
        let read = bob
            .send(
                "POST",
                "/api/v1/notifications/read",
                Some(json!({"up_to": "01900000-0000-7000-8000-000000000000"})),
            )
            .await?;
        assert_eq!(read.status, StatusCode::NO_CONTENT, "{}", read.body);
        // Twenty minutes later, asking without the header: the read did not count as use, the
        // post did.
        rig.clock.advance(20 * 60);
        ada.clear_headers();
        bob.clear_headers();
        let gone = ada
            .send("GET", "/api/v1/notifications/unread", None)
            .await?;
        assert_eq!(gone.status, StatusCode::UNAUTHORIZED, "{}", gone.body);
        let kept = bob
            .send("GET", "/api/v1/notifications/unread", None)
            .await?;
        assert_eq!(kept.status, StatusCode::OK, "{}", kept.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_the_check_of_versions_in_a_tab_left_open_does_not_keep_a_session_alive()
-> TestResult {
    each_database("sec_12_versions_background", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        ada.set_header(bb_api::cookies::BACKGROUND_HEADER, "1");
        // A tab with an issue asks for the versions on its screen every half minute. Twenty
        // minutes in, the session stands and the check is answered ...
        rig.clock.advance(20 * 60);
        let check = ada
            .send(
                "POST",
                "/api/v1/versions/check",
                Some(json!({"records": []})),
            )
            .await?;
        assert_eq!(check.status, StatusCode::OK, "{}", check.body);
        // ... but it was no use: twenty minutes later the session is gone.
        rig.clock.advance(20 * 60);
        ada.clear_headers();
        let gone = ada
            .send("GET", "/api/v1/notifications/unread", None)
            .await?;
        assert_eq!(gone.status, StatusCode::UNAUTHORIZED, "{}", gone.body);
        Ok(())
    })
    .await
}
