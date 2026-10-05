//! Making accounts: the first administrator with the setup code, signing up, invitations, and
//! the rule for who may sign up.

use axum::extract::State;
use axum::response::Response;
use bb_app::auth::{
    AcceptInvitation, AppError, ChangeSignUpPolicy, CompleteSetup, CreateInvitation,
    InvitationLookupQuery, InvitationsQuery, RevokeInvitation, SignUp, SignUpPolicy,
    SignUpPolicyQuery,
};
use bb_core::ErrorCode;
use bb_core::id::InvitationId;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer,
    PreviousSession, Session, SignedInBody, answer, done, signed_in,
};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Page};

/// The first administrator, with the setup code.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SetupBody {
    /// The setup code the server printed.
    code: String,
    /// The name of the administrator.
    display_name: String,
    /// The e-mail address of the administrator.
    email: String,
    /// The password of the administrator.
    password: String,
}

/// Somebody who makes an account on their own.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SignUpBody {
    /// The e-mail address.
    email: String,
    /// The name others see.
    display_name: String,
    /// The password.
    password: String,
}

/// Somebody who makes an account from an invitation.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AcceptBody {
    /// The secret from the invitation link (the part after the `#`).
    secret: String,
    /// The name others see.
    display_name: String,
    /// The password.
    password: String,
}

/// The secret of an invitation link.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SecretBody {
    /// The secret from the link (the part after the `#`).
    secret: String,
}

/// Which address an invitation is for.
#[derive(Serialize, ToSchema)]
pub(crate) struct InvitationLookupBody {
    /// The address the account will get.
    email: String,
}

/// Somebody to invite.
#[derive(Deserialize, ToSchema)]
pub(crate) struct InviteBody {
    /// The e-mail address to invite.
    email: String,
}

/// The fields of an invitation that can be asked for, and which of them a list holds when the
/// request names none: what *Open invitations* needs to find the person to take back.
const INVITATION_FIELDS: [FieldSpec; 4] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("email"),
    FieldSpec::on_request("created_at"),
    FieldSpec::on_request("expires_at"),
];
/// The name of the list, so its cursors are its own.
const INVITATIONS_LIST: &str = "invitations";

/// An invitation in a list: the fields that can be asked for. An answer holds only those that
/// were asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct InvitationItem {
    /// The ID of the invitation.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    /// The address it is for.
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    /// When it was made.
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
    /// When it stops working.
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<String>,
}

impl From<&bb_app::auth::InvitationView> for InvitationItem {
    fn from(invitation: &bb_app::auth::InvitationView) -> Self {
        Self {
            id: Some(invitation.id.to_string()),
            email: Some(invitation.email.clone()),
            created_at: Some(invitation.created_at.to_string()),
            expires_at: Some(invitation.expires_at.to_string()),
        }
    }
}

/// An invitation as the administrator sees it.
#[derive(Serialize, ToSchema)]
pub(crate) struct InvitationBody {
    /// The ID of the invitation.
    id: String,
    /// The address it is for.
    email: String,
    /// When it was made.
    created_at: String,
    /// When it stops working.
    expires_at: String,
}

impl From<&bb_app::auth::InvitationView> for InvitationBody {
    fn from(invitation: &bb_app::auth::InvitationView) -> Self {
        Self {
            id: invitation.id.to_string(),
            email: invitation.email.clone(),
            created_at: invitation.created_at.to_string(),
            expires_at: invitation.expires_at.to_string(),
        }
    }
}

/// A new invitation: the secret is shown only now.
#[derive(Serialize, ToSchema)]
pub(crate) struct CreatedInvitationBody {
    /// The invitation.
    invitation: InvitationBody,
    /// The page the link opens, with the secret after the `#`. Put the address of this server
    /// in front. The secret is not stored and cannot be shown again.
    #[schema(extensions(("x-secret" = json!(true))))]
    link_path: String,
}

/// Who may sign up on their own.
#[derive(Serialize, Deserialize, ToSchema)]
pub(crate) struct SignUpPolicyBody {
    /// Whether signing up without an invitation is on.
    enabled: bool,
    /// The e-mail domains that may sign up. Empty: every domain, while signing up is on.
    domains: Vec<String>,
}

/// Makes the first administrator with the setup code, and signs them in. Setup is closed for
/// good afterwards.
#[utoipa::path(
    post,
    path = "/api/v1/auth/setup",
    operation_id = "completeSetup",
    tag = "auth",
    request_body = SetupBody,
    responses(
        (status = 200, description = "The administrator was made and is signed in.", body = SignedInBody),
        InvalidAnswer,
        (status = 401, description = "The setup code is wrong, or setup is over (auth.wrong_setup_code).", body = bb_core::Problem)
    )
)]
async fn complete_setup(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    PreviousSession(previous_session): PreviousSession,
    ApiJson(body): ApiJson<SetupBody>,
) -> Result<Response, ApiError> {
    let started = auth
        .api()?
        .complete_setup(
            info,
            CompleteSetup {
                code: body.code,
                display_name: body.display_name,
                email: body.email,
                password: body.password,
                previous_session,
            },
        )
        .await?;
    signed_in(&started)
}

/// Makes an account without an invitation, when an administrator allows it.
#[utoipa::path(
    post,
    path = "/api/v1/auth/sign-up",
    operation_id = "signUp",
    tag = "auth",
    request_body = SignUpBody,
    responses(
        (status = 200, description = "The account was made and is signed in.", body = SignedInBody),
        InvalidAnswer,
        (status = 403, description = "Signing up is not possible here (auth.sign_up_refused).", body = bb_core::Problem)
    )
)]
async fn sign_up(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    PreviousSession(previous_session): PreviousSession,
    ApiJson(body): ApiJson<SignUpBody>,
) -> Result<Response, ApiError> {
    let started = auth
        .api()?
        .sign_up(
            info,
            SignUp {
                email: body.email,
                display_name: body.display_name,
                password: body.password,
                previous_session,
            },
        )
        .await?;
    signed_in(&started)
}

/// Shows which address an invitation is for. It is a `POST` so the secret is in the body, not in
/// an address that logs and browser history keep.
#[utoipa::path(
    post,
    path = "/api/v1/auth/invitation/lookup",
    operation_id = "lookUpInvitation",
    tag = "auth",
    request_body = SecretBody,
    responses((status = 200, description = "The invitation works.", body = InvitationLookupBody), InvalidAnswer)
)]
async fn look_up_invitation(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    ApiJson(body): ApiJson<SecretBody>,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .invitation_lookup(
            info,
            InvitationLookupQuery {
                secret: body.secret,
            },
        )
        .await?;
    Ok(answer(InvitationLookupBody { email: found.email }))
}

/// Makes an account from an invitation link, and signs the person in.
#[utoipa::path(
    post,
    path = "/api/v1/auth/invitation/accept",
    operation_id = "acceptInvitation",
    tag = "auth",
    request_body = AcceptBody,
    responses(
        (status = 200, description = "The account was made and is signed in.", body = SignedInBody),
        InvalidAnswer
    )
)]
async fn accept_invitation(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    PreviousSession(previous_session): PreviousSession,
    ApiJson(body): ApiJson<AcceptBody>,
) -> Result<Response, ApiError> {
    let started = auth
        .api()?
        .accept_invitation(
            info,
            AcceptInvitation {
                secret: body.secret,
                display_name: body.display_name,
                password: body.password,
                previous_session,
            },
        )
        .await?;
    signed_in(&started)
}

/// The open invitations, a page at a time. Administrators only.
#[utoipa::path(
    get,
    path = "/api/v1/auth/invitations",
    operation_id = "listInvitations",
    tag = "auth",
    extensions(("x-never-stored" = json!(true))),
    params(ListParams),
    responses((status = 200, description = "The open invitations, newest first. Only invitations that are still open are listed.", body = Page<InvitationItem>), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_invitations(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    // Who asks is judged first (401 and 403 before 400): a person who may not list learns
    // nothing about the fields or cursors of the list. A cursor that does not open is passed on
    // as one the use case cannot read, so its answer comes after the check of the person.
    let fields = params.fields(&INVITATION_FIELDS);
    let position = params.position(INVITATIONS_LIST);
    let cursor = match &position {
        Ok(position) => position.clone(),
        Err(_) => Some(String::new()),
    };
    let page = auth
        .api()?
        .invitations(
            info,
            InvitationsQuery {
                session_token,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(|error| match error {
            // The only thing the use case can find invalid is the cursor.
            AppError::Refused(ErrorCode::InvalidRequest) => ApiError::new(ErrorCode::BadCursor),
            other => ApiError::from(other),
        })?;
    position?;
    let items: Vec<InvitationItem> = page.items.iter().map(InvitationItem::from).collect();
    Ok(answer(Page::of(
        INVITATIONS_LIST,
        &items,
        &fields?,
        page.next_cursor.as_deref(),
    )?))
}

/// Makes an invitation. The link is shown once. Administrators only.
#[utoipa::path(
    post,
    path = "/api/v1/auth/invitations",
    operation_id = "createInvitation",
    tag = "auth",
    request_body = InviteBody,
    responses(
        (status = 200, description = "The invitation with its link.", body = CreatedInvitationBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn create_invitation(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<InviteBody>,
) -> Result<Response, ApiError> {
    let created = auth
        .api()?
        .create_invitation(
            info,
            CreateInvitation {
                session_token,
                email: body.email,
            },
        )
        .await?;
    Ok(answer(CreatedInvitationBody {
        invitation: InvitationBody::from(&created.invitation),
        link_path: created.link_path,
    }))
}

/// Takes an invitation back. Administrators only.
#[utoipa::path(
    delete,
    path = "/api/v1/auth/invitations/{invitation_id}",
    operation_id = "revokeInvitation",
    tag = "auth",
    params(("invitation_id" = String, Path, description = "The ID of the invitation.")),
    responses((status = 204, description = "Taken back."), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn revoke_invitation(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(invitation_id): ApiPath<String>,
) -> Result<Response, ApiError> {
    let invitation_id: InvitationId = invitation_id
        .parse()
        .map_err(|_| ApiError::new(ErrorCode::NotFound))?;
    auth.api()?
        .revoke_invitation(
            info,
            RevokeInvitation {
                session_token,
                invitation_id,
            },
        )
        .await?;
    Ok(done())
}

fn policy_body(policy: SignUpPolicy) -> Response {
    answer(SignUpPolicyBody {
        enabled: policy.enabled,
        domains: policy.domains,
    })
}

/// Who may sign up on their own. Administrators only.
#[utoipa::path(
    get,
    path = "/api/v1/auth/sign-up-policy",
    operation_id = "getSignUpPolicy",
    tag = "auth",
    responses((status = 200, description = "The rule for signing up.", body = SignUpPolicyBody), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn get_sign_up_policy(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let policy = auth
        .api()?
        .sign_up_policy(info, SignUpPolicyQuery { session_token })
        .await?;
    Ok(policy_body(policy))
}

/// Changes who may sign up on their own. Administrators only.
#[utoipa::path(
    put,
    path = "/api/v1/auth/sign-up-policy",
    operation_id = "changeSignUpPolicy",
    tag = "auth",
    request_body = SignUpPolicyBody,
    responses(
        (status = 200, description = "The new rule.", body = SignUpPolicyBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn change_sign_up_policy(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<SignUpPolicyBody>,
) -> Result<Response, ApiError> {
    let policy = auth
        .api()?
        .change_sign_up_policy(
            info,
            ChangeSignUpPolicy {
                session_token,
                policy: SignUpPolicy {
                    enabled: body.enabled,
                    domains: body.domains,
                },
            },
        )
        .await?;
    Ok(policy_body(policy))
}

/// The routes that try a setup code, an invitation or an address: they get the small
/// per-network limit.
pub(crate) fn tried_routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(complete_setup))
        .routes(routes!(sign_up))
        .routes(routes!(look_up_invitation))
        .routes(routes!(accept_invitation))
}

/// The routes of an administrator.
pub(crate) fn session_routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_invitations, create_invitation))
        .routes(routes!(revoke_invitation))
        .routes(routes!(get_sign_up_policy, change_sign_up_policy))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_an_invitation() {
        let item = InvitationItem {
            id: Some("i".to_owned()),
            email: Some("e".to_owned()),
            created_at: Some("c".to_owned()),
            expires_at: Some("x".to_owned()),
        };
        crate::lean::assert_fields_are_the_keys_of(&INVITATION_FIELDS, &item);
    }
}
