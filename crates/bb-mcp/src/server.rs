//! `/mcp`: the MCP server on the server's own port, over streamable HTTP (AI-20, ADR-0042).
//!
//! Every request carries its token in `Authorization: Bearer …`: the access token of an app, or an
//! access key. A request with neither, or with one that does not work, is answered `401` with the
//! `WWW-Authenticate` header that names the metadata of the sign-in, so an app can find it by itself
//! (the authorization part of the MCP specification). A request is answered in its own response as
//! JSON; the server never opens a stream of its own, so `GET` is `405`. A request holds one message:
//! a batch is refused, as the newer versions of the protocol dropped it.
//!
//! The server keeps no session of its own: the token says who calls, and the run of the session of an
//! app is kept per grant (`session.rs`). The reading of a request (who asks, from where, with which
//! token, from which origin) is the one of the API (`bb-api`), written once there.

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, State};
use axum::http::request::Parts;
use axum::http::{HeaderValue, Method, Request, StatusCode, header};
use axum::response::{IntoResponse as _, Response};
use axum::routing::any;
use bb_api::{bearer_token, is_same_origin_of, json_no_store, request_info, with_retry_after};
use bb_app::auth::{AuthApi, McpIdentity, McpIdentityQuery, OAuthMetadataQuery, RequestInfo};
use bb_core::ErrorCode;
use bb_core::message;
use bb_core::oauth::{MCP_PATH, PROTECTED_RESOURCE_METADATA_PATH};
use serde_json::{Value, json};

use crate::methods::{self, Caller, Failure};
use crate::protocol::{self, Fault, Message, sentence};
use crate::session::Sessions;

/// The biggest message the server reads.
const BODY_LIMIT_BYTES: usize = 1024 * 1024;

/// What the MCP server needs from the server around it.
#[derive(Clone)]
pub struct McpOptions {
    /// The use cases.
    pub api: Arc<dyn AuthApi>,
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

/// Whether a browser page of another website sent the request (a defence against DNS rebinding,
/// MCP specification). A program that is not a browser sends no `Origin`. A page is of this
/// server when its `Origin` is the one the request came to (the API's rule) and, where the server
/// knows its public address, is that address: a name an attacker points at this machine is neither.
fn from_another_site(parts: &Parts, own_origin: &str) -> bool {
    let Some(origin) = parts
        .headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    !is_same_origin_of(parts) || !origin.eq_ignore_ascii_case(own_origin)
}

/// `401` with the header that tells the app where the sign-in is.
fn challenge(origin: &str, bad_token: bool) -> Response {
    let error = if bad_token {
        ", error=\"invalid_token\""
    } else {
        ""
    };
    let value =
        format!("Bearer resource_metadata=\"{origin}{PROTECTED_RESOURCE_METADATA_PATH}\"{error}");
    let mut response = json_no_store(
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
fn failure_response(origin: &str, failure: Failure) -> Response {
    match failure {
        Failure::Unauthorized => challenge(origin, true),
        Failure::Limited(wait) => {
            let mut response = json_no_store(
                StatusCode::TOO_MANY_REQUESTS,
                &json!({ "error": "too_many_requests", "retry_after": wait }),
            );
            with_retry_after(&mut response, wait);
            response
        }
        Failure::SecondStep => json_no_store(
            StatusCode::FORBIDDEN,
            &json!({ "error": "second_step_required" }),
        ),
        Failure::Fault(fault, text) => json_no_store(
            StatusCode::INTERNAL_SERVER_ERROR,
            &protocol::error_without_id(fault, &text),
        ),
    }
}

/// What the HTTP request is about, found out once.
struct Asked<'a> {
    state: &'a McpState,
    info: RequestInfo,
    /// The address of this server as told to apps.
    origin: String,
}

async fn handle(State(state): State<McpState>, request: Request<Body>) -> Response {
    let (parts, body) = request.into_parts();
    let info = request_info(&parts);
    let origin = state
        .options
        .api
        .oauth_metadata(info.clone(), OAuthMetadataQuery)
        .await
        .map_or_else(|_| info.origin_seen.clone(), |found| found.origin);
    if from_another_site(&parts, &origin) {
        return json_no_store(
            StatusCode::FORBIDDEN,
            &json!({ "error": "forbidden_origin" }),
        );
    }
    let Some(token) = bearer_token(&parts.headers).map(str::to_owned) else {
        return challenge(&origin, false);
    };
    let asked = Asked {
        state: &state,
        info,
        origin,
    };
    let bytes = if parts.method == Method::POST {
        let Ok(bytes) = axum::body::to_bytes(body, BODY_LIMIT_BYTES).await else {
            return json_no_store(
                StatusCode::PAYLOAD_TOO_LARGE,
                &protocol::error_without_id(
                    Fault::InvalidRequest,
                    &sentence(message::MCP_ERROR_TOO_BIG),
                ),
            );
        };
        bytes
    } else {
        Bytes::new()
    };
    let who = match state
        .options
        .api
        .mcp_identity(
            asked.info.clone(),
            McpIdentityQuery {
                token: token.clone(),
            },
        )
        .await
    {
        Ok(who) => who,
        Err(error) => return asked.turned_away(error.into(), &bytes),
    };
    match parts.method {
        Method::POST => asked.post(&token, &who, &bytes).await,
        Method::DELETE => {
            state.sessions.end(&asked.info, &who).await;
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

impl Asked<'_> {
    /// The answer when the credential was turned away. A tool call over the limit is answered as a
    /// result the AI can read, with the time to wait; everything else as the HTTP answer.
    fn turned_away(&self, failure: Failure, bytes: &Bytes) -> Response {
        if let Failure::Limited(wait) = failure
            && let Ok(value) = serde_json::from_slice::<Value>(bytes)
            && let Some(message) = Message::read(&value)
            && message.method == "tools/call"
            && let Some(id) = message.id
        {
            let text = format!(
                "{} {}",
                sentence(ErrorCode::TooManyRequests.message()),
                message::mcp_wait(wait).render()
            );
            let mut response = json_no_store(
                StatusCode::OK,
                &protocol::result(
                    &id,
                    &json!({ "content": [{ "type": "text", "text": text }], "isError": true }),
                ),
            );
            with_retry_after(&mut response, wait);
            return response;
        }
        failure_response(&self.origin, failure)
    }

    async fn post(&self, token: &str, who: &McpIdentity, bytes: &Bytes) -> Response {
        let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
            return json_no_store(
                StatusCode::BAD_REQUEST,
                &protocol::error_without_id(Fault::Parse, &sentence(message::MCP_ERROR_NOT_JSON)),
            );
        };
        // A batch, an empty one too, is no request of this server (the newer protocol has none).
        if value.is_array() {
            return json_no_store(
                StatusCode::BAD_REQUEST,
                &protocol::error_without_id(
                    Fault::InvalidRequest,
                    &sentence(message::MCP_ERROR_BATCH),
                ),
            );
        }
        match self.reply_to(token, who, &value).await {
            Ok(Some((answer, began))) => {
                let mut response = json_no_store(StatusCode::OK, &answer);
                if began && let Ok(id) = HeaderValue::from_str(&who.credential) {
                    response.headers_mut().insert(protocol::SESSION_HEADER, id);
                }
                response
            }
            Ok(None) => StatusCode::ACCEPTED.into_response(),
            Err(response) => response,
        }
    }

    /// The answer to the message, `None` for a notification, and whether it was `initialize`. An
    /// `Err` is the answer of the whole request (the token stopped working).
    async fn reply_to(
        &self,
        token: &str,
        who: &McpIdentity,
        item: &Value,
    ) -> Result<Option<(Value, bool)>, Response> {
        let Some(message) = Message::read(item) else {
            let id = item.get("id").cloned().unwrap_or(Value::Null);
            return Ok(Some((
                protocol::error(
                    &id,
                    Fault::InvalidRequest,
                    &sentence(message::MCP_ERROR_NOT_A_REQUEST),
                ),
                false,
            )));
        };
        let Some(id) = message.id.clone() else {
            // A notification: nothing to answer.
            return Ok(None);
        };
        let begins = message.method == "initialize";
        let run = if matches!(message.method.as_str(), "tools/call" | "resources/read") || begins {
            self.state
                .sessions
                .run_of(&self.info, token, who)
                .await
                .map_err(|error| failure_response(&self.origin, error.into()))?
        } else {
            None
        };
        let caller = Caller {
            api: &self.state.options.api,
            info: &self.info,
            token,
            who,
            run,
        };
        match dispatch(&caller, &message).await {
            Ok(result) => Ok(Some((protocol::result(&id, &result), begins))),
            Err(Failure::Fault(fault, text)) => {
                Ok(Some((protocol::error(&id, fault, &text), false)))
            }
            Err(failure) => Err(failure_response(&self.origin, failure)),
        }
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
        _ => Err(Failure::fault(
            Fault::MethodNotFound,
            message::MCP_ERROR_METHOD,
        )),
    }
}
