//! Signing in and out and the person's sessions: password, second step, passkey sign-in,
//! *Where you're signed in*, and changing the password.

use axum::extract::State;
use axum::response::Response;
use bb_app::auth::{
    AuthenticateQuery, ChangePassword, EndSession, PasskeySecondStep, PasskeySecondStepQuery,
    PasskeySignIn, PasskeySignInQuery, SecondStep, SessionsQuery, SignIn, SignOut,
    SignOutEverywhere, StatusQuery,
};
use bb_core::ErrorCode;
use bb_core::id::SessionId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer,
    PersonBody, PreviousSession, Session, SignedInBody, answer, done, signed_in,
};
use crate::cookies::Cookie;
use crate::errors::{ApiError, ApiJson, ApiPath};

/// What a person types to sign in.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SignInBody {
    /// The e-mail address.
    email: String,
    /// The password.
    password: String,
}

/// The code of the second step.
#[derive(Deserialize, ToSchema)]
pub(crate) struct CodeBody {
    /// A code from the code app (six digits) or a backup code.
    pub(crate) code: String,
}

/// A challenge for a passkey, as the passkey library writes it.
#[derive(Serialize, ToSchema)]
pub(crate) struct ChallengeBody {
    /// Names the waiting challenge; hand it back with the answer.
    pub(crate) challenge_id: String,
    /// What the device is asked, to give to `navigator.credentials`.
    #[schema(value_type = Object)]
    pub(crate) options: Value,
}

impl From<bb_app::auth::PasskeyChallenge> for ChallengeBody {
    fn from(challenge: bb_app::auth::PasskeyChallenge) -> Self {
        Self {
            challenge_id: challenge.challenge_id,
            options: challenge.options,
        }
    }
}

/// What the device answered to a passkey challenge.
#[derive(Deserialize, ToSchema)]
pub(crate) struct PasskeyAnswerBody {
    /// The challenge that was answered.
    challenge_id: String,
    /// The answer of the device, as the browser gives it.
    #[schema(value_type = Object)]
    credential: Value,
}

/// The new password with the current one.
#[derive(Deserialize, ToSchema)]
pub(crate) struct PasswordBody {
    /// The password now.
    current_password: String,
    /// The password from now on.
    new_password: String,
}

/// What the app needs to know before anybody signs in.
#[derive(Serialize, ToSchema)]
pub(crate) struct AuthStatusBody {
    /// No administrator exists yet: the first-start setup with the setup code is open.
    setup_needed: bool,
    /// People may sign up on their own.
    sign_up_open: bool,
    /// Passkeys can be used: the public address of the server has a host name.
    passkeys_available: bool,
}

/// One session in *Where you're signed in*.
#[derive(Serialize, ToSchema)]
pub(crate) struct SessionBody {
    /// The ID to end it with.
    id: String,
    /// When the person signed in.
    started_at: String,
    /// The last time it was used.
    last_seen_at: String,
    /// The network it came from, shortened.
    network: Option<String>,
    /// The name of the browser, if known.
    browser: Option<String>,
    /// The name of the system, if known.
    system: Option<String>,
    /// Whether this is the session of the request.
    current: bool,
}

/// The sessions of a person.
#[derive(Serialize, ToSchema)]
pub(crate) struct SessionsBody {
    /// Newest first. A person has only a few, so the list is not paged: it has a hard cap, and
    /// `truncated` says when the cap cut it off (M1-F44).
    sessions: Vec<SessionBody>,
    /// There are more sessions than the list holds.
    truncated: bool,
}

/// Whether setup is open and whether people may sign up. Open to everyone.
#[utoipa::path(
    get,
    path = "/api/v1/auth/status",
    operation_id = "getAuthStatus",
    tag = "auth",
    responses((status = 200, description = "What the app needs before anybody signs in.", body = AuthStatusBody))
)]
async fn get_auth_status(
    State(auth): State<AuthHandle>,
    Info(info): Info,
) -> Result<Response, ApiError> {
    let status = auth.api()?.status(info, StatusQuery).await?;
    Ok(answer(AuthStatusBody {
        setup_needed: status.setup_needed,
        sign_up_open: status.sign_up_open,
        passkeys_available: status.passkeys_available,
    }))
}

/// Signs in with e-mail address and password. An unknown address and a wrong password get the
/// same answer. When the person has a second step, the session can only finish it.
#[utoipa::path(
    post,
    path = "/api/v1/auth/sign-in",
    operation_id = "signIn",
    tag = "auth",
    request_body = SignInBody,
    responses(
        (status = 200, description = "Signed in, or the second step is still to do.", body = SignedInBody),
        InvalidAnswer,
        (status = 401, description = "The e-mail address or the password is wrong (auth.wrong_credentials).", body = bb_core::Problem)
    )
)]
async fn sign_in(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    PreviousSession(previous_session): PreviousSession,
    ApiJson(body): ApiJson<SignInBody>,
) -> Result<Response, ApiError> {
    let started = auth
        .api()?
        .sign_in(
            info,
            SignIn {
                email: body.email,
                password: body.password,
                previous_session,
            },
        )
        .await?;
    signed_in(&started)
}

/// Finishes signing in with a code from the code app or a backup code.
#[utoipa::path(
    post,
    path = "/api/v1/auth/second-step",
    operation_id = "finishSignInWithCode",
    tag = "auth",
    request_body = CodeBody,
    responses(
        (status = 200, description = "Signed in.", body = SignedInBody),
        (status = 401, description = "The code is wrong or was used already, or there is no half finished sign-in (auth.wrong_second_step_code, auth.not_signed_in).", body = bb_core::Problem),
        ForbiddenAnswer
    )
)]
async fn second_step(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<CodeBody>,
) -> Result<Response, ApiError> {
    let started = auth
        .api()?
        .second_step(
            info,
            SecondStep {
                session_token,
                code: body.code,
            },
        )
        .await?;
    signed_in(&started)
}

/// Starts checking a passkey as the second step of signing in.
#[utoipa::path(
    post,
    path = "/api/v1/auth/second-step/passkey/options",
    operation_id = "startSecondStepWithPasskey",
    tag = "auth",
    responses(
        (status = 200, description = "The challenge for the device.", body = ChallengeBody),
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn second_step_passkey_options(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let challenge = auth
        .api()?
        .passkey_second_step_options(info, PasskeySecondStepQuery { session_token })
        .await?;
    Ok(answer(ChallengeBody::from(challenge)))
}

/// Finishes signing in with a passkey after the password.
#[utoipa::path(
    post,
    path = "/api/v1/auth/second-step/passkey",
    operation_id = "finishSignInWithPasskey",
    tag = "auth",
    request_body = PasskeyAnswerBody,
    responses(
        (status = 200, description = "Signed in.", body = SignedInBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn second_step_passkey(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<PasskeyAnswerBody>,
) -> Result<Response, ApiError> {
    let started = auth
        .api()?
        .passkey_second_step(
            info,
            PasskeySecondStep {
                session_token,
                challenge_id: body.challenge_id,
                credential: body.credential,
            },
        )
        .await?;
    signed_in(&started)
}

/// Starts signing in with a passkey alone: the device offers its passkeys for this server.
#[utoipa::path(
    post,
    path = "/api/v1/auth/passkey/sign-in/options",
    operation_id = "startPasskeySignIn",
    tag = "auth",
    responses((status = 200, description = "The challenge for the device.", body = ChallengeBody))
)]
async fn passkey_sign_in_options(
    State(auth): State<AuthHandle>,
    Info(info): Info,
) -> Result<Response, ApiError> {
    let challenge = auth
        .api()?
        .passkey_sign_in_options(info, PasskeySignInQuery)
        .await?;
    Ok(answer(ChallengeBody::from(challenge)))
}

/// Signs in with a passkey alone.
#[utoipa::path(
    post,
    path = "/api/v1/auth/passkey/sign-in",
    operation_id = "signInWithPasskey",
    tag = "auth",
    request_body = PasskeyAnswerBody,
    responses(
        (status = 200, description = "Signed in.", body = SignedInBody),
        InvalidAnswer
    )
)]
async fn passkey_sign_in(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    PreviousSession(previous_session): PreviousSession,
    ApiJson(body): ApiJson<PasskeyAnswerBody>,
) -> Result<Response, ApiError> {
    let started = auth
        .api()?
        .passkey_sign_in(
            info,
            PasskeySignIn {
                challenge_id: body.challenge_id,
                credential: body.credential,
                previous_session,
            },
        )
        .await?;
    signed_in(&started)
}

/// Ends the session of the request and clears both cookies.
#[utoipa::path(
    post,
    path = "/api/v1/auth/sign-out",
    operation_id = "signOut",
    tag = "auth",
    responses((status = 204, description = "Signed out."), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn sign_out(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    auth.api()?
        .sign_out(info, SignOut { session_token })
        .await?;
    Ok(forget_cookies(done()))
}

/// Who is signed in.
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    operation_id = "getMe",
    tag = "auth",
    extensions(("x-never-stored" = json!(true))),
    responses((status = 200, description = "The signed-in person.", body = PersonBody), NotSignedInAnswer)
)]
async fn get_me(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let me = auth
        .api()?
        .authenticate(info, AuthenticateQuery { session_token })
        .await?;
    Ok(answer(PersonBody::from(&me.person)))
}

/// The sessions of the signed-in person.
#[utoipa::path(
    get,
    path = "/api/v1/auth/sessions",
    operation_id = "listSessions",
    tag = "auth",
    extensions(("x-never-stored" = json!(true))),
    responses((status = 200, description = "Where the person is signed in.", body = SessionsBody), NotSignedInAnswer)
)]
async fn list_sessions(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let sessions = auth
        .api()?
        .sessions(info, SessionsQuery { session_token })
        .await?;
    Ok(answer(SessionsBody {
        truncated: sessions.truncated,
        sessions: sessions
            .sessions
            .iter()
            .map(|session| SessionBody {
                id: session.id.to_string(),
                started_at: session.started_at.to_string(),
                last_seen_at: session.last_seen_at.to_string(),
                network: session.network.clone(),
                browser: session.browser.clone(),
                system: session.system.clone(),
                current: session.current,
            })
            .collect(),
    }))
}

/// Ends every session of the person, the one of the request included ("sign out everywhere").
#[utoipa::path(
    delete,
    path = "/api/v1/auth/sessions",
    operation_id = "signOutEverywhere",
    tag = "auth",
    responses((status = 204, description = "Every session ended."), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn sign_out_everywhere(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    auth.api()?
        .sign_out_everywhere(info, SignOutEverywhere { session_token })
        .await?;
    Ok(forget_cookies(done()))
}

/// Ends one session of the person.
#[utoipa::path(
    delete,
    path = "/api/v1/auth/sessions/{session_id}",
    operation_id = "endSession",
    tag = "auth",
    params(("session_id" = String, Path, description = "The ID of the session.")),
    responses((status = 204, description = "The session ended."), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn end_session(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(session_id): ApiPath<String>,
) -> Result<Response, ApiError> {
    let session_id: SessionId = session_id
        .parse()
        .map_err(|_| ApiError::new(ErrorCode::NotFound))?;
    auth.api()?
        .end_session(
            info,
            EndSession {
                session_token,
                session_id,
            },
        )
        .await?;
    Ok(done())
}

/// Changes the password. Other sessions end and the session of the request gets a new secret.
#[utoipa::path(
    post,
    path = "/api/v1/auth/password",
    operation_id = "changePassword",
    tag = "auth",
    request_body = PasswordBody,
    responses(
        (status = 200, description = "Changed; new cookies come with it.", body = SignedInBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn change_password(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<PasswordBody>,
) -> Result<Response, ApiError> {
    let started = auth
        .api()?
        .change_password(
            info,
            ChangePassword {
                session_token,
                current_password: body.current_password,
                new_password: body.new_password,
            },
        )
        .await?;
    signed_in(&started)
}

/// Makes the browser forget both cookies.
fn forget_cookies(mut response: Response) -> Response {
    Cookie::Session.forget(&mut response);
    Cookie::Csrf.forget(&mut response);
    response
}

/// The routes that try a password, a code or a passkey: they get the small per-network limit.
pub(crate) fn tried_routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(sign_in))
        .routes(routes!(second_step))
        .routes(routes!(second_step_passkey_options))
        .routes(routes!(second_step_passkey))
        .routes(routes!(passkey_sign_in_options))
        .routes(routes!(passkey_sign_in))
}

/// The routes of a person who is signed in.
pub(crate) fn session_routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_auth_status))
        .routes(routes!(sign_out))
        .routes(routes!(get_me))
        .routes(routes!(list_sessions, sign_out_everywhere))
        .routes(routes!(end_session))
        .routes(routes!(change_password))
}
