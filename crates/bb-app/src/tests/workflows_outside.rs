//! An issue never gets around the rules of its workflow by changing its type or its project
//! (WF-03, ISS-02, ISS-09, ADR-0031 amendment 3): a status change in the same request is judged in
//! the workflow that counts afterwards, and an issue whose status that workflow lacks can only
//! start again in its starting status.

use bb_core::ErrorCode;
use bb_core::id::{IssueId, IssueTypeId};
use bb_core::message::{self, Message};
use bb_core::workflow::starter::{DONE, IN_PROGRESS, OPEN};
use bb_core::workflow::{ChangeDraft, StatusDraft};

use super::many_changes::{apply, preview, targets};
use super::scenarios::a_project;
use super::scenarios_issues::an_issue;
use super::scenarios_workflows::{a_workflow, published_with};
use super::workflows::change;
use super::workflows_rules::status_of;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::issues::{
    ChangeIssue, IssueTypesQuery, MoveIssue, change_issue, issue_types, move_issue,
};
use crate::many_changes::ManyChangeAction;

const TESTING: &str = "testing";

async fn bug_type(world: &World) -> Result<IssueTypeId, String> {
    let page = issue_types(
        &world.ctx(),
        IssueTypesQuery {
            session_token: world.admin.session_token.clone(),
            cursor: None,
            limit: 50,
        },
    )
    .await
    .map_err(|error| error.to_string())?
    .into_value();
    page.items
        .iter()
        .find(|found| found.name == Message::from(message::ISSUE_TYPE_BUG).render())
        .map(|found| found.id)
        .ok_or_else(|| "no issue type Bug".to_owned())
}

/// The same request, with the issue type changed.
fn as_type(mut asked: ChangeIssue, issue_type: IssueTypeId) -> ChangeIssue {
    asked.issue_type = Some(issue_type);
    asked
}

/// The version the issue has now.
async fn version_of(world: &World, token: &str, address: &str) -> Result<i64, AppError> {
    Ok(crate::issues::issue(
        &world.ctx(),
        crate::issues::IssueQuery {
            session_token: token.to_owned(),
            issue: address.to_owned(),
        },
    )
    .await?
    .into_value()
    .issue
    .version)
}

#[tokio::test]
async fn wf_03_changing_the_type_does_not_skip_the_rules() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let admin = world.admin.session_token.clone();
    let bug = bug_type(&world).await?;
    // A task in progress becomes a bug: the workflow Bug has no way from In progress to Done, and
    // asking for it in the same request is not a way around that.
    let task = an_issue(&world, project).await?;
    let address = task.to_string();
    change_issue(&world.ctx(), change(&admin, &address, 1, IN_PROGRESS)?).await?;
    let around = as_type(change(&admin, &address, 2, DONE)?, bug);
    assert_eq!(
        change_issue(&world.ctx(), around).await.err(),
        Some(AppError::Refused(ErrorCode::WorkflowNoSuchChange))
    );
    // The type alone keeps the status, which the workflow of a Bug has too.
    let mut only_type = as_type(change(&admin, &address, 2, DONE)?, bug);
    only_type.status = None;
    change_issue(&world.ctx(), only_type).await?;
    assert_eq!(status_of(&world, &admin, task).await?, IN_PROGRESS);
    // A bug in testing becomes a task: Simple has no Testing, so the issue can only start again,
    // and asking for Done at the same time is refused.
    let tested = change(&admin, &address, 3, TESTING)?;
    change_issue(&world.ctx(), tested).await?;
    let simple = crate::issues::IssueQuery {
        session_token: admin.clone(),
        issue: address.clone(),
    };
    let version = crate::issues::issue(&world.ctx(), simple)
        .await?
        .into_value()
        .issue
        .version;
    let task_type = super::scenarios_issues::a_type(&world).await?;
    let skipping = as_type(change(&admin, &address, version, DONE)?, task_type);
    assert_eq!(
        change_issue(&world.ctx(), skipping).await.err(),
        Some(AppError::Refused(ErrorCode::WorkflowNoSuchChange))
    );
    // Naming the status it has already is no way around it either: Simple lacks Testing, and the
    // issue is not left in it.
    let named = as_type(change(&admin, &address, version, TESTING)?, task_type);
    change_issue(&world.ctx(), named).await?;
    assert_eq!(status_of(&world, &admin, task).await?, OPEN);
    // Back to a bug in testing, for the case where the request names no status.
    let version = version_of(&world, &admin, &address).await?;
    let mut bug_again = as_type(change(&admin, &address, version, OPEN)?, bug);
    bug_again.status = None;
    change_issue(&world.ctx(), bug_again).await?;
    for (step, status) in [IN_PROGRESS, TESTING].into_iter().enumerate() {
        let next = version + 1 + i64::try_from(step)?;
        change_issue(&world.ctx(), change(&admin, &address, next, status)?).await?;
    }
    let version = version + 3;
    // Without a status in the request, the issue is put into the starting status.
    let mut back = as_type(change(&admin, &address, version, OPEN)?, task_type);
    back.status = None;
    change_issue(&world.ctx(), back).await?;
    assert_eq!(status_of(&world, &admin, task).await?, OPEN);
    Ok(())
}

/// ACME keeps *Simple*; OTHER has a workflow of its own with two statuses that Simple lacks.
async fn two_projects(world: &World) -> Result<IssueId, AppError> {
    let acme = a_project(world, "ACME").await.map_err(AppError::Internal)?;
    a_project(world, "OTHER")
        .await
        .map_err(AppError::Internal)?;
    let editor = a_workflow(world, "OTHER")
        .await
        .map_err(AppError::Internal)?;
    published_with(world, &editor, |sent| {
        sent.statuses = vec![
            StatusDraft {
                key: None,
                name: "Intake".to_owned(),
                group: bb_core::issue::StatusGroup::NotStarted,
            },
            StatusDraft {
                key: None,
                name: "Closed".to_owned(),
                group: bb_core::issue::StatusGroup::Done,
            },
        ];
        sent.changes = vec![ChangeDraft {
            from: "Intake".to_owned(),
            to: "Closed".to_owned(),
            button: "Close".to_owned(),
            rules: bb_core::workflow::StatusRules::default(),
        }];
    })
    .await?;
    let id = an_issue(world, acme).await.map_err(AppError::Internal)?;
    let admin = world.admin.session_token.clone();
    change_issue(&world.ctx(), change(&admin, &id.to_string(), 1, DONE)?).await?;
    Ok(id)
}

#[tokio::test]
async fn wf_03_moving_to_another_project_does_not_skip_the_rules() -> Outcome {
    let world = World::new().await?;
    let id = two_projects(&world).await?;
    let admin = world.admin.session_token.clone();
    // The issue is Done, which OTHER does not have: it starts again in the starting status of the
    // workflow there, not in a status of its own choice.
    move_issue(
        &world.ctx(),
        MoveIssue {
            session_token: admin.clone(),
            issue: id.to_string(),
            version: 2,
            project: "OTHER".to_owned(),
        },
    )
    .await?;
    assert_eq!(status_of(&world, &admin, id).await?, "intake");
    Ok(())
}

#[tokio::test]
async fn wf_03_a_move_by_many_changes_does_not_skip_the_rules() -> Outcome {
    let world = World::new().await?;
    let id = two_projects(&world).await?;
    let admin = world.admin.session_token.clone();
    let action = ManyChangeAction::Move {
        project: "OTHER".to_owned(),
    };
    let shown = preview(&world, &admin, &[id], &action).await?;
    apply(&world, &admin, targets(&shown), &action).await?;
    assert_eq!(status_of(&world, &admin, id).await?, "intake");
    Ok(())
}
