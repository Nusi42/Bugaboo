//! The exit review of M3 (WP-12), changes to many issues that go on in the background (ISS-09,
//! SEC-12): the person who asked, and an administrator, can stop a run, and signing out everywhere
//! or changing the password stops the runs of the person. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::{Permission, StartingRole};
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::exit_masks::person_with_levels;
use super::http::{Browser, rig_and_app};
use super::issues_site::Site;
use super::issues_site::ok;
use super::many_changes_http::targets_for_a_job;
use super::rig::{OTHER_PASSWORD, PASSWORD, Rig};

const CHANGES: &str = "/api/v1/many-changes";
const RUNS: &str = "/api/v1/many-changes/runs";

/// Starts a run in the background as `person`: the ID of the run.
async fn start(person: &mut Browser, targets: &[Value]) -> Result<String, Failure> {
    let started = person
        .send(
            "POST",
            CHANGES,
            Some(json!({
                "issues": targets,
                "action": {"kind": "change", "change": {"status": "done"}},
            })),
        )
        .await?;
    let run = &ok(&started)?["run"];
    assert_eq!(run["status"], "queued", "{run}");
    Ok(run["id"].as_str().ok_or("no run ID")?.to_owned())
}

/// A worker takes the steps that are due.
async fn work(rig: &Rig) -> Result<(), Failure> {
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
    Ok(())
}

/// The run as `person` reads it.
async fn read(person: &mut Browser, run: &str) -> Result<Value, Failure> {
    let read = person.send("GET", &format!("{RUNS}/{run}"), None).await?;
    Ok(ok(&read)?.clone())
}

/// Nothing was changed and the run says it stopped.
fn assert_stopped(run: &Value) {
    assert_eq!(run["status"], "stopped", "{run}");
    assert_eq!(run["changed"], 0, "{run}");
    assert_eq!(run["done"], 0, "{run}");
}

#[tokio::test]
async fn iss_09_a_run_is_stopped_by_who_asked_and_by_an_administrator_and_by_nobody_else()
-> TestResult {
    each_database("iss_09_cancel_run", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let targets = targets_for_a_job(&rig, &site).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let (mut cat, _) = site.person(StartingRole::Contributor, "cat").await?;
        let cancel = |run: &str| format!("{RUNS}/{run}/stop");
        let first = start(&mut bob, &targets).await?;
        // Somebody else, who is no administrator, does not find the run.
        let hidden = cat.send("POST", &cancel(&first), None).await?;
        assert_eq!(hidden.code(), "resource.not_found", "{}", hidden.body);
        assert_eq!(read(&mut bob, &first).await?["status"], "queued");
        // An administrator, who manages people, stops it.
        let stopped = ada.send("POST", &cancel(&first), None).await?;
        assert_eq!(stopped.status, StatusCode::OK, "{}", stopped.body);
        assert_eq!(
            stopped.body,
            json!({"id": first, "status": "stopped"}),
            "only the status"
        );
        // Stopped is stopped: the worker does not go on, and the person reads it.
        work(&rig).await?;
        assert_stopped(&read(&mut bob, &first).await?);
        // Stopping again changes nothing.
        let again = bob.send("POST", &cancel(&first), None).await?;
        assert_eq!(again.status, StatusCode::OK, "{}", again.body);
        // Whoever manages people stops the run of another person, whoever only manages roles does
        // not find it.
        let (mut ted, _) = person_with_levels(
            &site,
            &rig,
            "ted",
            &[Permission::SeeProjects, Permission::ManagePeople],
            &[],
        )
        .await?;
        let (mut ron, _) = person_with_levels(
            &site,
            &rig,
            "ron",
            &[Permission::SeeProjects, Permission::ManageRoles],
            &[],
        )
        .await?;
        let third = start(&mut bob, &targets).await?;
        let no = ron.send("POST", &cancel(&third), None).await?;
        assert_eq!(no.code(), "resource.not_found", "{}", no.body);
        let yes = ted.send("POST", &cancel(&third), None).await?;
        assert_eq!(yes.status, StatusCode::OK, "{}", yes.body);
        assert_eq!(yes.body["status"], "stopped");
        // The person who asked stops their own.
        let second = start(&mut bob, &targets).await?;
        let own = bob.send("POST", &cancel(&second), None).await?;
        assert_eq!(own.status, StatusCode::OK, "{}", own.body);
        work(&rig).await?;
        assert_stopped(&read(&mut bob, &second).await?);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_signing_out_everywhere_stops_the_runs_of_the_person() -> TestResult {
    each_database("sec_12_sign_out_stops_runs", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let targets = targets_for_a_job(&rig, &site).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let run = start(&mut bob, &targets).await?;
        let out = bob.send("DELETE", "/api/v1/auth/sessions", None).await?;
        assert_eq!(out.status, StatusCode::NO_CONTENT, "{}", out.body);
        work(&rig).await?;
        let mut again = Browser::signed_in_as(&app, "bob@example.test").await?;
        assert_stopped(&read(&mut again, &run).await?);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_changing_the_password_stops_the_runs_of_the_person() -> TestResult {
    each_database("sec_12_password_stops_runs", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let targets = targets_for_a_job(&rig, &site).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let run = start(&mut bob, &targets).await?;
        let changed = bob
            .send(
                "POST",
                "/api/v1/auth/password",
                Some(json!({"current_password": PASSWORD, "new_password": OTHER_PASSWORD})),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        work(&rig).await?;
        assert_stopped(&read(&mut bob, &run).await?);
        Ok(())
    })
    .await
}
