//! The Messages API of Anthropic: how a request is written and how an answer is read.
//!
//! Both functions are pure, so they are tested against recorded examples of the shape
//! (`tests/examples/`). What the provider sent back is read for the text, the tool calls and the
//! token counts, and nothing else of it is kept.

use bb_app::ai::{AiAnswer, AiPart, AiRequest, AiRole, AiToolCall};
use serde_json::{Value, json};

/// The address of the Messages API: Anthropic has one, and it is not a setting.
pub(crate) const ADDRESS: &str = "https://api.anthropic.com/v1/messages";

/// The version of the API the shape below is written for.
pub(crate) const API_VERSION: &str = "2023-06-01";

fn part_of(part: &AiPart) -> Value {
    match part {
        AiPart::Text(text) => json!({ "type": "text", "text": text }),
        AiPart::ToolCall(call) => json!({
            "type": "tool_use",
            "id": call.id,
            "name": call.name,
            "input": call.arguments,
        }),
        AiPart::ToolResult { call, text } => json!({
            "type": "tool_result",
            "tool_use_id": call,
            "content": text,
        }),
    }
}

/// The body of a request.
pub(crate) fn request_body(model: &str, request: &AiRequest) -> Value {
    let messages: Vec<Value> = request
        .turns
        .iter()
        .map(|turn| {
            json!({
                "role": match turn.role {
                    AiRole::User => "user",
                    AiRole::Assistant => "assistant",
                },
                "content": turn.parts.iter().map(part_of).collect::<Vec<_>>(),
            })
        })
        .collect();
    let mut body = json!({
        "model": model,
        "max_tokens": request.max_tokens,
        "messages": messages,
    });
    if !request.system.is_empty() {
        body["system"] = json!(request.system);
    }
    if !request.tools.is_empty() {
        body["tools"] = request
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.schema,
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
    let mut answer = AiAnswer {
        tokens_in: count(&body["usage"]["input_tokens"]),
        tokens_out: count(&body["usage"]["output_tokens"]),
        ..AiAnswer::default()
    };
    for block in body["content"].as_array()? {
        match block["type"].as_str()? {
            "text" => answer.text.push_str(block["text"].as_str()?),
            "tool_use" => answer.tool_calls.push(AiToolCall {
                id: block["id"].as_str()?.to_owned(),
                name: block["name"].as_str()?.to_owned(),
                arguments: block["input"].clone(),
            }),
            _ => {}
        }
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
                        id: "toolu_1".to_owned(),
                        name: "get_issue".to_owned(),
                        arguments: json!({ "id": "WEB-1" }),
                    })],
                },
                AiTurn {
                    role: AiRole::User,
                    parts: vec![AiPart::ToolResult {
                        call: "toolu_1".to_owned(),
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
    fn ai_01_the_anthropic_adapter_writes_and_reads_the_messages_shape() {
        let written = request_body("claude-test", &request());
        let recorded: Value =
            serde_json::from_str(include_str!("../tests/examples/anthropic-request.json")).unwrap();
        assert_eq!(written, recorded);

        let answer = read_answer(include_str!("../tests/examples/anthropic-answer.json")).unwrap();
        assert_eq!(answer.text, "I will look it up.");
        assert_eq!(answer.tokens_in, 120);
        assert_eq!(answer.tokens_out, 31);
        assert_eq!(
            answer.tool_calls,
            vec![AiToolCall {
                id: "toolu_2".to_owned(),
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
            r#"{"content":[{"type":"text"}]}"#,
            r#"{"type":"error","error":{"message":"the request said: secret"}}"#,
        ] {
            assert_eq!(read_answer(text), None, "{text}");
        }
    }
}
