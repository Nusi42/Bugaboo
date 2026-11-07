//! The words of MCP over JSON-RPC 2.0 (AI-20, ADR-0042): what a message looks like, the versions of
//! the protocol this server speaks, and the errors it answers with.
//!
//! Only what the server needs is read: a request has a `method`, maybe an `id` and `params`. A
//! message without an `id` is a notification and is not answered.

use serde_json::{Map, Value, json};

/// The versions of the protocol this server speaks, newest first. An app that asks for one of them
/// gets it; any other asks for the newest, and the app decides whether it can speak that
/// (negotiation in the lifecycle of the MCP specification).
pub(crate) const PROTOCOL_VERSIONS: [&str; 4] =
    ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

/// The name the server gives itself.
pub(crate) const SERVER_NAME: &str = "Bugaboo";

/// The header that names a session in the transport of older versions of the protocol. The server
/// sends one at `initialize` and never asks for it: what counts is the token.
pub(crate) const SESSION_HEADER: &str = "mcp-session-id";

/// What went wrong in the way JSON-RPC says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fault {
    /// The text is no JSON.
    Parse,
    /// The JSON is no request.
    InvalidRequest,
    /// There is no such method.
    MethodNotFound,
    /// The arguments are not the method's.
    InvalidParams,
    /// The server could not do it.
    Internal,
}

impl Fault {
    /// The code of JSON-RPC.
    pub(crate) const fn code(self) -> i64 {
        match self {
            Self::Parse => -32700,
            Self::InvalidRequest => -32600,
            Self::MethodNotFound => -32601,
            Self::InvalidParams => -32602,
            Self::Internal => -32603,
        }
    }
}

/// A message an app sent.
pub(crate) struct Message {
    /// The `id`, when the message is a request that wants an answer.
    pub(crate) id: Option<Value>,
    /// The method.
    pub(crate) method: String,
    /// The parameters, an object or nothing.
    pub(crate) params: Map<String, Value>,
}

impl Message {
    /// Reads one message; `None` when it is not a request or a notification.
    pub(crate) fn read(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        if object.get("jsonrpc")?.as_str()? != "2.0" {
            return None;
        }
        let params = match object.get("params") {
            None | Some(Value::Null) => Map::new(),
            Some(Value::Object(params)) => params.clone(),
            Some(_) => return None,
        };
        Some(Self {
            id: object.get("id").filter(|id| !id.is_null()).cloned(),
            method: object.get("method")?.as_str()?.to_owned(),
            params,
        })
    }

    /// The text parameter `name`.
    pub(crate) fn text(&self, name: &str) -> Option<&str> {
        self.params.get(name)?.as_str()
    }
}

/// The answer to a request.
pub(crate) fn result(id: &Value, result: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// The error answer to a request.
pub(crate) fn error(id: &Value, fault: Fault, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": fault.code(), "message": message },
    })
}

/// The error answer when there is no usable `id`.
pub(crate) fn error_without_id(fault: Fault, message: &str) -> Value {
    error(&Value::Null, fault, message)
}

/// The version to speak with an app that asked for `asked`.
pub(crate) fn negotiated(asked: Option<&str>) -> &'static str {
    asked
        .and_then(|asked| PROTOCOL_VERSIONS.iter().find(|each| **each == asked))
        .copied()
        .unwrap_or(PROTOCOL_VERSIONS[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_20_a_request_and_a_notification_are_read_and_nothing_else_is() {
        let request = Message::read(&json!({"jsonrpc":"2.0","id":1,"method":"ping"})).unwrap();
        assert_eq!(request.id, Some(json!(1)));
        let note = Message::read(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        assert!(note.unwrap().id.is_none());
        for bad in [
            json!({"id":1,"method":"ping"}),
            json!({"jsonrpc":"1.0","id":1,"method":"ping"}),
            json!({"jsonrpc":"2.0","id":1}),
            json!({"jsonrpc":"2.0","id":1,"method":"ping","params":[1]}),
            json!("ping"),
        ] {
            assert!(Message::read(&bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn ai_20_the_version_an_app_asks_for_is_kept_when_it_is_known() {
        assert_eq!(negotiated(Some("2025-03-26")), "2025-03-26");
        assert_eq!(negotiated(Some("1999-01-01")), PROTOCOL_VERSIONS[0]);
        assert_eq!(negotiated(None), PROTOCOL_VERSIONS[0]);
    }
}
