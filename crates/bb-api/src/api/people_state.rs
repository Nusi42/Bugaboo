//! *Switch off* and *Switch on* (PRIV-04, ADR-0044, decision 5): the routes below
//! `/api/v1/people/{person}` that end a person's every way in at once and give back the sign-in.
//! Both need *Manage people* for the whole organization.

use axum::extract::State;
use axum::response::Response;
use bb_app::people::{SwitchOff, SwitchOn};
use bb_core::id::UserId;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, done,
};
use super::path_id;
use crate::errors::{ApiError, ApiPath};

/// Switches a person off at once: their sessions end, their access keys and the grants of AI apps
/// are taken back, their agents stop and the invitations they made are taken back. Their
/// assignments stay. Needs *Manage people*.
#[utoipa::path(
    post,
    path = "/api/v1/people/{person}/switch-off",
    operation_id = "switchPersonOff",
    tag = "people",
    params(("person" = String, Path, description = "The ID of the person.")),
    responses(
        (status = 204, description = "The person is switched off."),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn switch_person_off(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(person): ApiPath<String>,
) -> Result<Response, ApiError> {
    let person = path_id::<UserId>(&person)?;
    auth.api()?
        .switch_off(
            info,
            SwitchOff {
                session_token,
                person,
            },
        )
        .await?;
    Ok(done())
}

/// Switches a switched-off person on again: they may sign in. What was taken back stays taken
/// back. Needs *Manage people*.
#[utoipa::path(
    post,
    path = "/api/v1/people/{person}/switch-on",
    operation_id = "switchPersonOn",
    tag = "people",
    params(("person" = String, Path, description = "The ID of the person.")),
    responses(
        (status = 204, description = "The person may sign in again."),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn switch_person_on(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(person): ApiPath<String>,
) -> Result<Response, ApiError> {
    let person = path_id::<UserId>(&person)?;
    auth.api()?
        .switch_on(
            info,
            SwitchOn {
                session_token,
                person,
            },
        )
        .await?;
    Ok(done())
}

pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(switch_person_off))
        .routes(routes!(switch_person_on))
}
