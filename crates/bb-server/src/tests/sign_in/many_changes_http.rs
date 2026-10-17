//! Changing many issues at once over HTTP (ISS-09, PRIV-13): the preview says what then happens to
//! each issue, an issue the person cannot change is skipped with its reason, an issue that changed
//! after the preview is named and left alone, a move skips the issues the other project cannot
//! take, and more than a thousand issues run as a job. Every flow runs on SQLite and PostgreSQL.

use std::collections::BTreeMap;

use bb_app::access::StartingRole;
use bb_app::issues::CreateIssue;
use bb_core::many_changes::MANY_CHANGES_JOB_ABOVE;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::rig_and_app;
use super::issues_site::{ISSUES, PROJECTS, Site, key_of, ok};
use super::rig::Rig;

const PREVIEW: &str = "/api/v1/many-changes/preview";
const CHANGES: &str = "/api/v1/many-changes";
const RUNS: &str = "/api/v1/many-changes/runs";

fn done() -> Value {
    json!({"kind": "change", "change": {"status": "done"}})
}

/// What the preview says about each issue: whether it is skipped and why.
fn skips(body: &Value, member: &str) -> Vec<Option<String>> {
    body[member]
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| row["skipped"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// The issues of a preview with the versions it read, as a change takes them.
fn targets(preview: &Value) -> Vec<Value> {
    preview["rows"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter(|row| row["version"].is_i64())
                .map(|row| json!({"issue": row["address"], "version": row["version"]}))
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn iss_09_the_preview_equals_what_happens() -> TestResult {
    each_database("iss_09_preview_equals_change", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut issues = Vec::new();
        for title in ["One", "Two", "Three"] {
            issues.push(key_of(&site.create(&mut ada, title).await?)?);
        }
        // The middle one is done already: the change has nothing to do there.
        ok(&ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{}", issues[1]),
                Some(json!({"version": 1, "status": "done"})),
            )
            .await?)?;
        let preview = ada
            .send(
                "POST",
                PREVIEW,
                Some(json!({"issues": issues, "action": done()})),
            )
            .await?;
        let preview = ok(&preview)?.clone();
        assert_eq!(preview["summary"], json!({"will_change": 2, "skipped": 1}));
        assert_eq!(
            skips(&preview, "rows"),
            vec![None, Some("nothing-to-change".to_owned()), None]
        );
        let changed = ada
            .send(
                "POST",
                CHANGES,
                Some(json!({"issues": targets(&preview), "action": done()})),
            )
            .await?;
        let result = ok(&changed)?.clone();
        assert_eq!(skips(&result, "outcomes"), skips(&preview, "rows"));
        for issue in &issues {
            let read = ada.send("GET", &format!("{ISSUES}/{issue}"), None).await?;
            assert_eq!(ok(&read)?["issue"]["status"]["key"], "done", "{issue}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_09_an_issue_you_cannot_change_is_skipped_with_its_reason() -> TestResult {
    each_database("iss_09_skipped_with_reason", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Ada's issue").await?)?;
        let secret = ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "SECRET", "name": "Secret"})),
            )
            .await?;
        let secret_id = ok(&secret)?["id"].as_str().ok_or("no ID")?.to_owned();
        let hidden = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{secret_id}/issues"),
                Some(site.new_issue("Hidden")),
            )
            .await?;
        let hidden = key_of(&hidden)?;
        // A viewer of ACME sees the issue and may not change it; the other project is not there.
        let (mut viewer, _) = site.person_in_acme(StartingRole::Viewer, "vera").await?;
        let asked = viewer
            .send(
                "POST",
                PREVIEW,
                Some(json!({"issues": [issue, hidden], "action": done()})),
            )
            .await?;
        let body = ok(&asked)?.clone();
        assert_eq!(
            skips(&body, "rows"),
            vec![Some("not-allowed".to_owned()), Some("not-found".to_owned())],
            "{body}"
        );
        // The hidden issue is not named.
        assert!(body["rows"][1].get("issue").is_none());
        assert!(body["rows"][1].get("version").is_none());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_09_an_issue_changed_after_the_preview_is_left_alone_and_named() -> TestResult {
    each_database("iss_09_changed_after_preview", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (first, second) = (
            key_of(&site.create(&mut ada, "First").await?)?,
            key_of(&site.create(&mut ada, "Second").await?)?,
        );
        let preview = ada
            .send(
                "POST",
                PREVIEW,
                Some(json!({"issues": [first, second], "action": done()})),
            )
            .await?;
        let preview = ok(&preview)?.clone();
        // Somebody changes the first issue meanwhile.
        ok(&ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{first}"),
                Some(json!({"version": 1, "title": "Changed meanwhile"})),
            )
            .await?)?;
        let changed = ada
            .send(
                "POST",
                CHANGES,
                Some(json!({"issues": targets(&preview), "action": done()})),
            )
            .await?;
        let result = ok(&changed)?.clone();
        assert_eq!(
            skips(&result, "outcomes"),
            vec![Some("changed-since".to_owned()), None]
        );
        // The skipped issue is named, and left as it is.
        assert_eq!(result["outcomes"][0]["issue"]["key"], first);
        let left = ada.send("GET", &format!("{ISSUES}/{first}"), None).await?;
        assert_eq!(ok(&left)?["issue"]["status"]["key"], "open");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_09_a_move_skips_issues_whose_type_the_target_lacks() -> TestResult {
    each_database("iss_09_move_skips_type", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let other = ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "INTO", "name": "Into"})),
            )
            .await?;
        ok(&other)?;
        let types = ada.send("GET", "/api/v1/issue-types", None).await?;
        let task = ok(&types)?["items"][1]["id"]
            .as_str()
            .ok_or("no second issue type")?
            .to_owned();
        let bug = key_of(&site.create(&mut ada, "A bug").await?)?;
        let made = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{}/issues", site.acme),
                Some(json!({"issue_type": task, "title": "A task"})),
            )
            .await?;
        let task_issue = key_of(&made)?;
        // The second type is retired: it is offered nowhere.
        ok(&ada
            .send(
                "PATCH",
                &format!("/api/v1/issue-types/{task}"),
                Some(json!({"version": 1, "retired": true})),
            )
            .await?)?;
        let action = json!({"kind": "move", "project": "INTO"});
        let preview = ada
            .send(
                "POST",
                PREVIEW,
                Some(json!({"issues": [bug, task_issue], "action": action})),
            )
            .await?;
        let preview = ok(&preview)?.clone();
        assert_eq!(
            skips(&preview, "rows"),
            vec![None, Some("type-missing".to_owned())]
        );
        let moved = ada
            .send(
                "POST",
                CHANGES,
                Some(json!({"issues": targets(&preview), "action": action})),
            )
            .await?;
        assert_eq!(
            skips(ok(&moved)?, "outcomes"),
            vec![None, Some("type-missing".to_owned())]
        );
        // The bug has an ID of the new project, and the old one still finds it.
        let by_old = ada.send("GET", &format!("{ISSUES}/{bug}"), None).await?;
        assert!(
            ok(&by_old)?["issue"]["key"]
                .as_str()
                .is_some_and(|key| key.starts_with("INTO-"))
        );
        Ok(())
    })
    .await
}

/// Issues in ACME, one more than a change takes at once, each as the body of a change names it
/// (the issue and the version the preview read): what it takes to start a run in the background.
pub(super) async fn targets_for_a_job(rig: &Rig, site: &Site<'_>) -> Result<Vec<Value>, Failure> {
    let mut targets = Vec::new();
    for number in 0..=MANY_CHANGES_JOB_ABOVE {
        let made = rig
            .api()
            .create_issue(
                Rig::info(),
                CreateIssue {
                    session_token: site.org.admin.session.clone(),
                    project: site.acme.clone(),
                    issue_type: site.bug.parse().map_err(|_| "no type ID")?,
                    title: format!("Issue {number}"),
                    description: None,
                    status: None,
                    priority: None,
                    assigned_to: None,
                    fields: BTreeMap::new(),
                },
            )
            .await
            .map_err(|error| error.to_string())?
            .into_value();
        targets.push(json!({"issue": made.id.to_string(), "version": made.version}));
    }
    Ok(targets)
}

#[tokio::test]
async fn iss_09_more_than_a_thousand_run_as_a_job() -> TestResult {
    each_database("iss_09_job", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let targets = targets_for_a_job(&rig, &site).await?;
        let started = ada
            .send(
                "POST",
                CHANGES,
                Some(json!({"issues": targets, "action": done()})),
            )
            .await?;
        let run = ok(&started)?["run"].clone();
        assert_eq!(run["status"], "queued");
        assert_eq!(run["total"], 1001);
        let id = run["id"].as_str().ok_or("no run ID")?.to_owned();
        // A worker takes the steps.
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
        let read = ada.send("GET", &format!("{RUNS}/{id}"), None).await?;
        let run = ok(&read)?;
        assert_eq!(run["status"], "done", "{run}");
        assert_eq!(
            (run["done"].clone(), run["changed"].clone()),
            (json!(1001), json!(1001))
        );
        // Nobody else reads the run.
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let hidden = bob.send("GET", &format!("{RUNS}/{id}"), None).await?;
        assert_eq!(hidden.code(), "resource.not_found");
        Ok(())
    })
    .await
}
