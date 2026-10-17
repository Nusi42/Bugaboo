//! The example requests of the routes of workflows (M4 WP-04).
//!
//! The routes work on the workflow of the target project. The ones that change a workflow make it
//! first, as the administrator of the matrix, so each actor works on a workflow that is there. A
//! publish carries a token of a lock nobody holds: an actor who is let in is told that the lock is
//! lost, which is not a refusal of the permission. *Try it* works on an issue of the target
//! project, for the administrator of the matrix.

use bb_app::workflows::CreateWorkflow;
use bb_testing::Failure;
use serde_json::json;

use super::actors::Project;
use super::examples::Request;
use super::examples_issues::an_issue;
use super::routes::Fixture;
use crate::sign_in_tests::rig::Rig;

/// A token that holds no lock.
const NOBODYS_LOCK: &str = "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f";

/// The workflow of the target project, made by the administrator of the matrix when it is not there:
/// its ID and its version.
async fn a_workflow(fixture: &Fixture<'_>) -> Result<(String, i64), Failure> {
    let editor = fixture
        .rig()
        .api()
        .create_workflow(
            Rig::info(),
            CreateWorkflow {
                session_token: fixture.admin_session().to_owned(),
                project: Some(fixture.project(Project::Target).to_string()),
                issue_type: None,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    let slot = editor.workflow.ok_or("the workflow was not made")?;
    Ok((slot.id.to_string(), slot.version))
}

pub(super) async fn workflow_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    let target = fixture.project(Project::Target).to_string();
    Ok(Some(match operation {
        "getProjectStatuses" => ("GET", format!("/api/v1/projects/{target}/statuses"), None),
        "getWhoMayChange" => (
            "GET",
            format!("/api/v1/projects/{target}/who-may-change/status"),
            None,
        ),
        "listWorkflows" => ("GET", format!("/api/v1/workflows?project={target}"), None),
        "getWorkflowEditor" => (
            "GET",
            format!("/api/v1/workflows/editor?project={target}"),
            None,
        ),
        "createWorkflow" => (
            "POST",
            "/api/v1/workflows".into(),
            Some(json!({"project": target})),
        ),
        "useWorkflow" => (
            "PUT",
            "/api/v1/workflows/uses".into(),
            Some(json!({"project": target, "workflow": null})),
        ),
        "publishWorkflow" => {
            let (workflow, version) = a_workflow(fixture).await?;
            (
                "PUT",
                format!("/api/v1/workflows/{workflow}"),
                Some(json!({
                    "version": version,
                    "lock_token": NOBODYS_LOCK,
                    "name": "Simple",
                    "statuses": [{"key": "open", "name": "Open", "group": "not-started"}],
                    "changes": [],
                })),
            )
        }
        "deleteWorkflow" => {
            let (workflow, version) = a_workflow(fixture).await?;
            (
                "DELETE",
                format!("/api/v1/workflows/{workflow}?version={version}"),
                None,
            )
        }
        "tryWorkflow" => {
            let issue = an_issue(fixture).await?;
            (
                "GET",
                format!(
                    "/api/v1/issues/{issue}/status-changes?person={}",
                    fixture.admin_id()
                ),
                None,
            )
        }
        _ => return Ok(None),
    }))
}
