//! The exit review of M3 (WP-12), field levels per issue type (SEC-04, SEC-06, ADR-0027): every
//! path that shows or changes a field of an issue judges it by the levels of the issue's own type,
//! not by the levels of the role for all types. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::Permission;
use bb_app::roles::SetFieldLevel;
use bb_core::access::FieldLevel;
use bb_core::fields::{Field, FieldKey};
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, ok};
use super::policy_rig::Member;
use super::rig::Rig;

const LOCKS: &str = "/api/v1/edit-locks";
const AN_EDITOR: &str = "7c1d2e3f-4a5b-4c6d-8e7f-a0b1c2d3e4f5";

/// One level of a role: the field, the issue type it is for (none: all types), the level.
pub(super) type Row<'a> = (FieldKey, Option<&'a str>, FieldLevel);

/// A person with a role of their own that holds `permissions` and the levels in `rows`; every field
/// that has no row is hidden from them.
pub(super) async fn person_with_levels(
    site: &Site<'_>,
    rig: &Rig,
    email: &str,
    permissions: &[Permission],
    rows: &[Row<'_>],
) -> Result<(Browser, Member), Failure> {
    let role = site.org.role(email, permissions).await?;
    for (field, issue_type, level) in rows {
        rig.api()
            .set_field_level(
                Rig::info(),
                SetFieldLevel {
                    session_token: site.org.admin.session.clone(),
                    role: role.id,
                    project: None,
                    issue_type: issue_type
                        .map(|id| id.parse().map_err(|_| "no issue type ID"))
                        .transpose()?,
                    field: *field,
                    level: Some(*level),
                },
            )
            .await?;
    }
    let email = format!("{email}@example.test");
    let started = rig.person(&site.org.admin.session, &email).await?;
    let member = Member {
        id: started.person.id,
        session: started.session_token,
    };
    site.org.give_everywhere(&member, role.id).await?;
    Ok((Browser::signed_in_as(site.app, &email).await?, member))
}

/// The ID of an issue type that is not *Bug*.
pub(super) async fn other_type(site: &Site<'_>, ada: &mut Browser) -> Result<String, Failure> {
    let types = ada.send("GET", "/api/v1/issue-types", None).await?;
    types.body["items"]
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .filter_map(|item| item["id"].as_str())
                .find(|id| *id != site.bug)
        })
        .map(str::to_owned)
        .ok_or_else(|| "no second issue type on a started server".into())
}

/// An issue of this type in ACME: the readable ID and the ID.
pub(super) async fn issue_of_type(
    site: &Site<'_>,
    ada: &mut Browser,
    issue_type: &str,
    title: &str,
) -> Result<(String, String), Failure> {
    let made = ada
        .send(
            "POST",
            &format!("/api/v1/projects/{}/issues", site.acme),
            Some(json!({"issue_type": issue_type, "title": title})),
        )
        .await?;
    let made = ok(&made)?;
    let text = |name: &str| -> Result<String, Failure> {
        made[name]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("no {name}").into())
    };
    Ok((text("key")?, text("id")?))
}

/// The lock on the description of an issue, by the person: the answer.
async fn lock_description(person: &mut Browser, id: &str) -> Result<Value, Failure> {
    let taken = person
        .send(
            "POST",
            LOCKS,
            Some(json!({
                "what": {"record": {"kind": "issue", "id": id}, "part": "description"},
                "editor_id": AN_EDITOR,
            })),
        )
        .await?;
    Ok(taken.body)
}

const SEE_AND_CHANGE: [Permission; 3] = [
    Permission::SeeProjects,
    Permission::SeeIssues,
    Permission::ChangeIssues,
];

#[tokio::test]
async fn sec_04_the_description_is_judged_by_the_levels_of_the_type_of_the_issue() -> TestResult {
    each_database("sec_04_description_per_type", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let task = other_type(&site, &mut ada).await?;
        // The description may only be seen by this role, except on a bug, where it may be changed.
        let (mut tim, _) = person_with_levels(
            &site,
            &rig,
            "tim",
            &SEE_AND_CHANGE,
            &[
                (Field::Title.into(), None, FieldLevel::CanChange),
                (Field::Description.into(), None, FieldLevel::CanSee),
                (
                    Field::Description.into(),
                    Some(&site.bug),
                    FieldLevel::CanChange,
                ),
            ],
        )
        .await?;
        let (bug_key, bug_id) = issue_of_type(&site, &mut ada, &site.bug, "A bug").await?;
        let (task_key, _) = issue_of_type(&site, &mut ada, &task, "A task").await?;
        // On the task the text is read-only: refused whatever the lock.
        let refused = tim
            .send(
                "PUT",
                &format!("{ISSUES}/{task_key}/description"),
                Some(json!({"version": 1, "lock_token": AN_EDITOR, "text": "Mine"})),
            )
            .await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        // On the bug the level of the type counts, and the text is saved.
        let lock = lock_description(&mut tim, &bug_id).await?;
        let token = lock["lock_token"].as_str().ok_or("no lock token")?;
        let saved = tim
            .send(
                "PUT",
                &format!("{ISSUES}/{bug_key}/description"),
                Some(json!({"version": 1, "lock_token": token, "text": "Mine"})),
            )
            .await?;
        assert_eq!(ok(&saved)?["description"], "Mine");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_a_related_issue_is_shown_by_the_levels_of_its_own_type() -> TestResult {
    each_database("sec_06_lines_per_type", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let task = other_type(&site, &mut ada).await?;
        // Title and status are shown, except on a task.
        let (mut rea, _) = person_with_levels(
            &site,
            &rig,
            "rea",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[
                (Field::Title.into(), None, FieldLevel::CanSee),
                (Field::Status.into(), None, FieldLevel::CanSee),
                (Field::Title.into(), Some(&task), FieldLevel::Hidden),
                (Field::Status.into(), Some(&task), FieldLevel::Hidden),
            ],
        )
        .await?;
        let (bug, _) = issue_of_type(&site, &mut ada, &site.bug, "The bug").await?;
        let (hidden, _) = issue_of_type(&site, &mut ada, &task, "The hidden title").await?;
        let (shown, _) = issue_of_type(&site, &mut ada, &site.bug, "The shown title").await?;
        let kind = ada.send("GET", "/api/v1/relation-kinds", None).await?;
        let blocks = kind.body["items"][0]["id"]
            .as_str()
            .ok_or("no relation kind")?
            .to_owned();
        for other in [&hidden, &shown] {
            ok(&ada
                .send(
                    "POST",
                    &format!("{ISSUES}/{bug}/relations"),
                    Some(json!({"other": other, "kind": blocks, "direction": "forward"})),
                )
                .await?)?;
        }
        // The task is under the bug.
        let seen = ada.send("GET", &format!("{ISSUES}/{hidden}"), None).await?;
        let version = ok(&seen)?["issue"]["version"].clone();
        ok(&ada
            .send(
                "PUT",
                &format!("{ISSUES}/{hidden}/parent"),
                Some(json!({"version": version, "parent": bug})),
            )
            .await?)?;
        let related = rea
            .send("GET", &format!("{ISSUES}/{bug}/related-issues"), None)
            .await?;
        let lines: Vec<&Value> = ok(&related)?["items"]
            .as_array()
            .ok_or("no items")?
            .iter()
            .map(|item| &item["issue"])
            .collect();
        assert_eq!(lines.len(), 2);
        for line in lines {
            let is_task = line["key"] == hidden;
            assert_eq!(line.get("title").is_none(), is_task, "{line}");
            assert_eq!(line.get("status").is_none(), is_task, "{line}");
        }
        let below = rea
            .send("GET", &format!("{ISSUES}/{bug}/sub-issues"), None)
            .await?;
        let sub = &ok(&below)?["items"][0];
        assert_eq!(sub["key"], hidden);
        assert!(sub.get("title").is_none() && sub.get("status").is_none());
        let child = rea
            .send("GET", &format!("{ISSUES}/{hidden}/relations"), None)
            .await?;
        let parent_line = &ok(&child)?["part_of"];
        assert_eq!(parent_line["key"], bug);
        assert!(parent_line.get("title").is_some(), "the bug's own levels");
        // A bug under the task: its parent line is the task's, without title and status.
        let seen = ada.send("GET", &format!("{ISSUES}/{shown}"), None).await?;
        let version = ok(&seen)?["issue"]["version"].clone();
        ok(&ada
            .send(
                "PUT",
                &format!("{ISSUES}/{shown}/parent"),
                Some(json!({"version": version, "parent": hidden})),
            )
            .await?)?;
        let under = rea
            .send("GET", &format!("{ISSUES}/{shown}/relations"), None)
            .await?;
        let parent_line = &ok(&under)?["part_of"];
        assert_eq!(parent_line["key"], hidden);
        assert!(parent_line.get("title").is_none() && parent_line.get("status").is_none());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_a_conflict_names_no_field_the_type_of_the_issue_hides() -> TestResult {
    each_database("sec_06_conflict_per_type", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut kim, _) = person_with_levels(
            &site,
            &rig,
            "kim",
            &SEE_AND_CHANGE,
            &[
                (Field::Title.into(), None, FieldLevel::CanChange),
                (Field::Priority.into(), None, FieldLevel::CanChange),
                (Field::Priority.into(), Some(&site.bug), FieldLevel::Hidden),
            ],
        )
        .await?;
        let (bug, _) = issue_of_type(&site, &mut ada, &site.bug, "A bug").await?;
        ok(&ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{bug}"),
                Some(json!({"version": 1, "priority": "high"})),
            )
            .await?)?;
        let stale = kim
            .send(
                "PATCH",
                &format!("{ISSUES}/{bug}"),
                Some(json!({"version": 1, "title": "Renamed"})),
            )
            .await?;
        assert_eq!(stale.code(), "edit.version_changed", "{}", stale.body);
        let conflict = &stale.body["conflict"];
        assert!(
            conflict["fields"]
                .as_array()
                .is_none_or(|fields| !fields.iter().any(|field| field == "priority")),
            "{conflict}"
        );
        assert!(conflict["current"].get("priority").is_none(), "{conflict}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_04_who_may_be_assigned_follows_the_levels_of_the_type_of_the_issue() -> TestResult {
    each_database("sec_04_assignable_per_type", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let task = other_type(&site, &mut ada).await?;
        // *Assigned to* may be changed on a bug only.
        let (mut lou, _) = person_with_levels(
            &site,
            &rig,
            "lou",
            &SEE_AND_CHANGE,
            &[
                (Field::AssignedTo.into(), None, FieldLevel::CanSee),
                (
                    Field::AssignedTo.into(),
                    Some(&site.bug),
                    FieldLevel::CanChange,
                ),
            ],
        )
        .await?;
        let (bug, _) = issue_of_type(&site, &mut ada, &site.bug, "A bug").await?;
        let (other, _) = issue_of_type(&site, &mut ada, &task, "A task").await?;
        let path = format!("/api/v1/projects/{}/assignable-people", site.acme);
        let on_bug = lou
            .send("GET", &format!("{path}?issue={bug}"), None)
            .await?;
        assert_eq!(on_bug.status, StatusCode::OK, "{}", on_bug.body);
        let on_task = lou
            .send("GET", &format!("{path}?issue={other}"), None)
            .await?;
        assert_eq!(
            (on_task.status, on_task.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        Ok(())
    })
    .await
}
