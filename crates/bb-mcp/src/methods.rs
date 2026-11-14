//! What the MCP server answers (AI-20, AI-21, AI-23, ADR-0042): the tools of the one registry the
//! token's scope allows, the resources and the prompts.
//!
//! Every answer comes from `bb-app` through [`AuthApi`]: a tool is listed and called only through
//! the registry, as the person behind the token and narrowed by what they allowed, so a field
//! hidden from the person is not in an answer. A resource is a tool call under another name (the
//! address of an issue is the tool that reads it), so it takes the same road and has the same
//! limits. Nothing here decides what the app may do.

use std::sync::Arc;

use bb_app::ai::context::PeopleMode;
use bb_app::ai::tools::{ToolCallQuery, ToolListQuery, ToolMode};
use bb_app::auth::{AppError, AuthApi, McpIdentity, RequestInfo};
use bb_core::ErrorCode;
use bb_core::ai::{Tool, ToolCaller, ToolScope};
use bb_core::id::RunId;
use bb_core::message::{self, Message, MessageKey};
use serde_json::{Value, json};

use crate::protocol::{Fault, Message as Request, sentence};

/// The address every resource of this server starts with.
const SCHEME: &str = "bugaboo://";
/// The text kind of every answer.
const TEXT: &str = "text/plain";
/// The argument every tool that changes something takes over MCP (ADR-0029, decision 9): `true` says
/// what the call would change and saves nothing.
const PREVIEW: &str = "preview";

/// The schema of a tool as an app is shown it: a tool that changes something also takes `preview`.
/// This is the one place that adds it, from the effect the registry names.
fn offered(tool: &Value) -> Value {
    let mut schema = tool["input_schema"].clone();
    if tool["effect"] == "changes"
        && let Some(object) = schema.as_object_mut()
    {
        let properties = object.entry("properties").or_insert_with(|| json!({}));
        if let Some(properties) = properties.as_object_mut() {
            properties.insert(
                PREVIEW.to_owned(),
                json!({
                    "type": "boolean",
                    "description": sentence(message::MCP_TOOL_PREVIEW_ARGUMENT),
                }),
            );
        }
    }
    schema
}

/// Why a message could not be answered with a result.
pub(crate) enum Failure {
    /// The token does not work (any more): the HTTP answer is `401`.
    Unauthorized,
    /// The app made too many calls: the HTTP answer is `429` with the time to wait.
    Limited(u64),
    /// The error of JSON-RPC.
    Fault(Fault, String),
    /// The person has to set up a second step first: the HTTP answer is `403`.
    SecondStep,
}

impl Failure {
    /// A fault with its sentence from the catalog.
    pub(crate) fn fault(fault: Fault, message: MessageKey) -> Self {
        Self::Fault(fault, sentence(message))
    }
}

impl From<AppError> for Failure {
    fn from(error: AppError) -> Self {
        match error {
            AppError::Refused(ErrorCode::NotSignedIn) | AppError::SessionEnded => {
                Self::Unauthorized
            }
            AppError::Refused(ErrorCode::SecondStepRequired) => Self::SecondStep,
            AppError::Limited { retry_after } => Self::Limited(retry_after),
            other => {
                tracing::error!(cause = %other, "an MCP call failed inside the server");
                Self::fault(Fault::Internal, message::MCP_ERROR_INTERNAL)
            }
        }
    }
}

/// The scope the registry is asked for. The token narrows what its person may do (a *See only*
/// grant holds no permission to change, a project limit takes projects away), and the registry
/// offers and runs only what the actor, so narrowed, may use (`effective_scope`), so the widest
/// scope here lets the grant decide and never lets a tool through that the grant does not allow.
const NARROWED_BY_THE_TOKEN: ToolScope = ToolScope::SeeAndChange;

/// Who calls, and what the call belongs to.
pub(crate) struct Caller<'a> {
    pub(crate) api: &'a Arc<dyn AuthApi>,
    pub(crate) info: &'a RequestInfo,
    pub(crate) token: &'a str,
    pub(crate) who: &'a McpIdentity,
    pub(crate) run: Option<RunId>,
}

/// What `initialize` answers: the version, what the server offers and who it is.
pub(crate) fn initialize(asked: Option<&str>) -> Value {
    json!({
        "protocolVersion": crate::protocol::negotiated(asked),
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": { "subscribe": false, "listChanged": false },
            "prompts": { "listChanged": false },
        },
        "serverInfo": {
            "name": crate::protocol::SERVER_NAME,
            "version": env!("CARGO_PKG_VERSION"),
        },
        "instructions": sentence(message::MCP_INSTRUCTIONS),
    })
}

impl Caller<'_> {
    /// Calls the tool as the person and gives the text the AI reads, or the sentence of the refusal.
    async fn call(
        &self,
        tool: &str,
        input: Value,
        mode: ToolMode,
    ) -> Result<Result<String, String>, Failure> {
        // What the call changes carries the run of the session (RequestInfo::run).
        let mut info = self.info.clone();
        info.run = self.run;
        info.handoff.clone_from(&self.who.handoff);
        let answer = self
            .api
            .call_tool(
                info,
                ToolCallQuery {
                    session_token: self.token.to_owned(),
                    caller: if self.who.agent.is_some() {
                        ToolCaller::Agent
                    } else {
                        ToolCaller::AiApp
                    },
                    app: self.who.app.clone(),
                    granted: NARROWED_BY_THE_TOKEN,
                    people: PeopleMode::AsSeen,
                    tool: tool.to_owned(),
                    input,
                    mode,
                },
            )
            .await;
        match answer {
            Ok(answer) => {
                let value = answer.masked_value().map_err(|_| {
                    Failure::fault(Fault::Internal, message::MCP_ERROR_ANSWER_UNREADABLE)
                })?;
                let text = value
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                Ok(Ok(text))
            }
            // A call the person's rights or the scope turn away is an answer the AI can read and
            // tell the person about, not a failure of the protocol.
            Err(AppError::Refused(code))
                if !matches!(code, ErrorCode::NotSignedIn | ErrorCode::SecondStepRequired) =>
            {
                Ok(Err(code.text().render()))
            }
            Err(AppError::RefusedAtField { code, .. }) => Ok(Err(code.text().render())),
            // A change made from an old version: the agent is told it is old, which version the
            // record has now and which fields changed, in words of the program; the values are
            // not told here, so no text of people is in it.
            Err(AppError::Conflict(conflict)) => Ok(Err(format!(
                "{} {}",
                Message::from(ErrorCode::VersionChanged.message()).render(),
                message::mcp_changed(
                    conflict.fields.join(", "),
                    conflict.version.unwrap_or_default().to_string()
                )
                .render()
            ))),
            Err(error @ (AppError::TakenOver { .. } | AppError::DayLeftOpen { .. })) => {
                let code = error.code().unwrap_or(ErrorCode::Forbidden);
                Ok(Err(code.text().render()))
            }
            Err(AppError::Mistakes(_)) => Ok(Err(Message::from(
                ErrorCode::SearchInvalid.message(),
            )
            .render())),
            Err(other) => Err(other.into()),
        }
    }

    /// `tools/list`: the tools the scope of this token allows, with their schemas.
    pub(crate) async fn list_tools(&self) -> Result<Value, Failure> {
        let mut info = self.info.clone();
        info.handoff.clone_from(&self.who.handoff);
        let listed = self
            .api
            .list_tools(
                info,
                ToolListQuery {
                    session_token: self.token.to_owned(),
                    granted: NARROWED_BY_THE_TOKEN,
                },
            )
            .await?;
        let value = listed
            .masked_value()
            .map_err(|_| Failure::fault(Fault::Internal, message::MCP_ERROR_TOOLS_UNREADABLE))?;
        let tools: Vec<Value> = value
            .get("tools")
            .and_then(Value::as_array)
            .map(|tools| {
                tools
                    .iter()
                    .map(|tool| {
                        json!({
                            "name": tool["name"],
                            "description": tool["description"],
                            "inputSchema": offered(tool),
                            "annotations": {
                                "readOnlyHint": tool["effect"] == "reads",
                            },
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(json!({ "tools": tools }))
    }

    /// Whether the tool is one that changes something, as `tools/list` shows it to this token.
    async fn changes_something(&self, name: &str) -> Result<bool, Failure> {
        let listed = self.list_tools().await?;
        Ok(listed["tools"].as_array().is_some_and(|tools| {
            tools
                .iter()
                .any(|tool| tool["name"] == name && tool["annotations"]["readOnlyHint"] == false)
        }))
    }

    /// `tools/call`.
    pub(crate) async fn call_tool(&self, request: &Request) -> Result<Value, Failure> {
        let name = request
            .text("name")
            .ok_or_else(|| Failure::fault(Fault::InvalidParams, message::MCP_ERROR_TOOL_NAME))?;
        let mut arguments = match request.params.get("arguments") {
            None | Some(Value::Null) => json!({}),
            Some(arguments @ Value::Object(_)) => arguments.clone(),
            Some(_) => {
                return Err(Failure::fault(
                    Fault::InvalidParams,
                    message::MCP_ERROR_ARGUMENTS,
                ));
            }
        };
        // `preview` is an argument of the tools that change something only (it says how to call
        // them, not what to do); a tool that reads keeps it as an input it does not have, and
        // refuses it.
        let mut mode = ToolMode::Run;
        if arguments.get(PREVIEW).is_some() && self.changes_something(name).await? {
            mode = match arguments
                .as_object_mut()
                .and_then(|map| map.remove(PREVIEW))
            {
                Some(Value::Bool(true)) => ToolMode::Preview,
                Some(Value::Bool(false)) => ToolMode::Run,
                _ => {
                    return Err(Failure::fault(
                        Fault::InvalidParams,
                        message::MCP_ERROR_PREVIEW,
                    ));
                }
            };
        }
        Ok(match self.call(name, arguments, mode).await? {
            Ok(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
            Err(sentence) => {
                json!({ "content": [{ "type": "text", "text": sentence }], "isError": true })
            }
        })
    }

    /// `resources/list`: the resources with a fixed address.
    pub(crate) fn list_resources() -> Value {
        json!({ "resources": [
            {
                "uri": format!("{SCHEME}projects"),
                "name": "projects",
                "title": sentence(message::MCP_RESOURCE_PROJECTS),
                "description": sentence(message::MCP_RESOURCE_PROJECTS_HELP),
                "mimeType": TEXT,
            },
            {
                "uri": format!("{SCHEME}my-work"),
                "name": "my-work",
                "title": sentence(message::MCP_RESOURCE_MY_WORK),
                "description": sentence(message::MCP_RESOURCE_MY_WORK_HELP),
                "mimeType": TEXT,
            },
        ] })
    }

    /// `resources/templates/list`: an issue by its ID.
    pub(crate) fn list_resource_templates() -> Value {
        json!({ "resourceTemplates": [{
            "uriTemplate": format!("{SCHEME}issue/{{key}}"),
            "name": "issue",
            "title": sentence(message::MCP_RESOURCE_ISSUE),
            "description": sentence(message::MCP_RESOURCE_ISSUE_HELP),
            "mimeType": TEXT,
        }] })
    }

    /// `resources/read`: the tool that reads what the address names.
    pub(crate) async fn read_resource(&self, request: &Request) -> Result<Value, Failure> {
        let uri = request.text("uri").ok_or_else(|| {
            Failure::fault(Fault::InvalidParams, message::MCP_ERROR_RESOURCE_ADDRESS)
        })?;
        let rest = uri.strip_prefix(SCHEME).ok_or_else(|| {
            Failure::fault(Fault::InvalidParams, message::MCP_ERROR_NOT_AN_ADDRESS)
        })?;
        let (tool, input) = match rest.split_once('/') {
            Some(("issue", key)) if !key.is_empty() => (Tool::GetIssue, json!({ "issue": key })),
            None if rest == "projects" => (Tool::ListProjects, json!({})),
            None if rest == "my-work" => (Tool::GetMyWork, json!({})),
            _ => {
                return Err(Failure::fault(
                    Fault::InvalidParams,
                    message::MCP_ERROR_NOT_AN_ADDRESS,
                ));
            }
        };
        let text = match self.call(tool.code(), input, ToolMode::Run).await? {
            Ok(text) | Err(text) => text,
        };
        Ok(json!({ "contents": [{ "uri": uri, "mimeType": TEXT, "text": text }] }))
    }
}

/// `prompts/list`.
pub(crate) fn list_prompts() -> Value {
    json!({ "prompts": [
        {
            "name": "summarize_issue",
            "title": sentence(message::MCP_PROMPT_SUMMARIZE_ISSUE),
            "description": sentence(message::MCP_PROMPT_SUMMARIZE_ISSUE),
            "arguments": [{
                "name": "issue",
                "description": sentence(message::MCP_PROMPT_ISSUE_ARGUMENT),
                "required": true,
            }],
        },
        {
            "name": "my_work",
            "title": sentence(message::MCP_PROMPT_MY_WORK),
            "description": sentence(message::MCP_PROMPT_MY_WORK),
            "arguments": [],
        },
    ] })
}

/// `prompts/get`: a message for the person to send, in words.
pub(crate) fn get_prompt(request: &Request) -> Result<Value, Failure> {
    let text = match request.text("name") {
        Some("summarize_issue") => {
            let issue = request
                .params
                .get("arguments")
                .and_then(|arguments| arguments.get("issue"))
                .and_then(Value::as_str)
                .filter(|issue| !issue.is_empty())
                .ok_or_else(|| {
                    Failure::fault(Fault::InvalidParams, message::MCP_ERROR_PROMPT_ISSUE)
                })?;
            format!(
                "{} {issue}",
                sentence(message::MCP_PROMPT_SUMMARIZE_ISSUE_TEXT)
            )
        }
        Some("my_work") => sentence(message::MCP_PROMPT_MY_WORK_TEXT),
        _ => {
            return Err(Failure::fault(
                Fault::InvalidParams,
                message::MCP_ERROR_NO_PROMPT,
            ));
        }
    };
    Ok(json!({ "messages": [{ "role": "user", "content": { "type": "text", "text": text } }] }))
}
