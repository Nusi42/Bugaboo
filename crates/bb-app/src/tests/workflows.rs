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
use crate::issues::{ChangeIssue, IssueQuery, change_issue, create_issue, issue};
use crate::workflows::{
    CreateStartingWorkflows, WorkflowEditorQuery, WorkflowsQuery, create_starting_workflows,
    workflow_editor, workflows,
};

pub(super) fn change(
    token: &str,
    issue: &str,
    version: i64,
    status: &str,
) -> Result<ChangeIssue, AppError> {
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

/// The ID of the issue type with this name.
async fn type_named(world: &World, name: &str) -> Result<bb_core::id::IssueTypeId, String> {
    use crate::issues::IssueStore as _;

    world
        .store
        .issue_types()
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|each| each.name == name)
        .map(|each| each.id)
        .ok_or_else(|| format!("no issue type {name}"))
}

/// The name of the workflow that counts for an issue type in the whole organization.
async fn workflow_of_type(world: &World, name: &str) -> Result<String, AppError> {
    let issue_type = type_named(world, name).await.map_err(AppError::Internal)?;
    Ok(workflow_editor(
        &world.ctx(),
        WorkflowEditorQuery {
            session_token: world.admin.session_token.clone(),
            project: None,
            issue_type: Some(issue_type),
        },
    )
    .await?
    .into_value()
    .name)
}

#[tokio::test]
async fn wf_06_a_new_organization_has_the_three_starter_workflows() -> Outcome {
    let world = World::new().await?;
    let listed = workflows(
        &world.ctx(),
        WorkflowsQuery {
            session_token: world.admin.session_token.clone(),
            project: None,
        },
    )
    .await?
    .into_value();
    let names: Vec<&str> = listed
        .workflows
        .iter()
        .map(|each| each.name.as_str())
        .collect();
    assert_eq!(names, ["Bug", "Development", "Simple"]);
    // Task and Goal use Simple, Bug uses Bug, Feature uses Development.
    for (issue_type, workflow) in [
        ("Task", "Simple"),
        ("Goal", "Simple"),
        ("Bug", "Bug"),
        ("Feature", "Development"),
    ] {
        assert_eq!(workflow_of_type(&world, issue_type).await?, workflow);
    }
    // A bug starts in the workflow of bugs: it can be started or turned down, not just finished.
    let project = a_project(&world, "ACME").await?;
    let created = create_issue(
        &world.ctx(),
        crate::issues::CreateIssue {
            session_token: world.admin.session_token.clone(),
            project: project.to_string(),
            issue_type: type_named(&world, "Bug").await?,
            title: "It breaks".to_owned(),
            description: None,
            status: None,
            priority: None,
            assigned_to: None,
            fields: BTreeMap::new(),
        },
    )
    .await?
    .into_value();
    let to: Vec<&str> = created
        .status_changes
        .iter()
        .map(|each| each.to.key.as_str())
        .collect();
    assert_eq!(to, ["in-progress", "wont-fix"]);
    // Starting twice makes nothing again.
    create_starting_workflows(&world.ctx(), CreateStartingWorkflows).await?;
    let again = workflows(
        &world.ctx(),
        WorkflowsQuery {
            session_token: world.admin.session_token.clone(),
            project: None,
        },
    )
    .await?
    .into_value();
    assert_eq!(again.workflows.len(), 3);
    Ok(())
}

#[tokio::test]
async fn wf_06_an_organization_that_has_issues_keeps_simple_for_every_type() -> Outcome {
    // An organization that ran before workflows: issues, and no workflow row.
    let world = World::new_without_workflows().await?;
    let project = a_project(&world, "ACME").await?;
    an_issue(&world, project).await?;
    create_starting_workflows(&world.ctx(), CreateStartingWorkflows).await?;
    let listed = workflows(
        &world.ctx(),
        WorkflowsQuery {
            session_token: world.admin.session_token.clone(),
            project: None,
        },
    )
    .await?
    .into_value();
    assert!(listed.workflows.is_empty());
    for issue_type in ["Task", "Bug", "Feature", "Goal"] {
        assert_eq!(workflow_of_type(&world, issue_type).await?, "Simple");
    }
    // The issue is a Bug and goes from Open straight to Done, as it always could.
    let id = an_issue(&world, project).await?.to_string();
    let admin = world.admin.session_token.clone();
    let done = change_issue(&world.ctx(), change(&admin, &id, 1, "done")?)
        .await?
        .into_value();
    assert_eq!(done.status.group, StatusGroup::Done);
    Ok(())
}
