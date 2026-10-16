//! Projects through the real wiring, on both databases (PRJ-01, PRJ-02, PRJ-03, PRIV-13, M2-F23):
//! nesting, short names and their history, what a person who may not see a project is told, the
//! preview of a move and the rule that a move lifts no *No access*.

use bb_app::access::{Permission, PersonRef};
use bb_app::auth::AppError;
use bb_app::projects::{
    ChangeProject, MovePreview, MovePreviewQuery, MoveProject, ProjectDetail, ProjectPage,
    ProjectQuery, ProjectView, ProjectsQuery,
};
use bb_app::roles::{AssignmentsQuery, Given, RemoveAssignment, Subject};
use bb_core::ErrorCode;
use bb_core::access::AppliesTo;
use bb_core::id::ProjectId;
use bb_core::sign_in::SecurityAction;
use bb_testing::{TestResult, each_database};

use super::policy_rig::{Member, Org};
use super::rig::{Rig, refused};

mod counts_and_log;
mod hidden;
mod moves;
mod names;

/// What a lead of a project may do.
pub(super) const LEAD: [Permission; 5] = [
    Permission::SeeProjects,
    Permission::CreateSubProjects,
    Permission::MoveProjects,
    Permission::ChangeProjectSettings,
    Permission::DecideRoles,
];
/// What somebody who only looks may do.
pub(super) const VIEW: [Permission; 1] = [Permission::SeeProjects];

pub(super) fn api<'o>(org: &'o Org<'_>) -> &'o dyn bb_app::auth::AuthApi {
    org.rig.api()
}

pub(super) async fn detail(
    org: &Org<'_>,
    by: &Member,
    address: &str,
) -> Result<ProjectDetail, AppError> {
    Ok(api(org)
        .project(
            Rig::info(),
            ProjectQuery {
                session_token: by.session.clone(),
                address: address.to_owned(),
            },
        )
        .await?
        .into_value())
}

pub(super) async fn list(
    org: &Org<'_>,
    by: &Member,
    inside: Option<ProjectId>,
    search: Option<&str>,
) -> Result<ProjectPage, AppError> {
    Ok(api(org)
        .projects(
            Rig::info(),
            ProjectsQuery {
                session_token: by.session.clone(),
                inside,
                search: search.map(str::to_owned),
                outside: None,
                movable: None,
                cursor: None,
                limit: 100,
            },
        )
        .await?
        .into_value())
}

pub(super) async fn create(
    org: &Org<'_>,
    by: &Member,
    parent: Option<ProjectId>,
    short_name: &str,
) -> Result<ProjectView, AppError> {
    Ok(api(org)
        .create_project(
            Rig::info(),
            bb_app::projects::CreateProject {
                session_token: by.session.clone(),
                parent,
                short_name: short_name.to_owned(),
                name: format!("Project {short_name}"),
            },
        )
        .await?
        .into_value())
}

pub(super) async fn change(
    org: &Org<'_>,
    by: &Member,
    project: &ProjectView,
    name: Option<&str>,
    short_name: Option<&str>,
) -> Result<ProjectView, AppError> {
    Ok(api(org)
        .change_project(
            Rig::info(),
            ChangeProject {
                session_token: by.session.clone(),
                project: project.id,
                version: project.version,
                name: name.map(str::to_owned),
                short_name: short_name.map(str::to_owned),
                fields: std::collections::BTreeMap::new(),
            },
        )
        .await?
        .into_value())
}

pub(super) async fn preview(
    org: &Org<'_>,
    by: &Member,
    project: ProjectId,
    to: Option<ProjectId>,
) -> Result<MovePreview, AppError> {
    Ok(api(org)
        .preview_move(
            Rig::info(),
            MovePreviewQuery {
                session_token: by.session.clone(),
                project,
                to,
            },
        )
        .await?
        .into_value())
}

pub(super) async fn relocate(
    org: &Org<'_>,
    by: &Member,
    project: ProjectId,
    to: Option<ProjectId>,
    preview: &str,
) -> Result<ProjectView, AppError> {
    Ok(api(org)
        .move_project(
            Rig::info(),
            MoveProject {
                session_token: by.session.clone(),
                project,
                to,
                preview: preview.to_owned(),
            },
        )
        .await?
        .into_value())
}

/// The project as the person asks for it by address, without the answer's content: only whether
/// and how it was refused.
pub(super) async fn asked(org: &Org<'_>, by: &Member, address: &str) -> Option<AppError> {
    detail(org, by, address).await.err()
}

pub(super) fn missing_id() -> ProjectId {
    ProjectId::from_parts(jiff::Timestamp::UNIX_EPOCH, [9; 10])
        .unwrap_or_else(|_| unreachable!("the epoch is not before 1970"))
}

pub(super) fn names(page: &ProjectPage) -> Vec<String> {
    page.items
        .iter()
        .map(|item| item.short_name.to_string())
        .collect()
}
