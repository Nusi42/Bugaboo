//! Runs over HTTP (AI-38, COMP-08, COMP-09): the changes of a run, *Undo the run* with its preview, the
//! rule that only the person of a run reads it, and the mark of a text that an agent wrote.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_app::auth::RequestInfo;
use bb_app::context::SystemIds;
use bb_app::issues::ChangeIssue;
use bb_app::many_changes::ManyChangeStore as _;
use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef, RecordKind};
use bb_core::id::{EntityId, EventId, RunId};
use bb_core::issue::Priority;
use bb_core::many_changes::{ManyChangeRun, ManyChangeStatus};
use bb_core::run::RunKind;
use bb_store::AuthStorage;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{ADA_EMAIL, Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, key_of, ok, version_of};
use super::rig::{PASSWORD, Rig};

/// Writes the row of a run as the first request that opens one writes it: with the event of its
/// making, through `channel`.
pub(super) async fn store_run(
    rig: &Rig,
    run: &ManyChangeRun,
    channel: Channel,
) -> Result<(), Failure> {
    let mut ids = SystemIds;
    let event = DomainEvent {
        id: EventId::generate(&mut ids)?,
        time: rig.clock_now(),
        actor: Actor::Person(run.person),
        channel,
        entity: EntityRef {
            kind: EntityKind::from(RecordKind::ChangeRun),
            id: EntityId::from(run.id),
        },
        changes: Vec::new(),
        reason: None,
        run: None,
    };
    let stored = run.clone();
    ChangeStore::record_change(&rig.storage, &event, move |transaction| {
        Box::pin(async move { AuthStorage::insert_many_changes_run(transaction, &stored).await })
    })
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

/// Opens a session of an AI app for Ada: the row of the run, as the way in that opens one writes it.
async fn open_session(rig: &Rig, person: &str) -> Result<RunId, Failure> {
    let mut ids = SystemIds;
    let run = ManyChangeRun {
        id: RunId::generate(&mut ids)?,
        kind: RunKind::AppSession,
        app: Some("Claude Code".to_owned()),
        agent: None,
        person: person.parse().map_err(|_| "no person")?,
        action: json!({}),
        items: Vec::new(),
        status: ManyChangeStatus::Running,
        done: 0,
        changed: 0,
        skipped: Vec::new(),
        created_at: rig.clock_now(),
        finished_at: None,
    };
    store_run(rig, &run, Channel::Mcp).await?;
    Ok(run.id)
}

use bb_app::ports::ChangeStore;

/// Changes the priority of an issue inside the run, the way a request of an AI app does: the request
/// carries the run, and the use case of a single change is the same one.
async fn change_in_run(
    rig: &Rig,
    run: RunId,
    key: &str,
    version: i64,
    priority: Priority,
) -> Result<(), Failure> {
    let session_token = rig.sign_in(ADA_EMAIL, PASSWORD).await?.session_token;
    let info = RequestInfo {
        run: Some(run),
        ..Rig::info()
    };
    rig.api()
        .change_issue(
            info,
            ChangeIssue {
                session_token,
                issue: key.to_owned(),
                version,
                issue_type: None,
                title: None,
                status: None,
                priority: Some(priority),
                estimated_time: None,
                time_left: None,
                assigned_to: None,
                fields: std::collections::BTreeMap::new(),
            },
        )
        .await?;
    Ok(())
}

async fn priority_of(ada: &mut Browser, key: &str) -> Result<String, Failure> {
    let read = ada.send("GET", &format!("{ISSUES}/{key}"), None).await?;
    Ok(ok(&read)?["issue"]["priority"]
        .as_str()
        .ok_or("no priority")?
        .to_owned())
}

fn field_of(body: &Value, at: usize) -> Option<&str> {
    body["items"][at]["fields"][0]["field"].as_str()
}

#[tokio::test]
async fn ai_38_a_run_lists_its_changes_previews_and_undoes_them_over_http() -> TestResult {
    each_database("ai_38_run_over_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let key = key_of(&site.create(&mut ada, "The printer jams").await?)?;
        let was = priority_of(&mut ada, &key).await?;
        let person = rig.person_id(ADA_EMAIL).await?;
        let run = open_session(&rig, &person.to_string()).await?;
        let version = version_of(&mut ada, &key).await?;
        change_in_run(&rig, run, &key, version, Priority::Critical).await?;
        assert_eq!(priority_of(&mut ada, &key).await?, "critical");

        let one = ada
            .send("GET", &format!("/api/v1/runs/{run}"), None)
            .await?;
        assert_eq!(ok(&one)?["kind"], "app-session");
        assert_eq!(ok(&one)?["app"], "Claude Code");
        let changes = ada
            .send("GET", &format!("/api/v1/runs/{run}/changes"), None)
            .await?;
        assert_eq!(ok(&changes)?["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(field_of(&changes.body, 0), Some("priority"));
        assert_eq!(changes.body["items"][0]["issue"]["readable_id"], key);

        let preview = ada
            .send("GET", &format!("/api/v1/runs/{run}/undo"), None)
            .await?;
        assert_eq!(ok(&preview)?["goes_back"], 1);
        let version = preview.body["version"].as_str().ok_or("no version")?;
        let undone = ada
            .send(
                "POST",
                &format!("/api/v1/runs/{run}/undo"),
                Some(json!({"version": version})),
            )
            .await?;
        assert_eq!(ok(&undone)?["goes_back"], 1);
        assert_eq!(undone.body["changed_since_preview"], false);
        assert_eq!(priority_of(&mut ada, &key).await?, was);

        // The undo is itself a change that came after the run: the same version now finds the
        // field changed since, writes nothing and shows the newer preview.
        let again = ada
            .send(
                "POST",
                &format!("/api/v1/runs/{run}/undo"),
                Some(json!({"version": version})),
            )
            .await?;
        assert_eq!(ok(&again)?["changed_since_preview"], true);
        assert_eq!(again.body["goes_back"], 0);
        assert_eq!(again.body["skipped_count"], 1);
        assert_eq!(again.body["skipped"][0]["kind"], "changed-since");
        assert_eq!(again.body["skipped"][0]["by_you"], true);
        assert_eq!(priority_of(&mut ada, &key).await?, was);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_38_a_run_is_not_there_for_anybody_but_its_person() -> TestResult {
    each_database("ai_38_run_of_somebody_else", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let key = key_of(&site.create(&mut ada, "The printer jams").await?)?;
        let person = rig.person_id(ADA_EMAIL).await?;
        let run = open_session(&rig, &person.to_string()).await?;
        let version = version_of(&mut ada, &key).await?;
        change_in_run(&rig, run, &key, version, Priority::Critical).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        for (method, path, body) in [
            ("GET", format!("/api/v1/runs/{run}"), None),
            ("GET", format!("/api/v1/runs/{run}/changes"), None),
            ("GET", format!("/api/v1/runs/{run}/undo"), None),
            (
                "POST",
                format!("/api/v1/runs/{run}/undo"),
                Some(json!({"version": "x"})),
            ),
        ] {
            let reply = bob.send(method, &path, body).await?;
            assert_eq!(reply.status, StatusCode::NOT_FOUND, "{method} {path}");
        }
        assert_eq!(priority_of(&mut ada, &key).await?, "critical");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn comp_09_the_client_cannot_set_or_remove_the_mark() -> TestResult {
    each_database("comp_09_client_and_the_mark", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let key = key_of(&site.create(&mut ada, "The printer jams").await?)?;
        // A mark in the body of a comment is not read: either the body is refused or the comment
        // has no mark.
        let asked = ada
            .send(
                "POST",
                &format!("{ISSUES}/{key}/comments"),
                Some(json!({"text": "Mine", "written_by": {"kind": "agent", "agent": "Fixer"}})),
            )
            .await?;
        if asked.status == StatusCode::OK {
            assert!(asked.body.get("written_by").is_none(), "{}", asked.body);
        } else {
            assert_eq!(asked.status, StatusCode::BAD_REQUEST);
        }
        let plain = ada
            .send(
                "POST",
                &format!("{ISSUES}/{key}/comments"),
                Some(json!({"text": "By hand"})),
            )
            .await?;
        assert!(ok(&plain)?.get("written_by").is_none());
        Ok(())
    })
    .await
}
