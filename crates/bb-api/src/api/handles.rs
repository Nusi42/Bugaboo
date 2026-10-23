//! The handle of a person (M3-F70, M4-Q9, ADR-0033 §6): `/api/v1/me/handle`, the short unique name
//! that mentions use, made from the name and changeable in *My settings*.
//!
//! It is the person's own, so only a session reaches it. Other people see a handle only where they
//! see names: in the choices after the `@` of a comment.

use axum::extract::State;
use axum::response::Response;
use bb_app::notifications::{ChangeHandle, MyHandle, MyHandleQuery};
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, Info, InvalidAnswer, NotSignedInAnswer, Session, TooSoonAnswer,
    answer_authorized,
};
use crate::errors::{ApiError, ApiJson};

/// A handle: the one a person has, or the new one they ask for. Without the `@`: lower case or
/// ASCII letters `a`…`z`, digits, `.`, `-` and `_`, like `ana.brandt`. Nobody else has it, nobody had
/// it, and no group or role has it.
#[derive(Deserialize, Serialize, ToSchema)]
pub(crate) struct HandleBody {
    /// The handle, without the `@`.
    handle: String,
}

impl Masked for HandleBody {}

impl From<MyHandle> for HandleBody {
    fn from(found: MyHandle) -> Self {
        Self {
            handle: found.handle,
        }
    }
}

/// The handle of the person who asks.
#[utoipa::path(
    get,
    path = "/api/v1/me/handle",
    operation_id = "getMyHandle",
    tag = "people",
    responses((status = 200, description = "The handle.", body = HandleBody), NotSignedInAnswer)
)]
async fn get_my_handle(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .my_handle(info, MyHandleQuery { session_token })
        .await?;
    answer_authorized(&found.map(HandleBody::from))
}

/// Changes the handle of the person who asks, once in a while (`handle.changed-lately` otherwise).
/// The one they had stays theirs: nobody else can take it, and the texts written before still name
/// them, but a new text that uses it names nobody.
#[utoipa::path(
    put,
    path = "/api/v1/me/handle",
    operation_id = "changeMyHandle",
    tag = "people",
    request_body = HandleBody,
    responses((status = 200, description = "The handle now.", body = HandleBody), InvalidAnswer, ConflictAnswer, NotSignedInAnswer, TooSoonAnswer)
)]
async fn change_my_handle(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<HandleBody>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .change_handle(
            info,
            ChangeHandle {
                session_token,
                handle: body.handle,
            },
        )
        .await?;
    answer_authorized(&changed.map(HandleBody::from))
}

/// The routes of the handle.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(get_my_handle, change_my_handle))
}
