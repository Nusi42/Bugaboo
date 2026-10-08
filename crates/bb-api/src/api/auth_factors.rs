//! The second step and passkeys of a signed-in person: the code app, backup codes, and adding and
//! removing passkeys.

use axum::extract::State;
use axum::response::Response;
use bb_app::auth::{
    AddPasskey, ConfirmCodeApp, PasskeyRegistrationQuery, RemoveCodeApp, RemovePasskey,
    RenewBackupCodes, StartCodeApp, TwoStepQuery,
};
use bb_core::ErrorCode;
use bb_core::id::PasskeyId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer, done,
};
use super::auth_sign_in::ChallengeBody;
use crate::errors::{ApiError, ApiJson, ApiPath};

/// A password typed again before a sensitive change.
#[derive(Deserialize, ToSchema)]
pub(crate) struct PasswordOnlyBody {
    /// The password.
    password: String,
}

/// A passkey the person registered.
#[derive(Serialize, ToSchema)]
pub(crate) struct PasskeyBody {
    /// The ID to remove it with.
    id: String,
    /// The name the person gave it.
    label: String,
    /// When it was added.
    created_at: String,
    /// When it was last used to sign in.
    last_used_at: Option<String>,
}

impl From<&bb_app::auth::PasskeyView> for PasskeyBody {
    fn from(passkey: &bb_app::auth::PasskeyView) -> Self {
        Self {
            id: passkey.id.to_string(),
            label: passkey.label.clone(),
            created_at: passkey.created_at.to_string(),
            last_used_at: passkey.last_used_at.map(|time| time.to_string()),
        }
    }
}

/// How the second step of a person is set up.
#[derive(Serialize, ToSchema)]
pub(crate) struct TwoStepBody {
    /// Whether a code app is set up.
    code_app: bool,
    /// How many backup codes are left.
    backup_codes_left: i64,
    /// The passkeys. A person has at most 20, so the list is not paged: it has a hard cap, and
    /// `truncated` says when the cap cut it off (M1-F44).
    passkeys: Vec<PasskeyBody>,
    /// There are more passkeys than the list holds.
    truncated: bool,
}

/// What the setup screen of the code app shows.
#[derive(Serialize, ToSchema)]
pub(crate) struct CodeAppBody {
    /// The address the code app reads.
    otpauth_url: String,
    /// The QR picture of the address (PNG, base64).
    qr_png_base64: String,
    /// The secret as text, for typing it into the app.
    secret: String,
}

/// Backup codes, shown once.
#[derive(Serialize, ToSchema)]
pub(crate) struct BackupCodesBody {
    /// The codes. They are not stored in this form and cannot be shown again.
    codes: Vec<String>,
}

/// The first code of the code app, and the password typed again.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ConfirmCodeBody {
    /// The code the app shows.
    code: String,
    /// The password.
    password: String,
}

/// A passkey the device made, with the name the person gives it.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AddPasskeyBody {
    /// The challenge that was answered.
    challenge_id: String,
    /// The answer of the device, as the browser gives it.
    #[schema(value_type = Object)]
    credential: Value,
    /// The name of the passkey, like "Work laptop".
    label: String,
    /// The password, typed again.
    password: String,
}

/// How the second step is set up.
#[utoipa::path(
    get,
    path = "/api/v1/auth/two-step",
    operation_id = "getTwoStep",
    tag = "auth",
    responses((status = 200, description = "The second step of the person.", body = TwoStepBody), NotSignedInAnswer)
)]
async fn get_two_step(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let status = auth
        .api()?
        .two_step(info, TwoStepQuery { session_token })
        .await?;
    Ok(answer(TwoStepBody {
        code_app: status.code_app,
        backup_codes_left: status.backup_codes_left,
        passkeys: status.passkeys.iter().map(PasskeyBody::from).collect(),
        truncated: status.truncated,
    }))
}

/// Starts setting up the code app. It is not asked for until the first code is confirmed. The
/// password is typed again, so a stolen session cannot add a way to sign in.
#[utoipa::path(
    post,
    path = "/api/v1/auth/two-step/code-app",
    operation_id = "startCodeApp",
    tag = "auth",
    request_body = PasswordOnlyBody,
    responses(
        (status = 200, description = "What the setup screen shows.", body = CodeAppBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn start_code_app(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<PasswordOnlyBody>,
) -> Result<Response, ApiError> {
    let setup = auth
        .api()?
        .start_code_app(
            info,
            StartCodeApp {
                session_token,
                password: body.password,
            },
        )
        .await?;
    Ok(answer(CodeAppBody {
        otpauth_url: setup.otpauth_url,
        qr_png_base64: setup.qr_png_base64,
        secret: setup.secret,
    }))
}

/// Finishes setting up the code app with its first code, and makes the backup codes.
#[utoipa::path(
    post,
    path = "/api/v1/auth/two-step/code-app/confirm",
    operation_id = "confirmCodeApp",
    tag = "auth",
    request_body = ConfirmCodeBody,
    responses(
        (status = 200, description = "Set up; the backup codes are shown once.", body = BackupCodesBody),
        InvalidAnswer,
        (status = 401, description = "The code is wrong (auth.wrong_second_step_code, auth.not_signed_in).", body = bb_core::Problem),
        ForbiddenAnswer
    )
)]
async fn confirm_code_app(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<ConfirmCodeBody>,
) -> Result<Response, ApiError> {
    let codes = auth
        .api()?
        .confirm_code_app(
            info,
            ConfirmCodeApp {
                session_token,
                code: body.code,
                password: body.password,
            },
        )
        .await?;
    Ok(answer(BackupCodesBody { codes: codes.0 }))
}

/// Switches the code app off. The password is typed again.
#[utoipa::path(
    post,
    path = "/api/v1/auth/two-step/code-app/remove",
    operation_id = "removeCodeApp",
    tag = "auth",
    request_body = PasswordOnlyBody,
    responses(
        (status = 204, description = "Switched off."),
        (status = 401, description = "The password is wrong (auth.wrong_credentials, auth.not_signed_in).", body = bb_core::Problem),
        ForbiddenAnswer
    )
)]
async fn remove_code_app(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<PasswordOnlyBody>,
) -> Result<Response, ApiError> {
    auth.api()?
        .remove_code_app(
            info,
            RemoveCodeApp {
                session_token,
                password: body.password,
            },
        )
        .await?;
    Ok(done())
}

/// Makes new backup codes; the old ones stop working. The password is typed again.
#[utoipa::path(
    post,
    path = "/api/v1/auth/two-step/backup-codes",
    operation_id = "renewBackupCodes",
    tag = "auth",
    request_body = PasswordOnlyBody,
    responses(
        (status = 200, description = "The new codes, shown once.", body = BackupCodesBody),
        InvalidAnswer,
        (status = 401, description = "The password is wrong (auth.wrong_credentials, auth.not_signed_in).", body = bb_core::Problem),
        ForbiddenAnswer
    )
)]
async fn renew_backup_codes(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<PasswordOnlyBody>,
) -> Result<Response, ApiError> {
    let codes = auth
        .api()?
        .renew_backup_codes(
            info,
            RenewBackupCodes {
                session_token,
                password: body.password,
            },
        )
        .await?;
    Ok(answer(BackupCodesBody { codes: codes.0 }))
}

/// Starts adding a passkey. The password is typed again, so a stolen session cannot add a way
/// to sign in.
#[utoipa::path(
    post,
    path = "/api/v1/auth/passkeys/options",
    operation_id = "startAddPasskey",
    tag = "auth",
    request_body = PasswordOnlyBody,
    responses((status = 200, description = "The challenge for the device.", body = ChallengeBody), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn passkey_registration_options(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<PasswordOnlyBody>,
) -> Result<Response, ApiError> {
    let challenge = auth
        .api()?
        .passkey_registration_options(
            info,
            PasskeyRegistrationQuery {
                session_token,
                password: body.password,
            },
        )
        .await?;
    Ok(answer(ChallengeBody::from(challenge)))
}

/// Adds a passkey.
#[utoipa::path(
    post,
    path = "/api/v1/auth/passkeys",
    operation_id = "addPasskey",
    tag = "auth",
    request_body = AddPasskeyBody,
    responses(
        (status = 200, description = "Added.", body = PasskeyBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn add_passkey(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<AddPasskeyBody>,
) -> Result<Response, ApiError> {
    let passkey = auth
        .api()?
        .add_passkey(
            info,
            AddPasskey {
                session_token,
                challenge_id: body.challenge_id,
                credential: body.credential,
                label: body.label,
                password: body.password,
            },
        )
        .await?;
    Ok(answer(PasskeyBody::from(&passkey)))
}

/// Removes a passkey. The password is typed again.
#[utoipa::path(
    post,
    path = "/api/v1/auth/passkeys/{passkey_id}/remove",
    operation_id = "removePasskey",
    tag = "auth",
    params(("passkey_id" = String, Path, description = "The ID of the passkey.")),
    request_body = PasswordOnlyBody,
    responses(
        (status = 204, description = "Removed."),
        (status = 401, description = "The password is wrong (auth.wrong_credentials, auth.not_signed_in).", body = bb_core::Problem),
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn remove_passkey(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(passkey_id): ApiPath<String>,
    ApiJson(body): ApiJson<PasswordOnlyBody>,
) -> Result<Response, ApiError> {
    let passkey_id: PasskeyId = passkey_id
        .parse()
        .map_err(|_| ApiError::new(ErrorCode::NotFound))?;
    auth.api()?
        .remove_passkey(
            info,
            RemovePasskey {
                session_token,
                passkey_id,
                password: body.password,
            },
        )
        .await?;
    Ok(done())
}

/// The routes of a person who is signed in.
pub(crate) fn session_routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_two_step))
        .routes(routes!(start_code_app))
        .routes(routes!(confirm_code_app))
        .routes(routes!(remove_code_app))
        .routes(routes!(renew_backup_codes))
        .routes(routes!(passkey_registration_options))
        .routes(routes!(add_passkey))
        .routes(routes!(remove_passkey))
}
