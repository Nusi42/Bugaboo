//! How an AI app signs in (AI-22, AI-24, ADR-0042): the routes of the authorization server inside
//! Bugaboo.
//!
//! Two kinds of route. The ones apps call stand outside `/api/v1`, because the standards fix their
//! addresses and their answers: the two metadata documents below `/.well-known/` (RFC 9728 and
//! RFC 8414), the registration at `/oauth/register` (RFC 7591) and the token endpoint at
//! `/oauth/token` (RFC 6749). They answer in the words of OAuth (`{"error": "invalid_grant"}`), not
//! in problem+json, and count the failed tries per network. The ones the browser app calls stand
//! below `/api/v1/oauth`: the consent step, *Connected apps* with *Take back*, and the list of web
//! addresses an administrator allowed. They are open to a session only (a token of an app or an
//! access key opens none of them).
//!
//! A token an app got is no session: the `Session` extractor does not take one from a cookie, and
//! the API takes none from `Authorization`; it works at `/mcp` only.

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse as _, Response};
use axum::routing::{get, post};
use bb_app::auth::{
    AddWebAddress, AuthorizationQuery, AuthorizationRequest, ConnectedAppView, ConnectedAppsQuery,
    DecideAuthorization, OAuthMetadataQuery, RegisterApp, RegisterOutcome, RemoveWebAddress,
    TakeBack, TokenOutcome, TokenRequest, WebAddressesQuery,
};
use bb_core::access_key::KeyChoice;
use bb_core::id::{GrantId, ProjectId};
use bb_core::oauth::{
    AUTHORIZATION_SERVER_METADATA_PATH, OAuthProblem, PKCE_METHOD,
    PROTECTED_RESOURCE_METADATA_PATH, REGISTRATION_PATH, TOKEN_PATH, form_pairs,
};
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_keys::ProjectLabelBody;
use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized, done,
};
use super::{body_id, path_id};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery, json_no_store, with_retry_after};
use crate::lean::{FieldSpec, ListParams, Listed, Page};
use crate::logging::Scheme;

/// The fields of an app a person allowed that can be asked for.
const APP_FIELDS: [FieldSpec; 7] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("on_this_computer"),
    FieldSpec::by_default("scope"),
    FieldSpec::by_default("allowed_at"),
    FieldSpec::by_default("last_used_at"),
    FieldSpec::on_request("projects"),
];
/// The name of the list, so its cursors are its own.
const APPS_LIST: &str = "connected-apps";

// ---- The routes of apps, outside `/api/v1` ----

/// The address the request came to, for a server that does not know its public address.
fn origin_seen(headers: &HeaderMap, scheme: Option<&Scheme>) -> String {
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    format!("{}://{host}", scheme.map_or("http", |scheme| scheme.0))
}

/// The address a request came to, for the code that reads requests outside this crate (the MCP
/// server).
#[must_use]
pub fn origin_of(parts: &axum::http::request::Parts) -> String {
    origin_seen(&parts.headers, parts.extensions.get::<Scheme>())
}

/// The answer to an app that was turned away: the error of OAuth.
fn oauth_error(problem: OAuthProblem) -> Response {
    let status = match problem {
        OAuthProblem::TemporarilyUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        OAuthProblem::InvalidClient => StatusCode::UNAUTHORIZED,
        _ => StatusCode::BAD_REQUEST,
    };
    json_no_store(status, &json!({ "error": problem.code() }))
}

/// A failure the app is told in the words of OAuth: a limit as a `429` with the time to wait, and
/// anything else as the error of the server.
fn app_failure(error: bb_app::auth::AppError) -> Response {
    match error {
        bb_app::auth::AppError::Limited { retry_after } => {
            let mut response = json_no_store(
                StatusCode::TOO_MANY_REQUESTS,
                &json!({ "error": "slow_down" }),
            );
            with_retry_after(&mut response, retry_after);
            response
        }
        other => ApiError::from(other).into_response(),
    }
}

/// The protected-resource metadata (RFC 9728): which server signs apps in for `/mcp`.
async fn protected_resource_metadata(
    State(auth): State<AuthHandle>,
    Info(info): Info,
) -> Result<Response, ApiError> {
    let seen = auth.api()?.oauth_metadata(info, OAuthMetadataQuery).await?;
    Ok(axum::Json(json!({
        "resource": seen.resource,
        "authorization_servers": [seen.origin],
        "bearer_methods_supported": ["header"],
        "scopes_supported": KeyChoice::APP_CHOICES.map(KeyChoice::code),
    }))
    .into_response())
}

/// The authorization-server metadata (RFC 8414): where an app registers, sends the person and
/// swaps a code.
async fn authorization_server_metadata(
    State(auth): State<AuthHandle>,
    Info(info): Info,
) -> Result<Response, ApiError> {
    let seen = auth.api()?.oauth_metadata(info, OAuthMetadataQuery).await?;
    let origin = &seen.origin;
    Ok(axum::Json(json!({
        "issuer": origin,
        "authorization_endpoint": format!("{origin}{}", bb_core::app_paths::CONNECT),
        "token_endpoint": format!("{origin}{TOKEN_PATH}"),
        "registration_endpoint": format!("{origin}{REGISTRATION_PATH}"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": [PKCE_METHOD],
        "token_endpoint_auth_methods_supported": ["none"],
        "scopes_supported": KeyChoice::APP_CHOICES.map(KeyChoice::code),
    }))
    .into_response())
}

/// What an app sends to register itself (RFC 7591); more members are allowed and left alone.
#[derive(Deserialize)]
struct RegistrationBody {
    client_name: Option<String>,
    #[serde(default)]
    redirect_uris: Vec<String>,
}

/// Registers an app. The app is a public client: it holds no secret and proves its codes with PKCE.
async fn register_app(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    body: Result<ApiJson<RegistrationBody>, ApiError>,
) -> Result<Response, ApiError> {
    let Ok(ApiJson(body)) = body else {
        return Ok(oauth_error(OAuthProblem::InvalidClientMetadata));
    };
    let outcome = auth
        .api()?
        .register_app(
            info,
            RegisterApp {
                name: body.client_name.unwrap_or_default(),
                return_addresses: body.redirect_uris,
            },
        )
        .await;
    Ok(match outcome {
        Ok(RegisterOutcome::Registered(app)) => json_no_store(
            StatusCode::CREATED,
            &json!({
                "client_id": app.client_id.to_string(),
                "client_name": app.name,
                "redirect_uris": app.return_addresses,
                "client_id_issued_at": app.registered_at.as_second(),
                "token_endpoint_auth_method": "none",
                "grant_types": ["authorization_code", "refresh_token"],
                "response_types": ["code"],
            }),
        ),
        Ok(RegisterOutcome::Refused(problem)) => oauth_error(problem),
        Err(error) => app_failure(error),
    })
}

/// Swaps a code or a renewing token for tokens. The body is a form (RFC 6749 §4.1.3).
async fn exchange_token(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    body: String,
) -> Result<Response, ApiError> {
    let Some(pairs) = form_pairs(&body) else {
        return Ok(oauth_error(OAuthProblem::InvalidRequest));
    };
    let field = |name: &str| {
        pairs
            .iter()
            .find(|(each, _)| each == name)
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    };
    let outcome = auth
        .api()?
        .exchange_token(
            info,
            TokenRequest {
                grant_type: field("grant_type"),
                client_id: field("client_id"),
                code: field("code").into(),
                return_address: field("redirect_uri"),
                verifier: field("code_verifier").into(),
                refresh_token: field("refresh_token").into(),
                resource: field("resource"),
            },
        )
        .await;
    Ok(match outcome {
        Ok(TokenOutcome::Issued(tokens)) => json_no_store(
            StatusCode::OK,
            &json!({
                "access_token": tokens.access_token.as_str(),
                "token_type": "Bearer",
                "expires_in": tokens.expires_in,
                "refresh_token": tokens.refresh_token.as_str(),
                "scope": tokens.scope.code(),
            }),
        ),
        Ok(TokenOutcome::Refused(problem)) => oauth_error(problem),
        Err(error) => app_failure(error),
    })
}

/// The routes apps call, outside the API.
pub(crate) fn app_routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    Router::new()
        .route(
            PROTECTED_RESOURCE_METADATA_PATH,
            get(protected_resource_metadata),
        )
        .route(
            AUTHORIZATION_SERVER_METADATA_PATH,
            get(authorization_server_metadata),
        )
        .route(REGISTRATION_PATH, post(register_app))
        .route(TOKEN_PATH, post(exchange_token))
}

// ---- The routes of the browser app, below `/api/v1/oauth` ----

/// What an app asked for when it sent the person to the consent step.
#[derive(Deserialize, Serialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct AuthorizationParams {
    /// The ID of the app, as it registered (`client_id` in the words of OAuth).
    client: String,
    /// `code`.
    response_type: String,
    /// The address to come back to (`redirect_uri`).
    redirect_uri: String,
    /// The PKCE challenge.
    code_challenge: String,
    /// `S256`.
    code_challenge_method: String,
    /// What the app asks to be allowed: `see-only` or `see-and-change`.
    #[serde(default)]
    scope: String,
    /// The resource the token is for.
    #[serde(default)]
    resource: String,
    /// What the app wants back unchanged.
    #[serde(default)]
    state: String,
}

impl From<AuthorizationParams> for AuthorizationRequest {
    fn from(params: AuthorizationParams) -> Self {
        Self {
            client_id: params.client,
            response_type: params.response_type,
            return_address: params.redirect_uri,
            challenge: params.code_challenge,
            challenge_method: params.code_challenge_method,
            scope: params.scope,
            resource: params.resource,
            state: params.state,
        }
    }
}

/// What the consent step shows before the person decides.
#[derive(Serialize, ToSchema)]
pub(crate) struct AuthorizationBody {
    /// The name the app gave itself. Bugaboo cannot check it.
    app_name: String,
    /// Whether the app comes back to the person's own computer.
    on_this_computer: bool,
    /// The web address the app comes back to, when it is not this computer.
    web_address: Option<String>,
    /// What the app asked for: the person may narrow it, never widen it.
    asked: KeyChoice,
}

impl Masked for AuthorizationBody {}

/// The person's answer to the consent step.
#[derive(Deserialize, ToSchema)]
pub(crate) struct DecisionBody {
    /// What the app asked.
    request: AuthorizationParams,
    /// `true` for *Allow*, `false` for *Cancel*.
    allow: bool,
    /// What the person allows: `see-only` or `see-and-change`, at most what the app asked.
    scope: KeyChoice,
    /// The projects the app is limited to; leave empty for every project the person can see. The
    /// app counts in them and in their sub-projects.
    #[serde(default)]
    projects: Vec<String>,
}

/// Where the browser goes next.
#[derive(Serialize, ToSchema)]
pub(crate) struct DecisionAnswerBody {
    /// The address of the app with the code or the refusal in it.
    redirect_to: String,
}

impl Masked for DecisionAnswerBody {}

/// What the consent step shows: the app, where it comes back to and what it asks. Only a session
/// opens it.
#[utoipa::path(
    get,
    path = "/api/v1/oauth/authorization",
    operation_id = "getAuthorization",
    tag = "oauth",
    params(AuthorizationParams),
    responses(
        (status = 200, description = "The app and what it asks.", body = AuthorizationBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn get_authorization(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    params: Result<ApiQuery<AuthorizationParams>, ApiError>,
) -> Result<Response, ApiError> {
    let ApiQuery(params) = auth.after_who_asks(&info, &session_token, params).await?;
    let seen = auth
        .api()?
        .authorization(
            info,
            AuthorizationQuery {
                session_token,
                request: params.into(),
            },
        )
        .await?;
    answer_authorized(&seen.map(|seen| AuthorizationBody {
        app_name: seen.app_name,
        on_this_computer: seen.on_this_computer,
        web_address: seen.web_address,
        asked: seen.asked,
    }))
}

/// *Allow* or *Cancel*: makes the grant and a code, or sends the app away. The answer is where the
/// browser goes next.
#[utoipa::path(
    post,
    path = "/api/v1/oauth/authorization",
    operation_id = "decideAuthorization",
    tag = "oauth",
    request_body = DecisionBody,
    responses(
        (status = 200, description = "Where the browser goes next.", body = DecisionAnswerBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn decide_authorization(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<DecisionBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| {
        let projects = body
            .projects
            .iter()
            .map(|project| body_id::<ProjectId>(project))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((body, projects))
    });
    let (body, projects) = auth.after_who_asks(&info, &session_token, read).await?;
    let answer = auth
        .api()?
        .decide_authorization(
            info,
            DecideAuthorization {
                session_token,
                request: body.request.into(),
                allow: body.allow,
                scope: body.scope,
                projects,
            },
        )
        .await?;
    answer_authorized(&answer.map(|answer| DecisionAnswerBody {
        redirect_to: answer.redirect_to.to_string(),
    }))
}

/// An app a person allowed, in a list: the fields that can be asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct ConnectedAppItem {
    /// The ID to take it back with.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    /// The name the app gave itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Whether the app is on the person's own computer.
    #[serde(skip_serializing_if = "Option::is_none")]
    on_this_computer: Option<bool>,
    /// What it may do.
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<KeyChoice>,
    /// When the person allowed it.
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_at: Option<String>,
    /// The last time it made a call, to the minute. Left out when it never did.
    #[serde(skip_serializing_if = "Option::is_none")]
    last_used_at: Option<String>,
    /// The projects it is limited to; empty for every project.
    #[serde(skip_serializing_if = "Option::is_none")]
    projects: Option<Vec<ProjectLabelBody>>,
}

impl Masked for ConnectedAppItem {}

impl From<&ConnectedAppView> for ConnectedAppItem {
    fn from(app: &ConnectedAppView) -> Self {
        Self {
            id: Some(app.id.to_string()),
            name: Some(app.name.clone()),
            on_this_computer: Some(app.on_this_computer),
            scope: Some(app.scope),
            allowed_at: Some(app.allowed_at.to_string()),
            last_used_at: app.last_used_at.map(|moment| moment.to_string()),
            projects: Some(app.projects.iter().map(ProjectLabelBody::from).collect()),
        }
    }
}

/// The AI apps the person allowed, newest first.
#[utoipa::path(
    get,
    path = "/api/v1/oauth/apps",
    operation_id = "listConnectedApps",
    tag = "oauth",
    params(ListParams),
    responses((status = 200, description = "The apps the person allowed.", body = Page<ConnectedAppItem>), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_connected_apps(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let apps = auth
        .api()?
        .connected_apps(info, ConnectedAppsQuery { session_token })
        .await?;
    let list = apps.map(|apps| Listed {
        items: apps.items.iter().map(ConnectedAppItem::from).collect(),
        next: None,
    });
    let fields = params.fields_readable(&APP_FIELDS, &list.hidden_properties())?;
    Ok(super::auth::answer(Page::of_authorized(
        APPS_LIST, &list, &fields,
    )?))
}

/// *Take back*: ends what the person allowed an app, at once.
#[utoipa::path(
    delete,
    path = "/api/v1/oauth/apps/{app}",
    operation_id = "takeBackApp",
    tag = "oauth",
    params(("app" = String, Path, description = "The ID of the app, as the list of connected apps gives it.")),
    responses((status = 204, description = "Taken back."), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn take_back_app(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(app): ApiPath<String>,
) -> Result<Response, ApiError> {
    let id: GrantId = auth
        .after_who_asks(&info, &session_token, path_id(&app))
        .await?;
    auth.api()?
        .take_back(info, TakeBack { session_token, id })
        .await?;
    Ok(done())
}

/// The fields of an allowed web address that can be asked for.
const ADDRESS_FIELDS: [FieldSpec; 2] = [
    FieldSpec::by_default("address"),
    FieldSpec::by_default("added_at"),
];
/// The name of the list, so its cursors are its own.
const ADDRESSES_LIST: &str = "app-addresses";

/// A web address apps may come back to, in a list: the fields that can be asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct WebAddressItem {
    /// The address, as an app has to write it.
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<String>,
    /// When an administrator allowed it.
    #[serde(skip_serializing_if = "Option::is_none")]
    added_at: Option<String>,
}

impl Masked for WebAddressItem {}

impl From<&bb_app::auth::WebAddressView> for WebAddressItem {
    fn from(each: &bb_app::auth::WebAddressView) -> Self {
        Self {
            address: Some(each.address.clone()),
            added_at: Some(each.added_at.to_string()),
        }
    }
}

/// A web address to allow or to take away.
#[derive(Deserialize, ToSchema)]
pub(crate) struct WebAddressRequest {
    /// The return address, such as `https://app.example.com/callback`.
    address: String,
}

/// The web addresses an administrator allowed (*Apps that may sign in*), by address.
#[utoipa::path(
    get,
    path = "/api/v1/oauth/web-addresses",
    operation_id = "listAppAddresses",
    tag = "oauth",
    params(ListParams),
    responses((status = 200, description = "The allowed addresses.", body = Page<WebAddressItem>), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_app_addresses(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .web_addresses(info, WebAddressesQuery { session_token })
        .await?;
    let list = found.map(|found| Listed {
        items: found.items.iter().map(WebAddressItem::from).collect(),
        next: None,
    });
    let fields = params.fields_readable(&ADDRESS_FIELDS, &list.hidden_properties())?;
    Ok(super::auth::answer(Page::of_authorized(
        ADDRESSES_LIST,
        &list,
        &fields,
    )?))
}

/// Allows a web address for AI apps.
#[utoipa::path(
    post,
    path = "/api/v1/oauth/web-addresses",
    operation_id = "addAppAddress",
    tag = "oauth",
    request_body = WebAddressRequest,
    responses((status = 204, description = "Allowed."), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer)
)]
async fn add_app_address(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<WebAddressRequest>, ApiError>,
) -> Result<Response, ApiError> {
    let ApiJson(body) = auth.after_who_asks(&info, &session_token, body).await?;
    auth.api()?
        .add_web_address(
            info,
            AddWebAddress {
                session_token,
                address: body.address,
            },
        )
        .await?;
    Ok(done())
}

/// Takes a web address away: apps that registered with it are refused at their next sign-in.
#[utoipa::path(
    post,
    path = "/api/v1/oauth/web-addresses/remove",
    operation_id = "removeAppAddress",
    tag = "oauth",
    request_body = WebAddressRequest,
    responses((status = 204, description = "Taken away."), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn remove_app_address(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<WebAddressRequest>, ApiError>,
) -> Result<Response, ApiError> {
    let ApiJson(body) = auth.after_who_asks(&info, &session_token, body).await?;
    auth.api()?
        .remove_web_address(
            info,
            RemoveWebAddress {
                session_token,
                address: body.address,
            },
        )
        .await?;
    Ok(done())
}

/// The routes of the browser app.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_authorization, decide_authorization))
        .routes(routes!(list_connected_apps))
        .routes(routes!(take_back_app))
        .routes(routes!(list_app_addresses, add_app_address))
        .routes(routes!(remove_app_address))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lean::assert_fields_are_the_keys_of;

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_a_connected_app() {
        let item = ConnectedAppItem::from(&ConnectedAppView {
            id: GrantId::from_random([1; 16]),
            name: "Claude Code".to_owned(),
            on_this_computer: true,
            scope: KeyChoice::SeeOnly,
            projects: Vec::new(),
            allowed_at: <_>::default(),
            last_used_at: Some(<_>::default()),
        });
        assert_fields_are_the_keys_of(&APP_FIELDS, &item);
    }
}
