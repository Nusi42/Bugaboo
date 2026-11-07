//! `/mcp`: the MCP server on the server's own port, over streamable HTTP (AI-20, ADR-0042).
//!
//! Every request carries its token in `Authorization: Bearer …`: the access token of an app, or an
//! access key. A request with neither, or with one that does not work, is answered `401` with the
//! `WWW-Authenticate` header that names the metadata of the sign-in, so an app can find it by itself
//! (the authorization part of the MCP specification). A request is answered in its own response as
//! JSON; the server never opens a stream of its own, so `GET` is `405`.
//!
//! The server keeps no session of its own: the token says who calls, and the run of the session of an
//! app is kept per grant (`session.rs`).

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, Method, Request, StatusCode, header};
use axum::response::{IntoResponse as _, Response};
use axum::routing::any;
use bb_app::auth::{AuthApi, McpIdentity, McpIdentityQuery, OAuthMetadataQuery, RequestInfo};
use bb_core::oauth::{MCP_PATH, PROTECTED_RESOURCE_METADATA_PATH};
use serde_json::{Value, json};

use crate::methods::{self, Caller, Failure};
use crate::protocol::{self, Fault, Message};
use crate::session::Sessions;

/// The biggest message the server reads.
const BODY_LIMIT_BYTES: usize = 1024 * 1024;

/// Who asks and where from, as the HTTP layer of the server knows it.
pub struct Seen {
    /// The client network and the program.
    pub info: RequestInfo,
    /// The address the request came to (`https://bugaboo.example.com`), for a server that does not
    /// know its public address.
    pub origin: String,
}

/// What the MCP server needs from the server around it.
#[derive(Clone)]
pub struct McpOptions {
    /// The use cases.
    pub api: Arc<dyn AuthApi>,
    /// How the HTTP layer reads the client and the address out of a request, written once there.
    pub seen: Arc<dyn Fn(&Parts) -> Seen + Send + Sync>,
}

#[derive(Clone)]
struct McpState {
    options: McpOptions,
    sessions: Arc<Sessions>,
}

/// The MCP server as the routes the HTTP layer of the server mounts.
pub type McpService = Router;

/// The routes of the MCP server: `/mcp`.
pub fn router(options: McpOptions) -> McpService {
    let sessions = Sessions::new(Arc::clone(&options.api));
    Router::new()
        .route(MCP_PATH, any(handle))
        .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES))
        .with_state(McpState { options, sessions })
}

/// The token of the request: `Authorization: Bearer <token>`.
fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

/// Whether a browser page of another website sent the request: its `Origin` is not the host the
/// request came to (a defence against DNS rebinding, MCP specification). A program that is not a
/// browser sends none.
fn from_another_site(headers: &HeaderMap) -> bool {
    let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    origin
        .split_once("://")
        .is_none_or(|(_, authority)| !authority.eq_ignore_ascii_case(host))
}

fn json_response(status: StatusCode, body: &Value) -> Response {
    let mut response = (status, axum::Json(body.clone())).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// `401` with the header that tells the app where the sign-in is.
async fn challenge(state: &McpState, seen: &Seen, bad_token: bool) -> Response {
    let origin = match state
        .options
        .api
        .oauth_metadata(
            seen.info.clone(),
            OAuthMetadataQuery {
                origin_seen: seen.origin.clone(),
            },
        )
        .await
    {
        Ok(found) => found.origin,
        Err(_) => seen.origin.clone(),
    };
    let error = if bad_token {
        ", error=\"invalid_token\""
    } else {
        ""
    };
    let value =
        format!("Bearer resource_metadata=\"{origin}{PROTECTED_RESOURCE_METADATA_PATH}\"{error}");
    let mut response = json_response(
        StatusCode::UNAUTHORIZED,
        &json!({ "error": "unauthorized" }),
    );
    if let Ok(value) = HeaderValue::from_str(&value) {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, value);
    }
    response
}

/// The answer for a failure that ends the whole request.
async fn failure_response(state: &McpState, seen: &Seen, failure: Failure) -> Option<Response> {
    match failure {
        Failure::Unauthorized => Some(challenge(state, seen, true).await),
        Failure::Limited(wait) => {
            let mut response = json_response(
                StatusCode::TOO_MANY_REQUESTS,
                &json!({ "error": "too_many_requests", "retry_after": wait }),
            );
            if let Ok(value) = HeaderValue::from_str(&wait.to_string()) {
                response.headers_mut().insert(header::RETRY_AFTER, value);
            }
            Some(response)
        }
        Failure::SecondStep => Some(json_response(
            StatusCode::FORBIDDEN,
            &json!({ "error": "second_step_required" }),
        )),
        Failure::Fault(..) => None,
    }
}

async fn handle(State(state): State<McpState>, request: Request<Body>) -> Response {
    let (parts, body) = request.into_parts();
    let seen = (state.options.seen)(&parts);
    if from_another_site(&parts.headers) {
        return json_response(
            StatusCode::FORBIDDEN,
            &json!({ "error": "forbidden_origin" }),
        );
    }
    let Some(token) = bearer(&parts.headers).map(str::to_owned) else {
        return challenge(&state, &seen, false).await;
    };
    let who = match state
        .options
        .api
        .mcp_identity(
            seen.info.clone(),
            McpIdentityQuery {
                token: token.clone(),
            },
        )
        .await
    {
        Ok(who) => who,
        Err(error) => {
            return failure_response(&state, &seen, error.into())
                .await
                .unwrap_or_else(|| json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({})));
        }
    };
    match parts.method {
        Method::POST => {
            let Ok(bytes) = axum::body::to_bytes(body, BODY_LIMIT_BYTES).await else {
                return json_response(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    &protocol::error_without_id(Fault::InvalidRequest, "The message is too big."),
                );
            };
            post(&state, &seen, &token, &who, &bytes).await
        }
        Method::DELETE => {
            state.sessions.end(&seen.info, &who).await;
            StatusCode::NO_CONTENT.into_response()
        }
        _ => {
            let mut response = StatusCode::METHOD_NOT_ALLOWED.into_response();
            response
                .headers_mut()
                .insert(header::ALLOW, HeaderValue::from_static("POST, DELETE"));
            response
        }
    }
}

async fn post(
    state: &McpState,
    seen: &Seen,
    token: &str,
    who: &McpIdentity,
    bytes: &Bytes,
) -> Response {
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return json_response(
            StatusCode::BAD_REQUEST,
            &protocol::error_without_id(Fault::Parse, "The message is not JSON."),
        );
    };
    let (batch, items) = match value {
        Value::Array(items) => (true, items),
        single => (false, vec![single]),
    };
    let mut answers = Vec::new();
    let mut started = false;
    for item in &items {
        match reply_to(state, seen, token, who, item).await {
            Ok(Some((answered, began))) => {
                started |= began;
                answers.push(answered);
            }
            Ok(None) => {}
            Err(response) => return response,
        }
    }
    if answers.is_empty() {
        return StatusCode::ACCEPTED.into_response();
    }
    let body = if batch {
        Value::Array(answers)
    } else {
        answers.remove(0)
    };
    let mut response = json_response(StatusCode::OK, &body);
    if started && let Ok(id) = HeaderValue::from_str(&session_id(who)) {
        response.headers_mut().insert(protocol::SESSION_HEADER, id);
    }
    response
}

/// The session id sent at `initialize` for clients of the older transport: it names the grant (or
/// key), which is not a secret, and is never asked for again.
fn session_id(who: &McpIdentity) -> String {
    who.credential.clone()
}

/// The answer to one message, `None` for a notification, and whether it began the session. An `Err`
/// is the answer of the whole request (the token stopped working).
async fn reply_to(
    state: &McpState,
    seen: &Seen,
    token: &str,
    who: &McpIdentity,
    item: &Value,
) -> Result<Option<(Value, bool)>, Response> {
    let Some(message) = Message::read(item) else {
        let id = item.get("id").cloned().unwrap_or(Value::Null);
        return Ok(Some((
            protocol::error(&id, Fault::InvalidRequest, "That is not a request."),
            false,
        )));
    };
    let Some(id) = message.id.clone() else {
        // A notification: nothing to answer.
        return Ok(None);
    };
    let begins = message.method == "initialize";
    let run = if matches!(message.method.as_str(), "tools/call" | "resources/read") || begins {
        match state.sessions.run_of(&seen.info, token, who, begins).await {
            Ok(run) => run,
            Err(error) => {
                return match failure_response(state, seen, error.into()).await {
                    Some(response) => Err(response),
                    None => Ok(Some((
                        protocol::error(&id, Fault::Internal, "The session could not begin."),
                        false,
                    ))),
                };
            }
        }
    } else {
        None
    };
    let caller = Caller {
        api: &state.options.api,
        info: &seen.info,
        token,
        who,
        run,
    };
    match dispatch(&caller, &message).await {
        Ok(result) => Ok(Some((protocol::result(&id, &result), begins))),
        Err(Failure::Fault(fault, text)) => Ok(Some((protocol::error(&id, fault, &text), false))),
        Err(failure) => Err(failure_response(state, seen, failure)
            .await
            .unwrap_or_else(|| json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({})))),
    }
}

/// The result of the method of a message.
async fn dispatch(caller: &Caller<'_>, message: &Message) -> Result<Value, Failure> {
    match message.method.as_str() {
        "initialize" => Ok(methods::initialize(message.text("protocolVersion"))),
        "ping" => Ok(json!({})),
        "tools/list" => caller.list_tools().await,
        "tools/call" => caller.call_tool(message).await,
        "resources/list" => Ok(Caller::list_resources()),
        "resources/templates/list" => Ok(Caller::list_resource_templates()),
        "resources/read" => caller.read_resource(message).await,
        "prompts/list" => Ok(methods::list_prompts()),
        "prompts/get" => methods::get_prompt(message),
        _ => Err(Failure::Fault(
            Fault::MethodNotFound,
            "This server does not know that method.".to_owned(),
        )),
    }
}
