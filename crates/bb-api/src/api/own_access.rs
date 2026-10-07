//! What a change of who has which role takes from the person who makes it, below
//! `/api/v1/own-access` (SEC-08, ADR-0025 decision 6).
//!
//! The pages ask first when a change would take away the person's own way to manage roles; they
//! ask here, before they send the change. Nothing is changed or written. The route needs a
//! signed-in person; a person who may not make the change is told that it takes nothing, so nobody
//! learns anything about a row, group or role they may not change.

use axum::extract::State;
use axum::response::Response;
use bb_app::access::{OwnChange, OwnLoss, OwnLossQuery};
use bb_core::id::{AssignmentId, GroupId, RoleId, UserId};
use bb_policy::{Masked, Permission};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::assignments::{WhoBody, subject_of};
use super::auth::{AuthHandle, Info, InvalidAnswer, NotSignedInAnswer, Session, answer_authorized};
use super::people::parse;
use super::projects::project_id;
use crate::errors::{ApiError, ApiJson};

/// The change that is about to be made.
#[derive(Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum OwnChangeBody {
    /// Take a row of *Who has which role* away.
    RemoveAssignment {
        /// The ID of the row.
        assignment: String,
    },
    /// Put *No access* on a person or a group in a project.
    NoAccess {
        /// Whom.
        who: WhoBody,
        /// The ID of the project.
        project: String,
    },
    /// Take a person out of a group.
    LeaveGroup {
        /// The ID of the group.
        group: String,
        /// The ID of the person.
        person: String,
    },
    /// Delete a group.
    DeleteGroup {
        /// The ID of the group.
        group: String,
    },
    /// Give a role exactly these permissions.
    ChangeRole {
        /// The ID of the role.
        role: String,
        /// What it allows afterwards.
        permissions: Vec<Permission>,
    },
    /// Delete a role; everybody who has it gets the role that takes its place.
    DeleteRole {
        /// The ID of the role.
        role: String,
        /// The ID of the role that takes its place, if somebody has the role.
        replacement: Option<String>,
    },
}

/// What the person would lose.
#[derive(Serialize, ToSchema)]
pub(crate) struct OwnLossBody {
    /// The permissions that manage roles and who has which role, held now and not after the
    /// change; empty when the change takes nothing from the person.
    loses: Vec<Permission>,
}

impl Masked for OwnLossBody {}

impl From<&OwnLoss> for OwnLossBody {
    fn from(loss: &OwnLoss) -> Self {
        Self {
            loses: loss.loses.clone(),
        }
    }
}

fn change_of(body: OwnChangeBody) -> Result<OwnChange, ApiError> {
    Ok(match body {
        OwnChangeBody::RemoveAssignment { assignment } => {
            OwnChange::RemoveAssignment(parse::<AssignmentId>(&assignment)?)
        }
        OwnChangeBody::NoAccess { who, project } => OwnChange::NoAccess {
            subject: subject_of(&who)?,
            project: project_id(&project)?,
        },
        OwnChangeBody::LeaveGroup { group, person } => OwnChange::LeaveGroup {
            group: parse::<GroupId>(&group)?,
            person: parse::<UserId>(&person)?,
        },
        OwnChangeBody::DeleteGroup { group } => OwnChange::DeleteGroup(parse::<GroupId>(&group)?),
        OwnChangeBody::ChangeRole { role, permissions } => OwnChange::ChangeRole {
            role: parse::<RoleId>(&role)?,
            permissions: permissions.into_iter().collect(),
        },
        OwnChangeBody::DeleteRole { role, replacement } => OwnChange::DeleteRole {
            role: parse::<RoleId>(&role)?,
            replacement: replacement.as_deref().map(parse::<RoleId>).transpose()?,
        },
    })
}

/// Says what a change would take away from the person who is about to make it. A person who may
/// not make the change is told that it takes nothing. Nothing is changed.
#[utoipa::path(
    post,
    path = "/api/v1/own-access/check-change",
    operation_id = "checkOwnAccessChange",
    tag = "roles",
    request_body = OwnChangeBody,
    responses(
        (status = 200, description = "What the change would take from the person.", body = OwnLossBody),
        InvalidAnswer,
        NotSignedInAnswer
    )
)]
async fn check_change(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<OwnChangeBody>,
) -> Result<Response, ApiError> {
    let change = change_of(body)?;
    let found = auth
        .api()?
        .own_loss(
            info,
            OwnLossQuery {
                session_token,
                change,
            },
        )
        .await?;
    answer_authorized(&found.map(|loss| OwnLossBody::from(&loss)))
}

pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(check_change))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_is_named_by_its_kind() {
        let body: OwnChangeBody =
            serde_json::from_str(r#"{"kind":"delete-group","group":"x"}"#).expect("a change");
        assert!(matches!(body, OwnChangeBody::DeleteGroup { .. }));
        assert!(serde_json::from_str::<OwnChangeBody>(r#"{"kind":"make-coffee"}"#).is_err());
    }
}
