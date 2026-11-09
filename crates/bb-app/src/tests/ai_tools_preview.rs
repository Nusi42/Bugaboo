//! A preview of a tool that changes something, as an AI app asks for it over MCP (M7-F225, ADR-0029,
//! decision 9).

use std::collections::BTreeMap;

use bb_core::ai::{Tool, ToolCaller, ToolEffect};
use serde_json::json;

use super::ai_tools::{call, issue_with};
use super::scenarios::{a_project, token};
use super::world::{Outcome, World};
use crate::ai::tools::{ToolMode, call_tool};

#[tokio::test]
async fn m7_f225_the_log_marks_a_preview_of_a_change_as_a_read() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    let key = issue_with(&world, "WEB", "Old title", "d", BTreeMap::new()).await?;
    let mut preview = call(
        &token(&world),
        "update_issue",
        json!({"issue": key, "version": 1, "title": "New title"}),
    );
    preview.caller = ToolCaller::AiApp;
    preview.mode = ToolMode::Preview;
    let answer = call_tool(&world.ctx(), preview).await?.into_value();
    assert!(!answer.changes.is_empty(), "the before and after");
    let rows = world.store.lock().tool_calls.clone();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].tool, Some(Tool::UpdateIssue));
    assert_eq!(rows[0].effect, Some(ToolEffect::Reads));
    Ok(())
}
