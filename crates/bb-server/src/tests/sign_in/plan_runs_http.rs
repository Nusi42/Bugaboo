//! A move of the dates of several issues is one run, written whole or not at all (PLAN-03,
//! PLAN-13, TEST-CONCURRENT, ADR-0043 decision 6): a run in which one issue is refused writes
//! nothing and records no event, for *Move it after* too, and runs sent at the same moments are
//! each written whole or refused whole. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};
use tokio::task::JoinSet;

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, ok};
use super::plan_site::{MONDAY, change, id_of_key, issue, moves_of, planned, previewed_move_after};

/// The route that moves the dates of several issues in one run.
const MOVE_DATES: &str = "/api/v1/issues/move-dates";
/// How many people send runs at the same moments.
const PEOPLE: usize = 10;

/// The version and the dates of each issue, as `browser` reads them.
async fn states(browser: &mut Browser, keys: &[String]) -> Result<Vec<Value>, Failure> {
    let mut found = Vec::new();
    for key in keys {
        let read = issue(browser, key).await?;
        found.push(json!([
            read["version"],
            read["start_date"],
            read["due_date"]
        ]));
    }
    Ok(found)
}

/// How many changes the history of an issue has.
async fn history_length(browser: &mut Browser, key: &str) -> Result<usize, Failure> {
    let reply = browser
        .send("GET", &format!("{ISSUES}/{key}/history?tab=changes"), None)
        .await?;
    Ok(ok(&reply)?["items"].as_array().map_or(0, Vec::len))
}

/// A run that moves each issue to these dates, on the versions given.
fn run_of(issues: &[(String, i64)], dates: (&str, &str)) -> Value {
    let moves: Vec<Value> = issues
        .iter()
        .map(|(id, version)| {
            json!({"issue": id, "version": version, "start_date": dates.0, "due_date": dates.1})
        })
        .collect();
    json!({ "issues": moves })
}

#[tokio::test]
async fn plan_03_a_run_with_one_issue_changed_meanwhile_writes_nothing() -> TestResult {
    each_database("plan_03_run_refused_whole", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut keys = Vec::new();
        for (title, due) in [
            ("API", "2027-03-03"),
            ("UI", "2027-03-04"),
            ("Docs", "2027-03-05"),
        ] {
            let members = json!({"start_date": MONDAY, "due_date": due});
            keys.push(planned(&site, &mut ada, title, members).await?);
        }
        let mut issues = Vec::new();
        for key in &keys {
            issues.push((id_of_key(&mut ada, key).await?, 1));
        }
        // Ben moves the third while Ada's run is on its way.
        let (mut ben, _) = site.person(StartingRole::TeamMember, "ben").await?;
        ok(&change(&mut ben, &keys[2], json!({"due_date": "2027-03-19"})).await?)?;
        let before = states(&mut ada, &keys).await?;
        let mut histories = Vec::new();
        for key in &keys {
            histories.push(history_length(&mut ada, key).await?);
        }
        let run = run_of(&issues, ("2027-03-08", "2027-03-10"));
        let refused = ada.send("POST", MOVE_DATES, Some(run)).await?;
        assert_eq!(refused.status, StatusCode::CONFLICT, "{}", refused.body);
        assert_eq!(refused.code(), "edit.version-changed");
        // The refusal is about the third issue: its version and the due date Ben gave it.
        assert_eq!(refused.body["conflict"]["version"], 2, "{}", refused.body);
        assert_eq!(
            refused.body["conflict"]["current"]["due-date"], "2027-03-19",
            "{}",
            refused.body
        );
        // Nothing of the run was written: no date, no version, no event.
        assert_eq!(states(&mut ada, &keys).await?, before);
        for (key, length) in keys.iter().zip(histories) {
            assert_eq!(history_length(&mut ada, key).await?, length, "{key}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_13_move_it_after_with_an_issue_changed_meanwhile_writes_nothing() -> TestResult {
    each_database("plan_13_move_after_refused_whole", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (then, after, preview) = previewed_move_after(&site, &mut ada).await?;
        let moves = moves_of(&preview);
        // Somebody changes the last issue the move moves, after the preview.
        let (mut ben, _) = site.person(StartingRole::TeamMember, "ben").await?;
        ok(&change(&mut ben, &after, json!({"due_date": "2027-03-20"})).await?)?;
        let keys = [then.clone(), after.clone()];
        let before = states(&mut ada, &keys).await?;
        let then_history = history_length(&mut ada, &then).await?;
        let refused = ada
            .send("POST", MOVE_DATES, Some(json!({ "issues": moves })))
            .await?;
        assert_eq!(refused.code(), "edit.version-changed", "{}", refused.body);
        assert_eq!(
            refused.body["conflict"]["current"]["due-date"], "2027-03-20",
            "{}",
            refused.body
        );
        assert_eq!(states(&mut ada, &keys).await?, before);
        assert_eq!(history_length(&mut ada, &then).await?, then_history);
        Ok(())
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn plan_03_runs_sent_at_the_same_moments_are_each_written_whole_or_not_at_all() -> TestResult
{
    each_database("plan_03_runs_at_once", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut keys = Vec::new();
        let mut ids = Vec::new();
        for title in ["One", "Two", "Three"] {
            let members = json!({"start_date": MONDAY, "due_date": "2027-03-03"});
            let key = planned(&site, &mut ada, title, members).await?;
            ids.push(id_of_key(&mut ada, &key).await?);
            keys.push(key);
        }
        let mut people = Vec::new();
        for number in 0..PEOPLE {
            let name = format!("mover{number}");
            people.push(site.person(StartingRole::TeamMember, &name).await?.0);
        }
        // Each sends one run of the three issues on version 1, in an order of its own, at once.
        let mut sending = JoinSet::new();
        for (number, mut browser) in people.into_iter().enumerate() {
            let mut issues: Vec<(String, i64)> = ids.iter().map(|id| (id.clone(), 1)).collect();
            let turn = number % issues.len();
            issues.rotate_left(turn);
            sending.spawn(async move {
                let run = run_of(&issues, ("2027-03-08", "2027-03-10"));
                let reply = browser.send("POST", MOVE_DATES, Some(run)).await?;
                Ok::<bool, Failure>(reply.status.is_success())
            });
        }
        let mut kept: i64 = 0;
        while let Some(done) = sending.join_next().await {
            if done?? {
                kept += 1;
            }
        }
        // All on version 1: at most one is kept, whole; the others wrote nothing, so every issue
        // has exactly one change more for each run kept.
        assert!(kept <= 1, "{kept} runs kept");
        for key in &keys {
            let read = issue(&mut ada, key).await?;
            assert_eq!(read["version"], 1 + kept, "{key}: {read}");
        }
        Ok(())
    })
    .await
}
