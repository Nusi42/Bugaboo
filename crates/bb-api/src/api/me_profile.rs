//! The profile of the person who asks (PRIV-06, ADR-0044, decision 9): `/api/v1/me/name`, their
//! name, changed at once, and `/api/v1/me/e-mail-change`, a new e-mail address, which counts once
//! the link sent to it is opened at `/api/v1/auth/e-mail-change/confirm`.
//!
//! The name and the address are the person's own, so only a session reaches them: no access key
//! and no agent. The link needs no session: its secret proves the new address.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse as _, Response};
use bb_app::people::{AskEmailChange, ChangeMyName, ConfirmEmailChange, MyName};
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::NameBody;
use super::auth::{
    AuthHandle, Info, InvalidAnswer, MailOffAnswer, NotSignedInAnswer, Session, TooSoonAnswer,
    answer_authorized,
};
use super::auth_people::SecretBody;
use crate::errors::{ApiError, ApiJson};

/// A new e-mail address for the person who asks, with their password.
#[derive(Deserialize, ToSchema)]
pub(crate) struct EmailChangeBody {
    /// The new address, which a link goes to.
    new_email: String,
    /// The password, asked again so a stolen session cannot move the account to another address.
    password: String,
}

/// The answer of a change of one's own name.
fn name_body(changed: MyName) -> NameBody {
    NameBody { name: changed.name }
}

/// Changes the name of the person who asks, at once. The history keeps the change.
#[utoipa::path(
    put,
    path = "/api/v1/me/name",
    operation_id = "changeMyName",
    tag = "people",
    request_body = NameBody,
    responses((status = 200, description = "The name now.", body = NameBody), InvalidAnswer, NotSignedInAnswer)
)]
async fn change_my_name(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NameBody>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .change_my_name(
            info,
            ChangeMyName {
                session_token,
                name: body.name,
            },
        )
        .await?;
    answer_authorized(&changed.map(name_body))
}

/// Asks for a new e-mail address: a link goes to it (one hour, once). The answer is `202` for
/// every address, whether another account has it or not; the old address works until the link is
/// opened. While e-mail is off, the answer is `e-mail-change.mail-off`: an administrator changes
/// the address then.
#[utoipa::path(
    post,
    path = "/api/v1/me/e-mail-change",
    operation_id = "askEmailChange",
    tag = "people",
    request_body = EmailChangeBody,
    responses(
        (status = 202, description = "An e-mail goes to the new address, the same answer for every address."),
        InvalidAnswer,
        NotSignedInAnswer,
        MailOffAnswer,
        TooSoonAnswer
    )
)]
async fn ask_e_mail_change(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<EmailChangeBody>,
) -> Result<Response, ApiError> {
    auth.api()?
        .ask_e_mail_change(
            info,
            AskEmailChange {
                session_token,
                password: body.password,
                email: body.new_email,
            },
        )
        .await?;
    Ok(StatusCode::ACCEPTED.into_response())
}

/// Opens the link to a new e-mail address: from now on it is the address of the account, and the
/// old address is told. It is a `POST` so the secret is in the body, not in an address that logs
/// and browser history keep.
#[utoipa::path(
    post,
    path = "/api/v1/auth/e-mail-change/confirm",
    operation_id = "confirmEmailChange",
    tag = "auth",
    request_body = SecretBody,
    responses((status = 204, description = "The new address is the account's."), InvalidAnswer)
)]
async fn confirm_e_mail_change(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    ApiJson(body): ApiJson<SecretBody>,
) -> Result<Response, ApiError> {
    auth.api()?
        .confirm_e_mail_change(
            info,
            ConfirmEmailChange {
                secret: body.secret,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// The routes of the profile.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(change_my_name))
        .routes(routes!(ask_e_mail_change))
        .routes(routes!(confirm_e_mail_change))
}
