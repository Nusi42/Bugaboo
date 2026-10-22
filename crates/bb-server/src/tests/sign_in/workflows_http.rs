//! Workflows over HTTP on both databases (WF-01…WF-06, ADR-0031): a new organization starts with
//! three workflows, a status change the workflow of the issue does not have is refused, publishing
//! a workflow without a status moves the issues that were in it, and the roles that may change a
//! field are told to those who may know them.

use axum::http::StatusCode;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, id_in, key_of, ok, text_in};
use super::layouts_http::{AN_EDITOR, LOCKS};

const WORKFLOWS: &str = "/api/v1/workflows";

/// The ID of the issue type with this name.
async fn type_named(ada: &mut Browser, name: &str) -> Result<String, Failure> {
    let types = ada.send("GET", "/api/v1/issue-types", None).await?;
    ok(&types)?["items"]
        .as_array()
        .and_then(|items| items.iter().find(|each| each["name"] == name))
        .map(id_in)
        .transpose()?
        .ok_or_else(|| format!("no issue type {name}").into())
}

/// Changes the status of an issue; the reply.
async fn go(ada: &mut Browser, issue: &str, status: &str) -> Result<super::http::Reply, Failure> {
    let path = format!("{ISSUES}/{issue}");
    let version = ok(&ada.send("GET", &path, None).await?)?["issue"]["version"].clone();
    ada.send(
        "PATCH",
        &path,
        Some(json!({"version": version, "status": status})),
    )
    .await
}

/// The lock of the editor of the workflow, taken by `who`: its token.
pub(super) async fn lock_workflow(who: &mut Browser, workflow: &str) -> Result<Value, Failure> {
    let lock = who
        .send(
            "POST",
            LOCKS,
            Some(json!({
                "what": {"record": {"kind": "workflow", "id": workflow}, "part": "workflow"},
                "editor_id": AN_EDITOR,
            })),
        )
        .await?;
    Ok(ok(&lock)?["lock_token"].clone())
}

#[tokio::test]
async fn wf_06_a_new_organization_offers_the_three_starter_workflows_over_http() -> TestResult {
    each_database("wf_06_starters_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_, mut ada) = Site::start(&rig, &app).await?;
        let listed = ada.send("GET", WORKFLOWS, None).await?;
        let names: Vec<&str> = ok(&listed)?["workflows"]
            .as_array()
            .ok_or("no workflows")?
            .iter()
            .filter_map(|each| each["name"].as_str())
            .collect();
        for wanted in ["Simple", "Bug", "Development"] {
            assert!(
                names.contains(&wanted),
                "{wanted} is missing from {names:?}"
            );
        }
        // A Bug goes through testing; a Task can go anywhere.
        let bug = type_named(&mut ada, "Bug").await?;
        let made = ada
            .send(
                "POST",
                &format!("{PROJECTS}/ACME/issues"),
                Some(json!({"issue_type": bug, "title": "A bug"})),
            )
            .await?;
        let issue = key_of(&made)?;
        let straight = go(&mut ada, &issue, "done").await?;
        assert_eq!(
            (straight.status, straight.code()),
            (StatusCode::CONFLICT, "workflow.no_such_change")
        );
        let offered: Vec<String> = ok(&ada
            .send("GET", &format!("{ISSUES}/{issue}"), None)
            .await?)?["issue"]["status_changes"]
            .as_array()
            .ok_or("no status changes")?
            .iter()
            .filter_map(|each| each["to"]["key"].as_str().map(str::to_owned))
            .collect();
        assert_eq!(offered, ["in-progress", "wont-fix"]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn wf_02_publishing_without_a_status_moves_its_issues_over_http() -> TestResult {
    each_database("wf_02_publish_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let made = site.create(&mut ada, "In the middle").await?;
        let issue = key_of(&made)?;
        ok(&go(&mut ada, &issue, "in-progress").await?)?;
        // The project gets a workflow of its own to change.
        let editor = ok(&ada
            .send("POST", WORKFLOWS, Some(json!({"project": site.acme})))
            .await?)?
        .clone();
        let workflow = text_in(&editor["workflow"], "id")?;
        let version = editor["workflow"]["version"].clone();
        let lock = lock_workflow(&mut ada, &workflow).await?;
        let publish = |moves: Value| {
            json!({
                "version": version,
                "lock_token": lock,
                "name": "Open or done",
                "statuses": [
                    {"key": "open", "name": "Open", "group": "not-started"},
                    {"key": "done", "name": "Done", "group": "done"},
                ],
                "changes": [
                    {"from": "open", "to": "done", "button": "Finish"},
                    {"from": "done", "to": "open", "button": "Reopen"},
                ],
                "moves": moves,
            })
        };
        // Nothing is published while an issue has no place to go.
        let path = format!("{WORKFLOWS}/{workflow}");
        let refused = ada.send("PUT", &path, Some(publish(json!([])))).await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::CONFLICT, "workflow.status_in_use")
        );
        ok(&ada
            .send(
                "PUT",
                &path,
                Some(publish(json!([{"from": "in-progress", "to": "open"}]))),
            )
            .await?)?;
        let moved = ada.send("GET", &format!("{ISSUES}/{issue}"), None).await?;
        assert_eq!(ok(&moved)?["issue"]["status"]["key"], "open");
        // The move is in the history of the issue, like any other change of the status.
        let history = ada
            .send(
                "GET",
                &format!("{ISSUES}/{issue}/history?tab=changes"),
                None,
            )
            .await?;
        let statuses: Vec<&Value> = ok(&history)?["items"]
            .as_array()
            .ok_or("no history")?
            .iter()
            .filter(|each| each["field"] == "status")
            .collect();
        // Newest first: the move, the change to *In progress*, and the start.
        assert_eq!(statuses.len(), 3, "{}", history.body);
        assert_eq!(statuses[0]["before_text"], "in-progress");
        assert_eq!(statuses[0]["after_text"], "open");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn m3_f53_who_may_change_names_the_roles_held_in_the_project_over_http() -> TestResult {
    each_database("m3_f53_who_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _) = Site::start(&rig, &app).await?;
        let (mut tim, _) = site
            .person(bb_app::access::StartingRole::TeamMember, "tim")
            .await?;
        let path = "/api/v1/projects/ACME/who-may-change/status";
        let answer = ok(&tim.send("GET", path, None).await?)?.clone();
        let names: Vec<&str> = answer["roles"]
            .as_array()
            .ok_or("no roles")?
            .iter()
            .filter_map(|each| each["name"].as_str())
            .collect();
        assert!(names.contains(&"Team member"), "{answer}");
        assert_eq!(answer["roles_hidden"], false);
        // A field that is not there, or is not seen, is not found.
        let missing = tim
            .send("GET", "/api/v1/projects/ACME/who-may-change/nothing", None)
            .await?;
        assert_eq!(missing.status, StatusCode::NOT_FOUND);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn wf_02_publishing_and_writing_an_issue_at_once_never_strand_an_issue() -> TestResult {
    each_database("wf_02_publish_race", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut ben, _) = site
            .person(bb_app::access::StartingRole::Administrator, "ben")
            .await?;
        for round in 0..6 {
            // A project with a workflow of its own, an issue in In progress (which the publish
            // drops, moving it to Open) and one in Open that a second person moves into it.
            let short = format!("RACE{round}");
            let project = ok(&ada
                .send(
                    "POST",
                    PROJECTS,
                    Some(json!({"short_name": short, "name": short})),
                )
                .await?)?
            .clone();
            let made = |title: &'static str| json!({"issue_type": site.kind, "title": title});
            let path = format!("{PROJECTS}/{short}/issues");
            let moving = key_of(&ada.send("POST", &path, Some(made("Moves"))).await?)?;
            ok(&go(&mut ada, &moving, "in-progress").await?)?;
            let other = key_of(&ada.send("POST", &path, Some(made("Races"))).await?)?;
            let editor = ok(&ada
                .send(
                    "POST",
                    WORKFLOWS,
                    Some(json!({"project": text_in(&project, "id")?})),
                )
                .await?)?
            .clone();
            let workflow = text_in(&editor["workflow"], "id")?;
            let lock = lock_workflow(&mut ada, &workflow).await?;
            let publish = json!({
                "version": editor["workflow"]["version"],
                "lock_token": lock,
                "name": "Open or done",
                "statuses": [
                    {"key": "open", "name": "Open", "group": "not-started"},
                    {"key": "done", "name": "Done", "group": "done"},
                ],
                "changes": [
                    {"from": "open", "to": "done", "button": "Finish"},
                    {"from": "done", "to": "open", "button": "Reopen"},
                ],
                "moves": [{"from": "in-progress", "to": "open"}],
            });
            let address = format!("{WORKFLOWS}/{workflow}");
            let (published, written) = tokio::join!(
                ada.send("PUT", &address, Some(publish)),
                go(&mut ben, &other, "in-progress"),
            );
            let (published, written) = (published?, written?);
            // Whatever order they met in, every issue is in a status its workflow has.
            let statuses: Vec<String> = ok(&ada
                .send(
                    "GET",
                    &format!("{WORKFLOWS}/editor?project={}", text_in(&project, "id")?),
                    None,
                )
                .await?)?["statuses"]
                .as_array()
                .ok_or("no statuses")?
                .iter()
                .filter_map(|each| each["key"].as_str().map(str::to_owned))
                .collect();
            for issue in [&moving, &other] {
                let read = ada.send("GET", &format!("{ISSUES}/{issue}"), None).await?;
                let status = ok(&read)?["issue"]["status"]["key"]
                    .as_str()
                    .ok_or("no status")?
                    .to_owned();
                assert!(
                    statuses.contains(&status),
                    "round {round}: {issue} is in {status}, which {statuses:?} lack \
                     (publish {}, write {})",
                    published.status,
                    written.status
                );
            }
        }
        Ok(())
    })
    .await
}
