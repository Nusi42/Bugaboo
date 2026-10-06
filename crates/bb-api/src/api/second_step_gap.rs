//! Two-step sign-in by role (AUTH-03, M1-F45): how many people with a role still have to set up a
//! second step, for the administrator who switches the role's switch on. The switch itself is
//! stored with the role (`bb_app::roles`); this is the number that goes next to it.

use axum::extract::State;
use axum::response::Response;
use bb_app::auth::SecondStepGapQuery;
use bb_core::ErrorCode;
use bb_core::id::RoleId;
use bb_policy::Masked;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized,
};
use crate::errors::{ApiError, ApiPath};

/// How many people with a role have no second step yet.
#[derive(Serialize, ToSchema)]
pub(crate) struct SecondStepGapBody {
    /// The people who hold the role, through their own rows or their groups', anywhere, and have
    /// no code app and no passkey yet. Only people the person who asks may see are counted: the
    /// answer needs *Manage roles*, and whoever holds it sees names everywhere in *Organization
    /// settings*.
    still_to_set_up: u64,
}

impl Masked for SecondStepGapBody {}

/// How many people with a role still have to set up a second step. Needs the permission *Make,
/// change and delete roles*.
#[utoipa::path(
    get,
    path = "/api/v1/roles/{role_id}/second-step-gap",
    operation_id = "getSecondStepGap",
    tag = "auth",
    extensions(("x-never-stored" = json!(true))),
    params(("role_id" = String, Path, description = "The ID of the role.")),
    responses((status = 200, description = "How many people with the role have no second step yet.", body = SecondStepGapBody), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn get_second_step_gap(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(role_id): ApiPath<String>,
) -> Result<Response, ApiError> {
    let role: RoleId = role_id
        .parse()
        .map_err(|_| ApiError::new(ErrorCode::NotFound))?;
    let gap = auth
        .api()?
        .second_step_gap(
            info,
            SecondStepGapQuery {
                session_token,
                role,
            },
        )
        .await?;
    answer_authorized(&gap.map(|gap| SecondStepGapBody {
        still_to_set_up: gap.still_to_set_up,
    }))
}

/// The route of the number.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(get_second_step_gap))
}
