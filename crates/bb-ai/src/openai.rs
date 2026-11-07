//! The chat completions shape of `OpenAI`, which most local model servers also speak: how a request
//! is written and how an answer is read.
//!
//! Both functions are pure, so they are tested against recorded examples of the shape
//! (`tests/examples/`). What the provider sent back is read for the text, the tool calls and the
//! token counts, and nothing else of it is kept.

use bb_app::ai::{AiAnswer, AiPart, AiRequest, AiRole, AiToolCall};
use serde_json::{Value, json};

/// The path below the address the administrator entered.
pub(crate) const PATH: &str = "/chat/completions";

fn messages_of(request: &AiRequest) -> Vec<Value> {
    let mut messages = Vec::new();
    if !request.system.is_empty() {
        messages.push(json!({ "role": "system", "content": request.system }));
    }
    for turn in &request.turns {
        // What a tool answered is a message of its own in this shape, before the text of the turn.
        for part in &turn.parts {
            if let AiPart::ToolResult { call, text } = part {
                messages.push(json!({ "role": "tool", "tool_call_id": call, "content": text }));
            }
        }
        let text: String = turn
            .parts
            .iter()
            .filter_map(|part| match part {
                AiPart::Text(text) => Some(text.as_str()),
                AiPart::ToolCall(_) | AiPart::ToolResult { .. } => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        let calls: Vec<Value> = turn
            .parts
            .iter()
            .filter_map(|part| match part {
                AiPart::ToolCall(call) => Some(json!({
                    "id": call.id,
                    "type": "function",
                    "function": { "name": call.name, "arguments": call.arguments.to_string() },
                })),
                AiPart::Text(_) | AiPart::ToolResult { .. } => None,
            })
            .collect();
        match turn.role {
            AiRole::User if !text.is_empty() => {
                messages.push(json!({ "role": "user", "content": text }));
            }
            AiRole::User => {}
            AiRole::Assistant => {
                let mut message = json!({ "role": "assistant" });
                message["content"] = if text.is_empty() {
                    Value::Null
                } else {
                    json!(text)
                };
                if !calls.is_empty() {
                    message["tool_calls"] = json!(calls);
                }
                messages.push(message);
            }
        }
    }
    messages
}

/// The body of a request.
pub(crate) fn request_body(model: &str, request: &AiRequest) -> Value {
    let mut body = json!({
        "model": model,
        "max_tokens": request.max_tokens,
        "messages": messages_of(request),
    });
    if !request.tools.is_empty() {
        body["tools"] = request
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.schema,
                    },
                })
            })
            .collect();
    }
    body
}

fn count(value: &Value) -> u32 {
    value
        .as_u64()
        .and_then(|number| u32::try_from(number).ok())
        .unwrap_or_default()
}

/// The answer in the body the provider sent, or none when the body is not an answer.
pub(crate) fn read_answer(text: &str) -> Option<AiAnswer> {
    let body: Value = serde_json::from_str(text).ok()?;
    let message = &body["choices"][0]["message"];
    if !message.is_object() {
        return None;
    }
    let mut answer = AiAnswer {
        text: message["content"].as_str().unwrap_or_default().to_owned(),
        tokens_in: count(&body["usage"]["prompt_tokens"]),
        tokens_out: count(&body["usage"]["completion_tokens"]),
        ..AiAnswer::default()
    };
    for call in message["tool_calls"].as_array().into_iter().flatten() {
        answer.tool_calls.push(AiToolCall {
            id: call["id"].as_str()?.to_owned(),
            name: call["function"]["name"].as_str()?.to_owned(),
            arguments: serde_json::from_str(call["function"]["arguments"].as_str()?).ok()?,
        });
    }
    Some(answer)
}

#[cfg(test)]
mod tests {
    use bb_app::ai::{AiToolSpec, AiTurn};

    use super::*;

    fn request() -> AiRequest {
        AiRequest {
            system: "You help with issues.".to_owned(),
            turns: vec![
                AiTurn {
                    role: AiRole::User,
                    parts: vec![AiPart::Text("Find WEB-1".to_owned())],
                },
                AiTurn {
                    role: AiRole::Assistant,
                    parts: vec![AiPart::ToolCall(AiToolCall {
                        id: "call_1".to_owned(),
                        name: "get_issue".to_owned(),
                        arguments: json!({ "id": "WEB-1" }),
                    })],
                },
                AiTurn {
                    role: AiRole::User,
                    parts: vec![AiPart::ToolResult {
                        call: "call_1".to_owned(),
                        text: "WEB-1: Fix login".to_owned(),
                    }],
                },
            ],
            tools: vec![AiToolSpec {
                name: "get_issue".to_owned(),
                description: "Reads one issue.".to_owned(),
                schema: json!({ "type": "object", "properties": { "id": { "type": "string" } } }),
            }],
            max_tokens: 512,
        }
    }

    #[test]
    fn ai_01_the_openai_compatible_adapter_writes_and_reads_the_chat_shape() {
        let written = request_body("local-test", &request());
        let recorded: Value =
            serde_json::from_str(include_str!("../tests/examples/openai-request.json")).unwrap();
        assert_eq!(written, recorded);

        let answer = read_answer(include_str!("../tests/examples/openai-answer.json")).unwrap();
        assert_eq!(answer.text, "I will look it up.");
        assert_eq!(answer.tokens_in, 98);
        assert_eq!(answer.tokens_out, 27);
        assert_eq!(
            answer.tool_calls,
            vec![AiToolCall {
                id: "call_2".to_owned(),
                name: "get_issue".to_owned(),
                arguments: json!({ "id": "WEB-2" }),
            }]
        );
    }

    #[test]
    fn ai_01_a_body_that_is_no_answer_is_no_answer() {
        for text in [
            "",
            "not json",
            "{}",
            r#"{"choices":[]}"#,
            r#"{"choices":[{"message":{"tool_calls":[{"id":"1","function":{"name":"x","arguments":"{"}}]}}]}"#,
            r#"{"error":{"message":"the request said: secret"}}"#,
        ] {
            assert_eq!(read_answer(text), None, "{text}");
        }
    }
}
