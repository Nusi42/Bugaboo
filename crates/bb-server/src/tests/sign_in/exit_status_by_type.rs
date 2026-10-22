//! The exit review of M4 (WP-12, M1), the *Status* level of the type an issue gets: one request
//! that changes the type and the status is judged by the levels of both types, as the two changes
//! made one after the other would be (ADR-0027, ADR-0031). Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::Permission;
use bb_core::access::FieldLevel;
use bb_core::fields::Field;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::exit_masks::{issue_of_type, other_type, person_with_levels};
use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, ok};
use super::rig::Rig;

const PREVIEW: &str = "/api/v1/many-changes/preview";

const SEE_AND_CHANGE: [Permission; 3] = [
    Permission::SeeProjects,
    Permission::SeeIssues,
    Permission::ChangeIssues,
];

/// Tim changes the status of a task, but only sees it on the other types.
async fn status_changer(site: &Site<'_>, rig: &Rig) -> Result<Browser, Failure> {
    let (tim, _) = person_with_levels(
        site,
        rig,
        "tim",
        &SEE_AND_CHANGE,
        &[
            (Field::Title.into(), None, FieldLevel::CanChange),
            (Field::Status.into(), None, FieldLevel::CanSee),
            (
                Field::Status.into(),
                Some(&site.kind),
                FieldLevel::CanChange,
            ),
        ],
    )
    .await?;
    Ok(tim)
}

#[tokio::test]
async fn wf_03_the_status_level_of_the_new_type_counts_when_the_type_changes() -> TestResult {
    each_database("wf_03_status_level_new_type", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let other = other_type(&site, &mut ada).await?;
        // Tim changes the status of a task, but only sees it on the other type.
        let mut tim = status_changer(&site, &rig).await?;
        let (task, _) = issue_of_type(&site, &mut ada, &site.kind, "A task").await?;
        let path = format!("{ISSUES}/{task}");
        let both = tim
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 1, "issue_type": other, "status": "in-progress"})),
            )
            .await?;
        assert_eq!(
            (both.status, both.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        // Nothing was written, and the two steps are judged the same way.
        let status = tim
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 1, "status": "in-progress"})),
            )
            .await?;
        assert_eq!(ok(&status)?["version"], 2);
        let type_then_status = tim
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 2, "issue_type": other})),
            )
            .await?;
        ok(&type_then_status)?;
        let after = tim
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 3, "status": "done"})),
            )
            .await?;
        assert_eq!(
            (after.status, after.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn wf_03_change_many_issues_skips_an_issue_whose_type_hides_the_status_level() -> TestResult {
    each_database("wf_03_many_status_level", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let other = other_type(&site, &mut ada).await?;
        let mut tim = status_changer(&site, &rig).await?;
        let (task, _) = issue_of_type(&site, &mut ada, &site.kind, "A task").await?;
        let (seen, _) = issue_of_type(&site, &mut ada, &other, "Another type").await?;
        let preview = tim
            .send(
                "POST",
                PREVIEW,
                Some(json!({
                    "issues": [task, seen],
                    "action": {"kind": "change", "change": {"status": "in-progress"}},
                })),
            )
            .await?;
        let rows = ok(&preview)?["rows"].clone();
        assert!(rows[0]["skipped"].is_null(), "{rows}");
        assert!(rows[1]["skipped"].is_string(), "{rows}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn wf_03_a_change_of_type_that_puts_the_status_back_needs_the_level_of_the_new_type()
-> TestResult {
    each_database("wf_03_reset_status_level", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let types = ada.send("GET", "/api/v1/issue-types", None).await?;
        let bug = types.body["items"]
            .as_array()
            .and_then(|items| items.iter().find(|each| each["name"] == "Bug"))
            .and_then(|found| found["id"].as_str())
            .ok_or("no issue type Bug")?
            .to_owned();
        // Tim changes the status of a bug, and only sees it on every other type.
        let (mut tim, _) = person_with_levels(
            &site,
            &rig,
            "tim",
            &SEE_AND_CHANGE,
            &[
                (Field::Title.into(), None, FieldLevel::CanChange),
                (Field::Status.into(), None, FieldLevel::CanSee),
                (Field::Status.into(), Some(&bug), FieldLevel::CanChange),
            ],
        )
        .await?;
        let (key, _) = issue_of_type(&site, &mut ada, &bug, "A bug in testing").await?;
        let path = format!("{ISSUES}/{key}");
        for (version, status) in [(1, "in-progress"), (2, "testing")] {
            let moved = tim
                .send(
                    "PATCH",
                    &path,
                    Some(json!({"version": version, "status": status})),
                )
                .await?;
            assert_eq!(moved.status, StatusCode::OK, "{}", moved.body);
        }
        // A task has no Testing: the type alone would put the status back to the starting status,
        // which is a change of the status, and Tim only sees the status of a task.
        let reset = tim
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 3, "issue_type": site.kind})),
            )
            .await?;
        assert_eq!(
            (reset.status, reset.code()),
            (StatusCode::FORBIDDEN, "workflow.not_for_you")
        );
        Ok(())
    })
    .await
}
