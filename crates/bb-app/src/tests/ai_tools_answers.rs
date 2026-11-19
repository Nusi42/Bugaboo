//! The line under the answer of a tool (AI-21): a change that changes nothing says so and never
//! that it saved.

use std::collections::BTreeMap;

use serde_json::json;

use super::ai_tools::{call, issue_with, read, without_nonces};
use super::scenarios::{a_project, token};
use super::world::{Outcome, World};
use crate::ai::tools::{ToolMode, call_tool};
use bb_core::ErrorCode;

#[tokio::test]
async fn ai_21_a_change_that_changes_nothing_says_nothing_changed() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    let key = issue_with(&world, "WEB", "Old title", "d", BTreeMap::new()).await?;
    let admin = token(&world);
    for (tool, input) in [
        (
            "change_status",
            json!({"issue": key, "version": 1, "status": "open"}),
        ),
        (
            "update_issue",
            json!({"issue": key, "version": 1, "title": "Old title"}),
        ),
    ] {
        // The person applies what the assistant proposed.
        let mut applied = call(&admin, tool, input);
        applied.mode = ToolMode::Apply;
        let text = without_nonces(&call_tool(&world.ctx(), applied).await?.into_value().text);
        assert_eq!(
            text.lines().last(),
            Some("Nothing changed."),
            "{tool}: {text}"
        );
        assert!(!text.contains("Saved"), "{tool}: {text}");
    }
    // The issue is still at the version it was read at.
    let after = read(&world, &admin, "get_issue", json!({"issue": key})).await?;
    assert!(after.contains("version: 1"), "{after}");
    Ok(())
}

#[tokio::test]
async fn ai_21_a_change_that_changes_nothing_on_an_old_version_says_the_version_changed() -> Outcome
{
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    let key = issue_with(&world, "WEB", "Old title", "d", BTreeMap::new()).await?;
    let admin = token(&world);
    let apply = |tool: &str, input| {
        let mut applied = call(&admin, tool, input);
        applied.mode = ToolMode::Apply;
        applied
    };
    // Somebody changes the issue: it is at version 2 now.
    call_tool(
        &world.ctx(),
        apply(
            "update_issue",
            json!({"issue": key, "version": 1, "title": "New title"}),
        ),
    )
    .await?;
    // A change asked on version 1 for what the issue holds now is refused as late, never answered
    // with "Nothing changed".
    for (tool, input) in [
        (
            "change_status",
            json!({"issue": key, "version": 1, "status": "open"}),
        ),
        (
            "update_issue",
            json!({"issue": key, "version": 1, "title": "New title"}),
        ),
    ] {
        let code = call_tool(&world.ctx(), apply(tool, input))
            .await
            .err()
            .and_then(|error| error.code());
        assert_eq!(code, Some(ErrorCode::VersionChanged), "{tool}");
        assert_eq!(
            code.map(|code| code.text().key().as_str()),
            Some("error.edit.version-changed"),
            "{tool}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn ai_21_an_empty_answer_says_nothing_was_found() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let projects = read(&world, &admin, "list_projects", json!({})).await?;
    assert_eq!(
        projects.lines().last(),
        Some("Nothing found."),
        "{projects}"
    );
    a_project(&world, "WEB").await?;
    for (tool, input) in [
        ("get_my_work", json!({})),
        (
            "search_issues",
            json!({"query": "title contains \"nowhere\""}),
        ),
    ] {
        let text = read(&world, &admin, tool, input).await?;
        assert_eq!(
            text.lines().last(),
            Some("Nothing found."),
            "{tool}: {text}"
        );
    }
    // An answer with something in it has no such line.
    let listed = read(&world, &admin, "list_projects", json!({})).await?;
    assert!(!listed.contains("Nothing found."), "{listed}");
    Ok(())
}

#[tokio::test]
async fn ai_21_a_preview_of_a_change_that_changes_nothing_says_no_change() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    let key = issue_with(&world, "WEB", "Old title", "d", BTreeMap::new()).await?;
    let mut preview = call(
        &token(&world),
        "change_status",
        json!({"issue": key, "version": 1, "status": "open"}),
    );
    preview.mode = ToolMode::Preview;
    let text = without_nonces(&call_tool(&world.ctx(), preview).await?.into_value().text);
    assert_eq!(text.lines().last(), Some("No change."), "{text}");
    Ok(())
}
