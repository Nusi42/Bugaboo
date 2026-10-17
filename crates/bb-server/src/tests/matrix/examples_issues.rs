//! The example requests of the routes of issues and issue types (M3 WP-03).
//!
//! Every example that changes or reads one issue makes a new one first, reported by the
//! administrator of the matrix in the target project, so each actor who is let in works on an
//! issue of their own and nobody else's work gets in the way; and the actor, who reported none of
//! them, shows what the rules about other people's issues say (*See issues*, *Change issues*).

use bb_app::issues::{CreateIssue, CreateIssueType, DeleteIssue, IssueQuery, IssueTypesQuery};
use bb_core::issue::IssueIcon;
use bb_testing::Failure;
use serde_json::json;

use super::actors::Project;
use super::examples::Request;
use super::routes::Fixture;
use crate::sign_in_tests::rig::Rig;

/// The ID of the first issue type (*Bug*).
async fn a_type(fixture: &Fixture<'_>) -> Result<String, Failure> {
    let page = fixture
        .rig()
        .api()
        .issue_types(
            Rig::info(),
            IssueTypesQuery {
                session_token: fixture.admin_session().to_owned(),
                cursor: None,
                limit: 50,
                project: None,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    // *Task* uses the workflow *Simple*, where a many-changes example can set any status.
    page.items
        .iter()
        .find(|found| found.name == "Task")
        .map(|found| found.id.to_string())
        .ok_or_else(|| "the matrix has no issue type Task".into())
}

/// The version of an issue (for the routes that change it).
pub(super) async fn version_of(fixture: &Fixture<'_>, issue: &str) -> Result<i64, Failure> {
    Ok(fixture
        .rig()
        .api()
        .issue(
            Rig::info(),
            IssueQuery {
                session_token: fixture.admin_session().to_owned(),
                issue: issue.to_owned(),
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value()
        .issue
        .version)
}

/// A new issue in the target project, reported by the administrator of the matrix: its readable ID.
pub(super) async fn an_issue(fixture: &Fixture<'_>) -> Result<String, Failure> {
    let made = fixture
        .rig()
        .api()
        .create_issue(
            Rig::info(),
            CreateIssue {
                session_token: fixture.admin_session().to_owned(),
                project: fixture.project(Project::Target).to_string(),
                issue_type: a_type(fixture).await?.parse().map_err(|_| "no type ID")?,
                title: format!("Issue {}", fixture.next_number()),
                description: None,
                status: None,
                priority: None,
                assigned_to: None,
                fields: std::collections::BTreeMap::new(),
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    Ok(made.key)
}

/// An issue type, created by the administrator of the matrix for the whole organization or in the
/// project: its ID.
async fn a_new_type(fixture: &Fixture<'_>, project: Option<String>) -> Result<String, Failure> {
    Ok(fixture
        .rig()
        .api()
        .create_issue_type(
            Rig::info(),
            CreateIssueType {
                session_token: fixture.admin_session().to_owned(),
                project,
                name: format!("Type {}", fixture.next_number()),
                icon: IssueIcon::Bug,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value()
        .id
        .to_string())
}

pub(super) async fn issue_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    let target = fixture.project(Project::Target).to_string();
    Ok(Some(match operation {
        "listIssueTypes" => ("GET", "/api/v1/issue-types".into(), None),
        "createIssueType" => (
            "POST",
            "/api/v1/issue-types".into(),
            Some(json!({
                "name": format!("Type {}", fixture.next_number()),
                "icon": "bug",
                "project": target,
            })),
        ),
        "changeIssueType" => (
            "PATCH",
            format!(
                "/api/v1/issue-types/{}",
                a_new_type(fixture, Some(target.clone())).await?
            ),
            Some(json!({"version": 1, "icon": "star"})),
        ),
        "listIssueTypeOffers" => (
            "GET",
            format!(
                "/api/v1/issue-types/{}/offers",
                a_new_type(fixture, None).await?
            ),
            None,
        ),
        "changeIssueTypeOffers" => (
            "PUT",
            format!(
                "/api/v1/issue-types/{}/offers",
                a_new_type(fixture, None).await?
            ),
            Some(json!({"version": 1, "changes": [{"project": target, "offered": false}]})),
        ),
        "listIssues" => ("GET", format!("/api/v1/projects/{target}/issues"), None),
        "listAssignablePeople" => (
            "GET",
            format!("/api/v1/projects/{target}/assignable-people"),
            None,
        ),
        "createIssue" => (
            "POST",
            format!("/api/v1/projects/{target}/issues"),
            Some(json!({"issue_type": a_type(fixture).await?, "title": "Made by the matrix"})),
        ),
        "getIssue" => (
            "GET",
            format!("/api/v1/issues/{}", an_issue(fixture).await?),
            None,
        ),
        "changeIssue" => (
            "PATCH",
            format!("/api/v1/issues/{}", an_issue(fixture).await?),
            Some(json!({"version": 1, "priority": "high"})),
        ),
        "saveIssueDescription" => (
            "PUT",
            format!("/api/v1/issues/{}/description", an_issue(fixture).await?),
            Some(json!({
                "version": 1,
                "lock_token": super::examples_editing::NO_LOCK_TOKEN,
                "text": "Typed by the matrix"
            })),
        ),
        "deleteIssue" => (
            "DELETE",
            format!("/api/v1/issues/{}", an_issue(fixture).await?),
            None,
        ),
        "restoreIssue" => {
            // An issue that is in the trash, so a person who is let in takes it out.
            let key = an_issue(fixture).await?;
            fixture
                .rig()
                .api()
                .delete_issue(
                    Rig::info(),
                    DeleteIssue {
                        session_token: fixture.admin_session().to_owned(),
                        issue: key.clone(),
                    },
                )
                .await
                .map_err(|error| error.to_string())?;
            ("POST", format!("/api/v1/issues/{key}/restore"), None)
        }
        _ => return Ok(None),
    }))
}
