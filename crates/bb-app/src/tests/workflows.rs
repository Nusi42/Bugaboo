//! Workflows in the issue use cases (WF-01, WF-03, ISS-09, ADR-0031): the status of an issue is a
//! key of its workflow, every issue answer carries the status changes the reader may make, and a
//! status change is decided by the one judge.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::issue::StatusGroup;
use bb_core::workflow::starter;

use super::scenarios::a_project;
use super::scenarios_issues::an_issue;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::issues::{ChangeIssue, IssueQuery, change_issue, issue};

fn change(token: &str, issue: &str, version: i64, status: &str) -> Result<ChangeIssue, AppError> {
    Ok(ChangeIssue {
        session_token: token.to_owned(),
        issue: issue.to_owned(),
        version,
        issue_type: None,
        title: None,
        status: Some(
            bb_core::workflow::StatusKey::new(status)
                .map_err(|error| AppError::Internal(error.to_string()))?,
        ),
        priority: None,
        assigned_to: None,
        fields: BTreeMap::new(),
    })
}

#[tokio::test]
async fn wf_01_an_issue_answer_carries_its_status_with_name_and_group_and_the_changes_to_offer()
-> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let id = an_issue(&world, project).await?;
    let admin = world.admin.session_token.clone();
    let read = issue(
        &world.ctx(),
        IssueQuery {
            session_token: admin.clone(),
            issue: id.to_string(),
        },
    )
    .await?
    .into_value();
    // A new issue starts in the first status of its workflow.
    let status = &read.issue.status;
    assert_eq!(status.key, starter::open());
    assert_eq!(status.name, "Open");
    assert_eq!(status.group, StatusGroup::NotStarted);
    // The administrator may go to the other two statuses, with the button text of the workflow.
    let buttons: Vec<(&str, &str)> = read
        .issue
        .status_changes
        .iter()
        .map(|each| (each.button.as_str(), each.to.key.as_str()))
        .collect();
    assert_eq!(
        buttons,
        [("Start working", "in-progress"), ("Mark as done", "done")]
    );
    assert!(
        read.issue
            .status_changes
            .iter()
            .all(|each| each.needs.is_empty())
    );
    // Somebody who cannot change the issue gets none.
    let bob = world.person("bob@example.test").await?.session_token;
    let bobs = issue(
        &world.ctx(),
        IssueQuery {
            session_token: bob,
            issue: id.to_string(),
        },
    )
    .await;
    if let Ok(seen) = bobs {
        assert!(seen.into_value().issue.status_changes.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn wf_01_a_status_change_follows_the_workflow_and_the_closed_date_follows_the_done_group()
-> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let id = an_issue(&world, project).await?.to_string();
    let admin = world.admin.session_token.clone();
    let done = change_issue(&world.ctx(), change(&admin, &id, 1, "done")?)
        .await?
        .into_value();
    assert_eq!(done.status.group, StatusGroup::Done);
    assert!(done.closed_at.is_some());
    // Leaving the group clears the date.
    let reopened = change_issue(&world.ctx(), change(&admin, &id, 2, "in-progress")?)
        .await?
        .into_value();
    assert_eq!(reopened.status.name, "In progress");
    assert!(reopened.closed_at.is_none());
    Ok(())
}

#[tokio::test]
async fn wf_01_a_status_the_workflow_does_not_have_is_refused_with_its_own_code() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let id = an_issue(&world, project).await?.to_string();
    let admin = world.admin.session_token.clone();
    let refused = change_issue(&world.ctx(), change(&admin, &id, 1, "testing")?).await;
    assert_eq!(
        refused.err(),
        Some(AppError::Refused(ErrorCode::WorkflowNoSuchChange))
    );
    Ok(())
}
