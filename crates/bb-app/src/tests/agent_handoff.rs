//! What the entry of the MCP server leaves for the call of the tool of the same message (M7-F127): only
//! the calls of that message take it, once, and never a request that comes some other way.

use bb_core::ErrorCode;
use bb_core::agent::LimitKind;
use bb_core::ai::ToolScope;
use serde_json::json;

use super::agent_limits::limit;
use super::agent_tools::as_agent;
use super::agent_work::handed;
use super::agents::refused;
use super::fake::FakeStore;
use super::scenarios::token;
use super::world::{Outcome, World};
use crate::ai::tools::{ToolListQuery, ToolMode, call_tool, list_tools};
use crate::auth::{AppError, Ctx, McpIdentityQuery, RequestInfo, mcp_identity};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

/// Whether the answer is the refusal `code`.
fn refused_like<T>(answer: &Result<T, AppError>, code: ErrorCode) -> bool {
    answer
        .as_ref()
        .err()
        .is_some_and(|error| refused(error, code))
}

/// The context of a request that comes with the handle of its message.
fn with_handle<'a>(world: &'a World, handle: Option<&str>) -> Ctx<'a, FakeStore> {
    world.services.ctx(RequestInfo {
        handoff: handle.map(str::to_owned),
        ..RequestInfo::default()
    })
}

/// The entry of a message: what it leaves is taken with the handle it gives back.
async fn entry(world: &World, key: &str) -> Result<String, String> {
    let who = mcp_identity(
        &world.ctx(),
        McpIdentityQuery {
            token: key.to_owned(),
        },
    )
    .await
    .map_err(|error| format!("{error:?}"))?;
    who.handoff.ok_or_else(|| "no handle".to_owned())
}

/// A look at the changes as the agent behind `key`, in this context.
async fn changes(key: &str, ctx: &Ctx<'_, FakeStore>) -> Result<(), AppError> {
    call_tool(
        ctx,
        as_agent(key, "changes_since", json!({}), ToolMode::Run),
    )
    .await
    .map(|_| ())
}

/// Agents off for the organization: the key of an agent is turned away whenever it is looked up.
async fn agents_off(world: &World) -> Outcome {
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(world),
            scope: SettingsScope::Organization,
            key: "agents.enabled".to_owned(),
            value: bb_core::settings::SettingValue::Bool(false),
        },
    )
    .await
    .map_err(|error| format!("{error:?}"))?;
    Ok(())
}

#[tokio::test]
async fn m7_f127_only_the_calls_of_the_message_take_what_its_entry_left_and_only_once() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let handle = entry(&world, &given.key).await?;
    let other = entry(&world, &given.key).await?;
    assert_ne!(handle, other, "every message has a handle of its own");
    agents_off(&world).await?;
    // Another request with the key looks it up whatever was left: it is turned away.
    let plain = changes(&given.key, &with_handle(&world, None)).await;
    assert!(refused_like(&plain, ErrorCode::NotSignedIn));
    let listed = list_tools(
        &with_handle(&world, None),
        ToolListQuery {
            session_token: given.key.clone(),
            granted: ToolScope::SeeAndChange,
        },
    )
    .await;
    assert!(refused_like(&listed, ErrorCode::NotSignedIn));
    // A handle that is not one the entry gave takes nothing.
    let guessed = changes(&given.key, &with_handle(&world, Some("not-a-handle"))).await;
    assert!(refused_like(&guessed, ErrorCode::NotSignedIn));
    // The message takes what its entry left, once.
    changes(&given.key, &with_handle(&world, Some(&handle))).await?;
    let again = changes(&given.key, &with_handle(&world, Some(&handle))).await;
    assert!(refused_like(&again, ErrorCode::NotSignedIn));
    // What the entry left for another message is not taken by this one's calls.
    let mismatched = call_tool(
        &with_handle(&world, Some(&other)),
        as_agent("bb_x_y", "changes_since", json!({}), ToolMode::Run),
    )
    .await;
    assert!(mismatched.is_err());
    Ok(())
}

#[tokio::test]
async fn m7_f127_a_change_of_a_message_with_a_handle_counts_against_the_limit() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    limit(&world, LimitKind::ChangesAMinute, 1);
    let issue = given.issue.to_string();
    let take = json!({"issue": issue});
    let report = json!({"issue": issue, "minutes": 1, "cost_cents": 0});
    for (input, tool) in [(take, "take"), (report, "report")] {
        let handle = entry(&world, &given.key).await?;
        let made = call_tool(
            &with_handle(&world, Some(&handle)),
            as_agent(&given.key, tool, input, ToolMode::Run),
        )
        .await;
        if tool == "take" {
            made?;
        } else {
            assert!(
                refused_like(&made, ErrorCode::AgentLimitReached),
                "the second change was not counted"
            );
        }
    }
    Ok(())
}
