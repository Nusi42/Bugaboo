//! Issue IDs over HTTP (ISS-01a, ADR-0021): counting and random codes, what happens to the old ones
//! when the mode of a project changes, and ten people making issues at the same moment.

use std::collections::BTreeSet;

use axum::http::StatusCode;
use bb_app::auth::AuthApi as _;
use bb_app::context::SystemIds;
use bb_app::issues::MakeIssue;
use bb_app::jobs::JobStore as _;
use bb_core::bulk::{BULK_JOB_ABOVE, BULK_STEP};
use bb_core::event::{Actor, EntityKind, RecordKind};
use bb_core::id::JobId;
use bb_core::issue_key::IssueKey;
use bb_core::technical::TechnicalName;
use bb_core::text::CROCKFORD;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{ADA_EMAIL, Browser, rig_and_app};
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
                            fields: std::collections::BTreeMap::new(),
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
async fn iss_01a_an_old_id_keeps_working_after_the_issue_moves() -> TestResult {
    each_database("iss_01a_old_id_after_move", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let made = site.make(&mut ada, "Moves twice").await?;
        let (first, version) = (key_of(ok(&made)?)?, made.body["version"].clone());
        let other =
            |short_name: &'static str| json!({"short_name": short_name, "name": short_name});
        let mut projects = Vec::new();
        for short_name in ["BETA", "GAMMA"] {
            let project = ada
                .send("POST", "/api/v1/projects", Some(other(short_name)))
                .await?;
            projects.push(ok(&project)?["id"].clone());
        }
        // The first move gives the issue an ID of BETA; the ID of ACME still opens it.
        let moved = ada
            .send(
                "POST",
                &format!("{ISSUES}/{first}/move"),
                Some(json!({"project": projects[0], "version": version})),
            )
            .await?;
        let (second, version) = (key_of(ok(&moved)?)?, moved.body["version"].clone());
        assert!(second.starts_with("BETA-"), "{second}");
        // The second move gives it an ID of GAMMA; both earlier IDs still open it.
        let moved = ada
            .send(
                "POST",
                &format!("{ISSUES}/{second}/move"),
                Some(json!({"project": projects[1], "version": version})),
            )
            .await?;
        let third = key_of(ok(&moved)?)?;
        assert!(third.starts_with("GAMMA-"), "{third}");
        for old in [&first, &second, &third] {
            let found = ada.send("GET", &format!("{ISSUES}/{old}"), None).await?;
            assert_eq!(ok(&found)?["issue"]["title"], "Moves twice", "{old}");
            assert_eq!(found.body["issue"]["key"], third.as_str(), "{old}");
        }
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
            // The new key, and the version counted up (a stale change names who renewed the key).
            let [change, version] = event.changes.as_slice() else {
                return Err("the key and the version for each event".into());
            };
            assert_eq!(change.field, "key");
            assert_eq!(version.field, "version");
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

/// Changes the title of an issue five times, each time from the version it finds; the statuses.
async fn retitle(browser: &mut Browser, path: &str) -> Result<Vec<StatusCode>, Failure> {
    let mut seen = Vec::new();
    for round in 0..5 {
        let now = browser.send("GET", path, None).await?;
        let Some(version) = now.body["issue"]["version"].as_i64() else {
            seen.push(now.status);
            continue;
        };
        let changed = browser
            .send(
                "PATCH",
                path,
                Some(json!({"version": version, "title": format!("Round {round}")})),
            )
            .await?;
        seen.push(changed.status);
    }
    Ok(seen)
}

#[tokio::test]
async fn iss_01a_makes_and_changes_at_the_moment_of_a_mode_change_finish_in_one_mode() -> TestResult
{
    each_database("iss_01a_concurrent_mode_change", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut paths = Vec::new();
        for number in 0..4 {
            let made = site.make(&mut ada, &format!("Before {number}")).await?;
            paths.push(format!("{ISSUES}/{}", key_of(ok(&made)?)?));
        }
        // Four people change an issue each, four make issues, and the mode changes, all at once.
        let email = ADA_EMAIL;
        let (mut c0, mut c1, mut c2, mut c3) = (
            Browser::signed_in_as(&app, email).await?,
            Browser::signed_in_as(&app, email).await?,
            Browser::signed_in_as(&app, email).await?,
            Browser::signed_in_as(&app, email).await?,
        );
        let (mut m0, mut m1, mut m2, mut m3) = (
            Browser::signed_in_as(&app, email).await?,
            Browser::signed_in_as(&app, email).await?,
            Browser::signed_in_as(&app, email).await?,
            Browser::signed_in_as(&app, email).await?,
        );
        let (r0, r1, r2, r3, k0, k1, k2, k3, switched) = tokio::join!(
            retitle(&mut c0, &paths[0]),
            retitle(&mut c1, &paths[1]),
            retitle(&mut c2, &paths[2]),
            retitle(&mut c3, &paths[3]),
            site.make(&mut m0, "During 0"),
            site.make(&mut m1, "During 1"),
            site.make(&mut m2, "During 2"),
            site.make(&mut m3, "During 3"),
            mode(&mut ada, Some("counting")),
        );
        switched?;
        let mut statuses = Vec::new();
        for rounds in [r0, r1, r2, r3] {
            statuses.extend(rounds?);
        }
        // Nobody got an error of the server: a change may be stopped by a newer version, no more.
        for status in statuses {
            assert!(
                [StatusCode::OK, StatusCode::CONFLICT, StatusCode::NOT_FOUND].contains(&status),
                "{status}"
            );
        }
        for made in [k0, k1, k2, k3] {
            assert_eq!(made?.status, StatusCode::OK);
        }
        // Every issue ends with a counting number, and no number twice.
        let list = ada
            .send("GET", "/api/v1/projects/ACME/issues?fields=key", None)
            .await?;
        let items = ok(&list)?["items"].as_array().ok_or("no items")?;
        assert_eq!(items.len(), 8);
        let mut numbers = BTreeSet::new();
        for item in items {
            let number: u32 = code_of(&item["key"])?.parse()?;
            assert!(numbers.insert(number), "{number} twice");
        }
        assert_eq!(numbers, (1..=8).collect::<BTreeSet<u32>>());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_01a_a_second_switch_to_counting_continues_the_counter() -> TestResult {
    each_database("iss_01a_second_switch", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let first = key_of(ok(&site.make(&mut ada, "First").await?)?)?;
        mode(&mut ada, Some("counting")).await?;
        let opened = ada.send("GET", &format!("{ISSUES}/ACME-1"), None).await?;
        assert_eq!(ok(&opened)?["issue"]["title"], "First");
        let second = site.make(&mut ada, "Second").await?;
        assert_eq!(ok(&second)?["key"], "ACME-2");
        mode(&mut ada, None).await?;
        mode(&mut ada, Some("counting")).await?;
        // The counter continues; the first issue keeps being found by every ID it ever had.
        let third = site.make(&mut ada, "Third").await?;
        let number: u32 = code_of(&ok(&third)?["key"])?.parse()?;
        assert!(number >= 3, "{number}");
        for address in ["ACME-1", &first] {
            let found = ada
                .send("GET", &format!("{ISSUES}/{address}"), None)
                .await?;
            assert_eq!(ok(&found)?["issue"]["title"], "First", "{address}");
        }
        Ok(())
    })
    .await
}

/// How many issues of ACME still have a counting ID, read page by page.
async fn counting_ids(ada: &mut Browser) -> Result<usize, Failure> {
    let (mut counting, mut cursor) = (0, String::new());
    loop {
        let page = ada
            .send(
                "GET",
                &format!("/api/v1/projects/ACME/issues?limit=100{cursor}"),
                None,
            )
            .await?;
        counting += ok(&page)?["items"].as_array().map_or(0, |items| {
            items
                .iter()
                .filter(|item| {
                    item["key"]
                        .as_str()
                        .and_then(|key| key.split('-').nth(1))
                        .is_some_and(|code| code.bytes().all(|byte| byte.is_ascii_digit()))
                })
                .count()
        });
        match page.body["next_cursor"].as_str() {
            Some(next) => cursor = format!("&cursor={next}"),
            None => return Ok(counting),
        }
    }
}

/// The step is idempotent (ADR-0017): a second job for a renewal that is done, queued by a worker
/// that was cut off before it reported, finds nothing to do and writes no event.
async fn a_done_renewal_does_nothing(
    rig: &Rig,
    site: &Site<'_>,
    ada: &mut Browser,
    old: &str,
    consumer: &TechnicalName,
) -> Result<(), Failure> {
    let key_now =
        ok(&ada.send("GET", &format!("{ISSUES}/{old}"), None).await?)?["issue"]["key"].clone();
    let events_before = rig.database.outbox_after(consumer, 1_000_000).await?.len();
    let now = rig.clock_now();
    let by = rig.person_id(ADA_EMAIL).await?.to_string();
    rig.storage
        .queue_job(
            JobId::generate(&mut SystemIds)?,
            &TechnicalName::new(bb_app::jobs::kind::RENEW_IDS)?,
            &json!({"project": site.acme, "by": by}),
            now,
            now,
        )
        .await?;
    let taken = rig
        .services
        .ctx(Rig::info())
        .run_due_jobs()
        .await
        .map_err(|error| error.to_string())?;
    assert_eq!(taken, 1);
    let events_after = rig.database.outbox_after(consumer, 1_000_000).await?.len();
    assert_eq!(
        events_after, events_before,
        "no event for a renewal that is done"
    );
    let again = ada.send("GET", &format!("{ISSUES}/{old}"), None).await?;
    assert_eq!(ok(&again)?["issue"]["key"], key_now);
    Ok(())
}

#[tokio::test]
async fn iss_01a_a_big_project_gets_new_ids_a_step_at_a_time_in_the_background() -> TestResult {
    each_database("iss_01a_renewal_job", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        mode(&mut ada, Some("counting")).await?;
        let total = BULK_JOB_ABOVE + 101;
        let mut old = Vec::new();
        for number in 0..total {
            let made = rig
                .api()
                .make_issue(
                    Rig::info(),
                    MakeIssue {
                        session_token: site.org.admin.session.clone(),
                        project: site.acme.clone(),
                        issue_type: site.bug.parse().map_err(|_| "no type")?,
                        title: format!("Issue {number}"),
                        description: None,
                        status: None,
                        priority: None,
                        assigned_to: None,
                        fields: std::collections::BTreeMap::new(),
                    },
                )
                .await
                .map_err(|error| error.to_string())?;
            old.push(
                made.masked_value()?["key"]
                    .as_str()
                    .ok_or("no key")?
                    .to_owned(),
            );
        }
        // Changing the mode renews a first step now and leaves the rest to a job.
        let consumer = TechnicalName::new("test-reader")?;
        let seen = rig.database.outbox_after(&consumer, 1_000_000).await?.len();
        mode(&mut ada, Some("random")).await?;
        let counting = counting_ids(&mut ada).await?;
        assert_eq!(
            counting,
            total - BULK_STEP,
            "a step is renewed in the change itself"
        );
        // The job goes on until none is left.
        for _ in 0..40 {
            let taken = rig
                .services
                .ctx(Rig::info())
                .run_due_jobs()
                .await
                .map_err(|error| error.to_string())?;
            if taken == 0 {
                break;
            }
        }
        let counting = counting_ids(&mut ada).await?;
        assert_eq!(counting, 0, "every issue has a random ID in the end");
        // The project says once that its IDs were renewed, not once for each step.
        let events = rig.database.outbox_after(&consumer, 1_000_000).await?;
        let project_events = events
            .iter()
            .skip(seen)
            .filter(|(_, event)| event.entity.kind == EntityKind::from(RecordKind::Project))
            .filter(|(_, event)| {
                event
                    .changes
                    .iter()
                    .any(|change| change.field == "issue-ids")
            })
            .count();
        assert_eq!(project_events, 1);
        // An old ID still finds its issue.
        let found = ada
            .send("GET", &format!("{ISSUES}/{}", old[0]), None)
            .await?;
        assert_eq!(ok(&found)?["issue"]["title"], "Issue 0");
        a_done_renewal_does_nothing(&rig, &site, &mut ada, &old[0], &consumer).await?;
        Ok(())
    })
    .await
}
