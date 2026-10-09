//! Issue IDs over HTTP (ISS-01a, ADR-0021): counting and random codes, what happens to the old ones
//! when the mode of a project changes, and ten people making issues at the same moment.

use std::collections::BTreeSet;

use axum::http::StatusCode;
use bb_app::auth::AuthApi as _;
use bb_app::issues::MakeIssue;
use bb_core::event::{Actor, EntityKind, RecordKind};
use bb_core::issue_key::IssueKey;
use bb_core::technical::TechnicalName;
use bb_core::text::CROCKFORD;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, ok};
use super::rig::Rig;

const MODE_KEY: &str = "issues.id-mode";

/// Sets (or, with `None`, resets) the mode of issue IDs of the project ACME.
async fn mode(ada: &mut Browser, value: Option<&str>) -> Result<(), Failure> {
    let path = format!("/api/v1/projects/ACME/settings/{MODE_KEY}");
    let reply = match value {
        Some(value) => {
            ada.send("PUT", &path, Some(json!({"value": value})))
                .await?
        }
        None => ada.send("DELETE", &path, None).await?,
    };
    ok(&reply)?;
    Ok(())
}

/// The readable ID in an answer.
fn key_of(reply: &Value) -> Result<String, Failure> {
    Ok(reply["key"].as_str().ok_or("no issue ID")?.to_owned())
}

/// The code of an issue ID (`ACME-142` is `142`).
fn code_of(key: &Value) -> Result<String, Failure> {
    let key: IssueKey = key.as_str().ok_or("no issue ID")?.parse()?;
    Ok(key.code.to_string())
}

#[tokio::test]
async fn iss_01a_counting_ids_count_per_project_without_gaps_under_ten_writers() -> TestResult {
    each_database("iss_01a_ten_writers", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        mode(&mut ada, Some("counting")).await?;
        let other = ada
            .send(
                "POST",
                "/api/v1/projects",
                Some(json!({"short_name": "WEB", "name": "Website"})),
            )
            .await?;
        let web = ok(&other)?["id"].as_str().ok_or("no id")?.to_owned();
        ada.send(
            "PUT",
            &format!("/api/v1/projects/{web}/settings/{MODE_KEY}"),
            Some(json!({"value": "counting"})),
        )
        .await?;
        // Ten writers at the same moment in ACME, and one in WEB, which counts on its own.
        let mut writers = Vec::new();
        for number in 0..10 {
            let services = std::sync::Arc::clone(&rig.services);
            let (session, project, issue_type) = (
                site.org.admin.session.clone(),
                site.acme.clone(),
                site.bug.clone(),
            );
            writers.push(tokio::spawn(async move {
                services
                    .make_issue(
                        Rig::info(),
                        MakeIssue {
                            session_token: session,
                            project,
                            issue_type: issue_type.parse().map_err(|_| "no type")?,
                            title: format!("Writer {number}"),
                            description: None,
                            status: None,
                            priority: None,
                            assigned_to: None,
                        },
                    )
                    .await
                    .map_err(|error| error.to_string())
            }));
        }
        let mut codes = BTreeSet::new();
        for writer in writers {
            let made = writer.await??;
            let key = made.masked_value()?["key"].clone();
            assert!(codes.insert(code_of(&key)?.parse::<u32>()?), "a code twice");
        }
        assert_eq!(codes, (1..=10).collect::<BTreeSet<u32>>());
        let in_web = ada
            .send(
                "POST",
                &format!("/api/v1/projects/{web}/issues"),
                Some(site.new_issue("First in WEB")),
            )
            .await?;
        assert_eq!(ok(&in_web)?["key"], "WEB-1");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_01a_an_old_id_redirects_after_the_mode_changes() -> TestResult {
    each_database("iss_01a_old_id_redirects", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut first_keys = Vec::new();
        for title in ["One", "Two", "Three"] {
            let made = site.make(&mut ada, title).await?;
            first_keys.push(ok(&made)?["key"].as_str().ok_or("no key")?.to_owned());
        }
        // The organization starts as Random: the codes are letters and digits, not 1, 2, 3.
        for key in &first_keys {
            let code = key.strip_prefix("ACME-").ok_or("no short name")?;
            assert_eq!(code.len(), 5, "{key}");
            assert!(code.bytes().all(|byte| CROCKFORD.contains(&byte)), "{key}");
        }
        // Counting: every issue gets a number, and the random IDs keep opening the issues.
        mode(&mut ada, Some("counting")).await?;
        let mut counted = BTreeSet::new();
        for (key, title) in first_keys.iter().zip(["One", "Two", "Three"]) {
            let old = ada.send("GET", &format!("{ISSUES}/{key}"), None).await?;
            assert_eq!(ok(&old)?["issue"]["title"], title);
            let now = old.body["issue"]["key"].as_str().ok_or("no key")?;
            assert_ne!(now, key, "the answer holds the current ID");
            counted.insert(code_of(&json!(now))?);
            let by_number = ada.send("GET", &format!("{ISSUES}/{now}"), None).await?;
            assert_eq!(ok(&by_number)?["issue"]["title"], title);
        }
        assert_eq!(counted, BTreeSet::from(["1", "2", "3"].map(str::to_owned)));
        // A new issue continues the count.
        let fourth = site.make(&mut ada, "Four").await?;
        assert_eq!(ok(&fourth)?["key"], "ACME-4");
        // Back to the default of the organization (Random): all the IDs of the past still work, and
        // the numbers 1 to 4 are not given to anybody else when the project counts again.
        mode(&mut ada, None).await?;
        let renamed = ada.send("GET", &format!("{ISSUES}/ACME-4"), None).await?;
        assert_eq!(ok(&renamed)?["issue"]["title"], "Four");
        assert_ne!(renamed.body["issue"]["key"], "ACME-4");
        mode(&mut ada, Some("counting")).await?;
        let fifth = site.make(&mut ada, "Five").await?;
        let code: u32 = code_of(&ok(&fifth)?["key"])?.parse()?;
        assert!(code > 4, "{code} was given out before");
        for key in &first_keys {
            let still = ada.send("GET", &format!("{ISSUES}/{key}"), None).await?;
            assert_eq!(still.status, StatusCode::OK, "{key}");
        }
        // The short name can change too: the old one keeps the old IDs working.
        let project = ada.send("GET", "/api/v1/projects/ACME", None).await?;
        let version = ok(&project)?["project"]["version"].clone();
        let renamed = ada
            .send(
                "PATCH",
                &format!("/api/v1/projects/{}", site.acme),
                Some(json!({"version": version, "short_name": "ACMEX"})),
            )
            .await?;
        ok(&renamed)?;
        let across = ada
            .send("GET", &format!("{ISSUES}/{}", first_keys[0]), None)
            .await?;
        assert_eq!(ok(&across)?["issue"]["title"], "One");
        assert!(
            across.body["issue"]["key"]
                .as_str()
                .ok_or("no key")?
                .starts_with("ACMEX-")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_01a_random_ids_made_in_a_project_never_repeat() -> TestResult {
    each_database("iss_01a_random_never_repeat", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut keys = BTreeSet::new();
        for number in 0..60 {
            let made = site.make(&mut ada, &format!("Issue {number}")).await?;
            assert!(keys.insert(ok(&made)?["key"].to_string()), "an ID twice");
        }
        // A longer code is used when the organization asks for one.
        let length = ada
            .send(
                "PUT",
                "/api/v1/settings/issues.id-length",
                Some(json!({"value": 8})),
            )
            .await?;
        assert_eq!(length.status, StatusCode::OK, "{}", length.body);
        let long = site.make(&mut ada, "A long one").await?;
        let key = ok(&long)?["key"].as_str().ok_or("no key")?.to_owned();
        assert_eq!(key.len(), "ACME-".len() + 8, "{key}");
        let too_long = ada
            .send(
                "PUT",
                "/api/v1/settings/issues.id-length",
                Some(json!({"value": 9})),
            )
            .await?;
        assert_eq!(too_long.code(), "request.invalid");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_01a_a_trashed_issue_is_renewed_too_and_never_blocks_making_issues() -> TestResult {
    each_database("iss_01a_trashed_renewed", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let kept = key_of(ok(&site.make(&mut ada, "Kept").await?)?)?;
        let trashed = key_of(ok(&site.make(&mut ada, "Trashed").await?)?)?;
        let gone = ada
            .send("DELETE", &format!("{ISSUES}/{trashed}"), None)
            .await?;
        assert_eq!(gone.status, StatusCode::NO_CONTENT);
        mode(&mut ada, Some("counting")).await?;
        // The issue in the trash got a number as well, so no number it holds is given again.
        let fresh = site.make(&mut ada, "Fresh").await?;
        assert_eq!(ok(&fresh)?["key"], "ACME-3");
        // Taken out of the trash by the code it had, it comes back under its number.
        let back = ada
            .send("POST", &format!("{ISSUES}/{trashed}/restore"), None)
            .await?;
        let number = code_of(&ok(&back)?["key"])?;
        assert!(["1", "2"].contains(&number.as_str()), "{number}");
        assert_eq!(back.body["title"], "Trashed");
        let by_old = ada.send("GET", &format!("{ISSUES}/{kept}"), None).await?;
        assert_eq!(ok(&by_old)?["issue"]["title"], "Kept");
        // A random code, drawn when the project goes back to random, never looks like a number,
        // so it cannot meet a counting code of the past.
        mode(&mut ada, None).await?;
        let random = site.make(&mut ada, "Random again").await?;
        let code = code_of(&ok(&random)?["key"])?;
        assert!(code.parse::<u64>().is_err(), "{code}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_01a_a_change_of_the_mode_writes_one_event_for_each_issue_it_renews() -> TestResult {
    each_database("iss_01a_renewal_events", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut before = BTreeSet::new();
        for title in ["One", "Two", "Three"] {
            before.insert(key_of(ok(&site.make(&mut ada, title).await?)?)?);
        }
        // One of them is in the trash: it is renewed as well.
        let trashed = before.iter().next().ok_or("no issue")?.clone();
        ada.send("DELETE", &format!("{ISSUES}/{trashed}"), None)
            .await?;
        let consumer = TechnicalName::new("test-reader")?;
        let seen = rig.database.outbox_after(&consumer, 100_000).await?.len();
        mode(&mut ada, Some("counting")).await?;
        let events = rig.database.outbox_after(&consumer, 100_000).await?;
        let renewals: Vec<_> = events
            .iter()
            .skip(seen)
            .map(|(_, event)| event)
            .filter(|event| event.entity.kind == EntityKind::from(RecordKind::Issue))
            .collect();
        assert_eq!(renewals.len(), 3, "one for each issue");
        let mut was = BTreeSet::new();
        for event in &renewals {
            assert_eq!(event.actor, Actor::Person(site.org.admin.id));
            let [change] = event.changes.as_slice() else {
                return Err("one change for each event".into());
            };
            assert_eq!(change.field, "key");
            was.insert(
                change
                    .before
                    .as_ref()
                    .and_then(Value::as_str)
                    .ok_or("no old")?
                    .to_owned(),
            );
            let now = change
                .after
                .as_ref()
                .and_then(Value::as_str)
                .ok_or("no new")?;
            assert!(now.starts_with("ACME-") && !before.contains(now), "{now}");
        }
        assert_eq!(was, before);
        Ok(())
    })
    .await
}
