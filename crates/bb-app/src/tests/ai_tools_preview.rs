//! A preview of a tool that changes something, as an AI app asks for it over MCP (M7-F225, ADR-0029,
//! decision 9).

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::agent_work::HandOverPower;
use bb_core::ai::{Tool, ToolCaller, ToolEffect};
use serde_json::json;

use super::agent_tools::{previewed, used};
use super::agent_work::{handed, handed_with, start_attempt};
use super::ai_tools::{call, issue_with};
use super::scenarios::{a_project, token};
use super::scenarios_issues::{a_type, an_issue};
use super::world::{Outcome, World};
use crate::ai::tools::{ToolMode, call_tool};
use crate::auth::AppError;

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

/// What both calls of a tool say, a preview and the real call, as text, so that two refusals can be
/// compared.
fn said<T>(answer: &Result<T, AppError>) -> String {
    match answer {
        Ok(_) => "allowed".to_owned(),
        Err(error) => format!("{error:?}"),
    }
}

#[tokio::test]
async fn m7_f225_an_agent_previews_a_sub_issue_under_the_checks_of_the_real_call() -> Outcome {
    let world = World::new().await?;
    let given = handed_with(&world, |asked| {
        asked.powers = vec![HandOverPower::CreateSubIssues];
    })
    .await?;
    let kind = a_type(&world).await?.to_string();
    let not_handed = an_issue(&world, given.project).await?;
    let under = |parent: String| {
        json!({
            "project": given.project.to_string(),
            "issue_type": kind,
            "title": "A step",
            "parent": parent,
        })
    };
    // A parent the agent was not handed.
    let input = under(not_handed.to_string());
    let real = used(&world, &given.key, "create_issue", input.clone()).await;
    let preview = previewed(&world, &given.key, "create_issue", input).await;
    assert!(real.is_err(), "the real call is refused");
    assert_eq!(said(&preview), said(&real));
    // A handed parent while no attempt runs.
    let input = under(given.issue.to_string());
    let real = used(&world, &given.key, "create_issue", input.clone()).await;
    let preview = previewed(&world, &given.key, "create_issue", input.clone()).await;
    assert!(real.is_err(), "no attempt runs");
    assert_eq!(said(&preview), said(&real));
    // While an attempt runs both are let through, and the preview shows the parent.
    start_attempt(&world, &given.key, given.issue).await?;
    let preview = previewed(&world, &given.key, "create_issue", input).await?;
    assert!(preview.text.contains("parent"), "{}", preview.text);
    Ok(())
}

#[tokio::test]
async fn m7_f225_an_agent_previews_no_change_while_no_attempt_runs() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let issue = given.issue.to_string();
    let version = 1;
    for (tool, input) in [
        (
            "update_issue",
            json!({"issue": issue, "version": version, "title": "New title"}),
        ),
        ("add_comment", json!({"issue": issue, "text": "A note"})),
    ] {
        let real = used(&world, &given.key, tool, input.clone()).await;
        let preview = previewed(&world, &given.key, tool, input).await;
        assert!(real.is_err(), "{tool}: no attempt runs");
        assert_eq!(said(&preview), said(&real), "{tool}");
    }
    Ok(())
}

#[tokio::test]
async fn m7_f225_a_person_previews_a_sub_issue_in_another_project_like_the_real_call() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    a_project(&world, "OTH").await?;
    let parent = issue_with(&world, "WEB", "Parent", "d", BTreeMap::new()).await?;
    let input = json!({
        "project": "OTH",
        "issue_type": a_type(&world).await?.to_string(),
        "title": "Elsewhere",
        "parent": parent,
    });
    let mut real = call(&token(&world), "create_issue", input.clone());
    real.caller = ToolCaller::AiApp;
    let real = call_tool(&world.ctx(), real).await;
    let mut preview = call(&token(&world), "create_issue", input);
    preview.caller = ToolCaller::AiApp;
    preview.mode = ToolMode::Preview;
    let preview = call_tool(&world.ctx(), preview).await;
    assert!(real.is_err(), "the real call is refused");
    assert_eq!(said(&preview), said(&real));
    Ok(())
}

/// A call that saves what it asks for, as an app with the scope to change does.
fn applying(who: &str, tool: &str, input: serde_json::Value) -> crate::ai::tools::ToolCallQuery {
    let mut query = call(who, tool, input);
    query.mode = ToolMode::Apply;
    query
}

#[tokio::test]
async fn plan_01_a_tool_refuses_a_start_after_the_due_date_like_every_other_way_in() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    let key = issue_with(&world, "WEB", "Plan", "d", BTreeMap::new()).await?;
    let kind = a_type(&world).await?.to_string();
    let who = token(&world);
    let backwards = json!({"start_date": "2026-11-10", "due_date": "2026-11-03"});
    let mut update = json!({"issue": key, "version": 1});
    let mut create = json!({"project": "WEB", "issue_type": kind, "title": "New plan"});
    for input in [&mut update, &mut create] {
        if let (Some(input), Some(dates)) = (input.as_object_mut(), backwards.as_object()) {
            input.extend(dates.clone());
        }
    }
    for (tool, input) in [("update_issue", update), ("create_issue", create)] {
        let refused = call_tool(&world.ctx(), applying(&who, tool, input)).await;
        assert!(
            matches!(
                refused,
                Err(AppError::Refused(ErrorCode::IssueStartAfterDue))
            ),
            "{tool}: {refused:?}"
        );
    }
    // A text that is no date is no request, and an order that is right is saved.
    let nonsense = json!({"issue": key, "version": 1, "due_date": "soon"});
    let refused = call_tool(&world.ctx(), applying(&who, "update_issue", nonsense)).await;
    assert!(matches!(
        refused,
        Err(AppError::Refused(ErrorCode::InvalidRequest))
    ));
    let fine =
        json!({"issue": key, "version": 1, "start_date": "2026-11-03", "due_date": "2026-11-10"});
    call_tool(&world.ctx(), applying(&who, "update_issue", fine)).await?;
    Ok(())
}

/// The value of a line `name: value` of an answer, at any depth.
fn line_value<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.trim_start().strip_prefix(name)?.strip_prefix(": "))
}

#[tokio::test]
async fn ai_21_a_created_issue_is_named_by_its_id_its_readable_id_and_its_version() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    let who = token(&world);
    let input = json!({
        "project": "WEB",
        "issue_type": a_type(&world).await?.to_string(),
        "title": "Brakes squeak",
    });
    // The preview says which project the issue is new in, and has no ID to name yet.
    let mut asked = call(&who, "create_issue", input.clone());
    asked.mode = ToolMode::Preview;
    let preview = call_tool(&world.ctx(), asked).await?.into_value();
    let new = "\nnew_issue:\n  short_name: WEB\n";
    assert!(preview.text.contains(new), "{}", preview.text);
    assert_eq!(line_value(&preview.text, "key"), None, "{}", preview.text);
    let made = call_tool(&world.ctx(), applying(&who, "create_issue", input))
        .await?
        .into_value();
    let key = line_value(&made.text, "key").ok_or(made.text.clone())?;
    let id = line_value(&made.text, "id").ok_or(made.text.clone())?;
    assert!(key.starts_with("WEB-"), "{}", made.text);
    assert_eq!(
        line_value(&made.text, "version"),
        Some("1"),
        "{}",
        made.text
    );
    // Both names reach the issue that was made, before any search could find it.
    for name in [key, id] {
        let read = call_tool(
            &world.ctx(),
            call(&who, "get_issue", json!({"issue": name})),
        )
        .await?
        .into_value();
        assert_eq!(line_value(&read.text, "key"), Some(key), "{}", read.text);
    }
    Ok(())
}
