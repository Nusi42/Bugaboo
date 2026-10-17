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
        assert_eq!(entry["kind"], "followed_activity");
        assert_eq!(entry["read"], false);
        assert_eq!(entry["field"], "status");
        assert_eq!(entry["after_text"], "done");
        assert_eq!(entry["issue"]["key"], issue.as_str());
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
            .send("GET", &format!("{NOTIFICATIONS}?filter=unread"), None)
            .await?;
        assert!(items(&unread_only).is_empty());
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
        let set = ada
            .send(
                "PUT",
                "/api/v1/mail/password",
                Some(json!({"password": password})),
            )
            .await?;
        assert_eq!(set.status, StatusCode::NO_CONTENT, "{}", set.body);
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
        assert_eq!(first, bb_app::jobs::kind::DAILY_JOBS.len());
        // A day later every kind runs once more, and none of them fails or leaves a dead job.
        rig.clock.advance(SECONDS_PER_DAY + 1);
        assert_eq!(rig.run_jobs().await?, bb_app::jobs::kind::DAILY_JOBS.len());
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
const ACCEPT: &str = "/api/v1/auth/invitation/accept";

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
    json!({"secret": secret, "display_name": "Eve Evans", "password": super::rig::PASSWORD})
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
        assert_eq!(refused.code(), "auth.wrong_credentials");
        assert!(rig.account_by_email(EVE).await?.is_none());
        rig.run_jobs().await?;
        let secret = secret_of_the_last_e_mail(&rig)?;
        assert_eq!(rig.mailer.messages()[0].to, EVE);
        // The link goes through the invitation page: the person chooses a name and a password.
        let made = eve.send("POST", ACCEPT, Some(accept_body(&secret))).await?;
        assert_eq!(made.status, StatusCode::OK, "{}", made.body);
        assert!(rig.account_by_email(EVE).await?.is_some());
        let again = eve.send("POST", ACCEPT, Some(accept_body(&secret))).await?;
        assert_eq!(again.code(), "auth.invitation_invalid", "once");
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
        assert_eq!(late.code(), "auth.invitation_invalid");
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
