//! What *Publish* moves and tells, over HTTP on both databases (WF-02, ADR-0031 amendments 11 and
//! 12): the moves of a publish are judged with the rights of the person who publishes, the preview
//! names what they see and only counts the rest, and a move follows no rule of the old status
//! change. The same rules run on the fakes of `bb-app`; here the statements are the database's.

use axum::http::StatusCode;
use bb_app::access::Permission;
use bb_testing::{Failure, TestDatabase, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, id_in, key_of, ok, text_in, version_of};
use super::layouts_http::{AN_EDITOR, LOCKS};

const WORKFLOWS: &str = "/api/v1/workflows";

/// A project inside ACME, by its ID.
async fn sub_project(ada: &mut Browser, site: &Site<'_>) -> Result<String, Failure> {
    let made = ada
        .send(
            "POST",
            &format!("{PROJECTS}/{}/sub-projects", site.acme),
            Some(json!({"short_name": "KID", "name": "Kid"})),
        )
        .await?;
    id_in(ok(&made)?)
}

/// An issue of *Task* in the project (by its short name), put into *In progress*.
async fn in_progress(
    ada: &mut Browser,
    site: &Site<'_>,
    project: &str,
    title: &str,
) -> Result<String, Failure> {
    let made = ada
        .send(
            "POST",
            &format!("{PROJECTS}/{project}/issues"),
            Some(site.new_issue(title)),
        )
        .await?;
    let issue = key_of(&made)?;
    let version = version_of(ada, &issue).await?;
    ok(&ada
        .send(
            "PATCH",
            &format!("{ISSUES}/{issue}"),
            Some(json!({"version": version, "status": "in-progress"})),
        )
        .await?)?;
    Ok(issue)
}

/// The workflow ACME has of its own, made by the administrator: its ID.
async fn own_workflow(ada: &mut Browser, site: &Site<'_>) -> Result<String, Failure> {
    let editor = ok(&ada
        .send("POST", WORKFLOWS, Some(json!({"project": site.acme})))
        .await?)?
    .clone();
    text_in(&editor["workflow"], "id")
}

/// The version of the workflow, as the editor of ACME reads it.
async fn version_of_workflow(who: &mut Browser, site: &Site<'_>) -> Result<Value, Failure> {
    let editor = who
        .send(
            "GET",
            &format!("{WORKFLOWS}/editor?project={}", site.acme),
            None,
        )
        .await?;
    Ok(ok(&editor)?["workflow"]["version"].clone())
}

/// The lock of the editor of the workflow, taken by `who`.
async fn lock_for(who: &mut Browser, workflow: &str) -> Result<Value, Failure> {
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

/// A workflow of *Open* and *Done* only.
fn open_or_done() -> Value {
    json!({
        "statuses": [
            {"key": "open", "name": "Open", "group": "not-started"},
            {"key": "done", "name": "Done", "group": "done"},
        ],
        "changes": [
            {"from": "open", "to": "done", "button": "Finish"},
            {"from": "done", "to": "open", "button": "Reopen"},
        ],
    })
}

/// The body of a publish of `open_or_done` with these moves.
fn publish(version: &Value, lock: &Value, moves: Value) -> Value {
    let mut body = open_or_done();
    body["version"] = version.clone();
    body["lock_token"] = lock.clone();
    body["name"] = json!("Open or done");
    body["moves"] = moves;
    body
}

/// The status and the version of an issue, as the administrator reads it.
async fn state(ada: &mut Browser, issue: &str) -> Result<(String, Value), Failure> {
    let read = ada.send("GET", &format!("{ISSUES}/{issue}"), None).await?;
    let issue = &ok(&read)?["issue"];
    Ok((text_in(&issue["status"], "key")?, issue["version"].clone()))
}

#[tokio::test]
async fn wf_02_a_publish_with_an_issue_the_publisher_does_not_see_publishes_nothing_over_http()
-> TestResult {
    each_database("wf_02_unseen_http", async |db| {
        refused_for_an_issue(db, true).await
    })
    .await
}

#[tokio::test]
async fn wf_02_a_publish_with_an_issue_whose_status_the_publisher_may_not_change_publishes_nothing_over_http()
-> TestResult {
    each_database("wf_02_not_changeable_http", async |db| {
        refused_for_an_issue(db, false).await
    })
    .await
}

/// A publisher who manages workflows in ACME drops *In progress* with an issue in it that they do
/// not see (`unseen`), or see and may not change: the publish is refused and nothing changes.
async fn refused_for_an_issue(db: &TestDatabase, unseen: bool) -> Result<(), Failure> {
    let (rig, app) = rig_and_app(db).await?;
    let (site, mut ada) = Site::start(&rig, &app).await?;
    let kid = sub_project(&mut ada, &site).await?;
    let project = if unseen { "KID" } else { "ACME" };
    let issue = in_progress(&mut ada, &site, project, "In the middle").await?;
    let workflow = own_workflow(&mut ada, &site).await?;
    let mut rights = vec![
        Permission::SeeProjects,
        Permission::SeeIssues,
        Permission::ManageWorkflows,
    ];
    if unseen {
        rights.push(Permission::ChangeIssues);
    }
    let role = site.org.role("Publisher", &rights).await?;
    let (mut lead, member) = site.person_holding(role.id, "lead", &site.acme).await?;
    if unseen {
        let kid = kid.parse().map_err(|_| "no project ID")?;
        site.org.forbid(&member, kid).await?;
    }
    let before = state(&mut ada, &issue).await?;
    let version = version_of_workflow(&mut lead, &site).await?;
    let lock = lock_for(&mut lead, &workflow).await?;
    let refused = lead
        .send(
            "PUT",
            &format!("{WORKFLOWS}/{workflow}"),
            Some(publish(
                &version,
                &lock,
                json!([{"from": "in-progress", "to": "open"}]),
            )),
        )
        .await?;
    assert_eq!(
        (refused.status, refused.code()),
        (StatusCode::CONFLICT, "workflow.status_in_use"),
        "{}",
        refused.body
    );
    assert_eq!(state(&mut ada, &issue).await?, before);
    Ok(())
}

#[tokio::test]
async fn wf_02_the_preview_names_what_the_publisher_sees_and_only_counts_the_rest_over_http()
-> TestResult {
    each_database("wf_02_preview_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let kid = sub_project(&mut ada, &site).await?;
        in_progress(&mut ada, &site, "ACME", "Seen").await?;
        in_progress(&mut ada, &site, "KID", "Unseen").await?;
        let workflow = own_workflow(&mut ada, &site).await?;
        // The lead sees ACME and may not change issues; the issue of KID is not theirs to see.
        let rights = [
            Permission::SeeProjects,
            Permission::SeeIssues,
            Permission::ManageWorkflows,
        ];
        let role = site.org.role("Watching lead", &rights).await?;
        let (mut lead, member) = site.person_holding(role.id, "lead", &site.acme).await?;
        let kid = kid.parse().map_err(|_| "no project ID")?;
        site.org.forbid(&member, kid).await?;
        let preview = lead
            .send(
                "POST",
                &format!("{WORKFLOWS}/{workflow}/publish-preview"),
                Some(open_or_done()),
            )
            .await?;
        let answer = ok(&preview)?;
        let dropped = answer["dropped"].as_array().ok_or("no dropped statuses")?;
        assert_eq!(dropped.len(), 1, "{answer}");
        let line = &dropped[0];
        assert_eq!(line["issues"], 2, "{answer}");
        assert_eq!(line["unseen"], 1, "{answer}");
        assert_eq!(line["blocked"], 2, "{answer}");
        assert_eq!(line["named"].as_array().map(Vec::len), Some(1), "{answer}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn wf_02_a_move_ignores_the_rules_of_the_old_change_and_closes_by_the_group_of_the_target_over_http()
-> TestResult {
    each_database("wf_02_move_rules_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = in_progress(&mut ada, &site, "ACME", "Without a description").await?;
        let workflow = own_workflow(&mut ada, &site).await?;
        // The old change into *Done* needs the description, which this issue lacks.
        let version = version_of_workflow(&mut ada, &site).await?;
        let lock = lock_for(&mut ada, &workflow).await?;
        let needing = json!({
            "version": version,
            "lock_token": lock,
            "name": "Needs a description",
            "statuses": [
                {"key": "open", "name": "Open", "group": "not-started"},
                {"key": "in-progress", "name": "In progress", "group": "in-progress"},
                {"key": "done", "name": "Done", "group": "done"},
            ],
            "changes": [
                {"from": "open", "to": "in-progress", "button": "Start"},
                {"from": "in-progress", "to": "done", "button": "Finish",
                 "rules": {"must_fill": ["description"]}},
                {"from": "done", "to": "open", "button": "Reopen"},
            ],
            "moves": [],
        });
        let path = format!("{WORKFLOWS}/{workflow}");
        ok(&ada.send("PUT", &path, Some(needing)).await?)?;
        let (_, current) = state(&mut ada, &issue).await?;
        let refused = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({"version": current, "status": "done"})),
            )
            .await?;
        assert!(!refused.status.is_success(), "{}", refused.body);
        // The move of a publish follows no such rule, and closes the issue like *Done* does.
        let version = version_of_workflow(&mut ada, &site).await?;
        let lock = lock_for(&mut ada, &workflow).await?;
        let moves = json!([{"from": "in-progress", "to": "done"}]);
        ok(&ada
            .send("PUT", &path, Some(publish(&version, &lock, moves)))
            .await?)?;
        let read = ada.send("GET", &format!("{ISSUES}/{issue}"), None).await?;
        let after = &ok(&read)?["issue"];
        assert_eq!(after["status"]["key"], "done");
        assert!(!after["closed_at"].is_null(), "Done closes the issue");
        Ok(())
    })
    .await
}
