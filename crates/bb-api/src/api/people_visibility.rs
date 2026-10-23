//! *Who sees people* below `/api/v1/people-visibility` (PRIV-12, ADR-0022, UX §5.2): how every
//! role sees every kind of people information, for the whole organization or for one project, and
//! changing one level. The table is the one of the field permissions (`field_permissions.rs`).
//!
//! Both routes need *Decide what roles see* in the place; a level above the person's own is
//! refused (`access.too-much`).

use axum::extract::State;
use axum::response::Response;
use bb_app::access::PeopleVisibilityQuery;
use bb_app::roles::SetPeopleLevel;
use bb_core::access::PeopleLevel;
use bb_core::id::RoleId;
use bb_core::people_kinds::PeopleKind;
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized, done,
};
use super::field_permissions::{LevelTableBody, PlaceParams};
use super::path_id;
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};

/// The level to set, or `null` to take away what the project set for the role, so the level
/// from above counts again.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewPeopleLevelBody {
    /// One of the `levels` of the table, or `null` (only for a project).
    level: Option<PeopleLevel>,
}

/// The path of one cell: a role and a kind of people information.
#[derive(Deserialize)]
pub(crate) struct PeopleCellPath {
    role: String,
    kind: PeopleKind,
}

/// How every role sees every kind of people information, for the whole organization or for one
/// project, and where each level was set. Needs the permission *Decide which fields and which
/// people information each role sees*.
#[utoipa::path(
    get,
    path = "/api/v1/people-visibility",
    operation_id = "getPeopleVisibility",
    tag = "access",
    params(PlaceParams),
    responses((status = 200, description = "How roles see people.", body = LevelTableBody<PeopleKind, PeopleLevel>), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn get_people_visibility(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(place): ApiQuery<PlaceParams>,
) -> Result<Response, ApiError> {
    let project = place.project()?;
    let table = auth
        .api()?
        .people_visibility(
            info,
            PeopleVisibilityQuery {
                session_token,
                project,
            },
        )
        .await?;
    answer_authorized(&table.map(|table| LevelTableBody::of(&table, PeopleLevel::ALL)))
}

/// Sets how a role sees one kind of people information, for the whole organization (the role's
/// own level) or for one project and its sub-projects; `null` takes a project's setting away. A
/// level above the person's own is refused. Needs the permission *Decide which fields and which
/// people information each role sees* in the place.
#[utoipa::path(
    put,
    path = "/api/v1/people-visibility/{role}/{kind}",
    operation_id = "changePeopleVisibility",
    tag = "access",
    params(
        ("role" = String, Path, description = "The ID of the role."),
        ("kind" = PeopleKind, Path, description = "The key of the kind of people information."),
        PlaceParams
    ),
    request_body = NewPeopleLevelBody,
    responses(
        (status = 204, description = "The level was set."),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn change_people_visibility(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<PeopleCellPath>,
    ApiQuery(place): ApiQuery<PlaceParams>,
    ApiJson(body): ApiJson<NewPeopleLevelBody>,
) -> Result<Response, ApiError> {
    let project = place.project()?;
    let role = path_id::<RoleId>(&path.role)?;
    auth.api()?
        .set_people_level(
            info,
            SetPeopleLevel {
                session_token,
                role,
                project,
                kind: path.kind,
                level: body.level,
            },
        )
        .await?;
    Ok(done())
}

/// The routes of *Who sees people*.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_people_visibility))
        .routes(routes!(change_people_visibility))
}
