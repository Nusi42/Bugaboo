//! *Which roles may change this field here* (M3-F53, ADR-0031): the one answer that the sentence
//! of a refusal and the *Who can do it* of the workflow editor both read. Needs the permission
//! *See issues*; a field the reader may not see is not found.

use axum::extract::State;
use axum::response::Response;
use bb_app::access::{ChangingRole, WhoMayChange, WhoMayChangeQuery};
use bb_core::fields::FieldKey;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::editor::issue_type_in;
use crate::api::auth::{
    AuthHandle, Info, NotFoundAnswer, NotSignedInAnswer, Session, answer_authorized,
};
use crate::errors::{ApiError, ApiPath, ApiQuery};

/// The project and the field.
#[derive(Deserialize)]
pub(crate) struct WhoMayChangePath {
    project: String,
    field: FieldKey,
}

/// For which issue type.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct WhoMayChangeParams {
    /// The ID of the issue type; leave it out for all types.
    issue_type: Option<String>,
}

/// A role that may change the field.
#[derive(Serialize, ToSchema)]
pub(crate) struct ChangingRoleBody {
    /// The ID of the role.
    id: String,
    /// Its name.
    name: String,
    /// Whether it may change the field only in the issues its holders reported.
    own_only: bool,
}

impl From<&ChangingRole> for ChangingRoleBody {
    fn from(role: &ChangingRole) -> Self {
        Self {
            id: role.id.to_string(),
            name: role.name.clone(),
            own_only: role.own_only,
        }
    }
}

/// The roles that may change a field.
#[derive(Serialize, ToSchema)]
pub(crate) struct WhoMayChangeBody {
    /// The roles somebody holds in the project that may change the field.
    roles: Vec<ChangingRoleBody>,
    /// The reader is not shown the people of the project, so no role is named.
    roles_hidden: bool,
}

impl Masked for WhoMayChangeBody {}

impl From<&WhoMayChange> for WhoMayChangeBody {
    fn from(found: &WhoMayChange) -> Self {
        Self {
            roles: found.roles.iter().map(ChangingRoleBody::from).collect(),
            roles_hidden: found.roles_hidden,
        }
    }
}

/// Which roles may change a field in the issues of a project, as the reader may know them: only
/// roles somebody holds there are named, and a reader who is not shown the people of the project
/// by role is named none. Needs the permission *See issues*.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{project}/who-may-change/{field}",
    operation_id = "getWhoMayChange",
    tag = "workflows",
    params(
        ("project" = String, Path, description = "The ID of the project, or its short name."),
        ("field" = FieldKey, Path, description = "The key of the field: its word, or the ID of an own field."),
        WhoMayChangeParams
    ),
    responses((status = 200, description = "The roles.", body = WhoMayChangeBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_who_may_change(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<WhoMayChangePath>,
    ApiQuery(params): ApiQuery<WhoMayChangeParams>,
) -> Result<Response, ApiError> {
    let answer = auth
        .api()?
        .who_may_change(
            info,
            WhoMayChangeQuery {
                session_token,
                project: path.project,
                field: path.field,
                issue_type: issue_type_in(params.issue_type.as_deref())?,
            },
        )
        .await?;
    answer_authorized(&answer.map(|answer| WhoMayChangeBody::from(&answer)))
}

/// The route of the question.
pub(super) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(get_who_may_change))
}
