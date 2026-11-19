//! What the AI reads back from a tool (AI-21): a change that changes nothing says so and never that
//! it saved, an empty answer says that nothing was found, and an issue names the statuses the reader
//! may move it to. Time added by hand or by the AI may lie in the future (TIME-02).

use std::collections::BTreeMap;

use jiff::civil::{Date, date};
use serde_json::json;

use super::ai_tools::{call, issue_with, mark, read, without_nonces};
use super::notifications::levels;
use super::scenarios::{a_project, token};
use super::world::{Outcome, World};
use crate::ai::NeverSentTarget;
use crate::ai::tools::{ToolMode, call_tool};
use crate::roles::{AssignRole, CreateRole, Given, Subject, assign_role, create_role};
use crate::time::{AddTime, EntryDetails, add_time};
use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, FieldLevel};
use bb_core::fields::Field;
use bb_core::time::Minutes;
use bb_policy::Permission;

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

#[tokio::test]
async fn ai_21_get_issue_names_the_statuses_the_reader_may_move_it_to() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    let key = issue_with(&world, "WEB", "Old title", "d", BTreeMap::new()).await?;
    let text = read(&world, &token(&world), "get_issue", json!({"issue": key})).await?;
    let next: Vec<&str> = text
        .lines()
        .skip_while(|line| line.trim() != "next_statuses:")
        .skip(1)
        .take_while(|line| line.starts_with("    "))
        .filter_map(|line| line.trim().strip_prefix("key: "))
        .collect();
    assert!(next.contains(&"in-progress"), "{text}");
    // The keys only: the text of the buttons stays on the screen.
    assert!(!text.contains("button"), "{text}");
    // A reader who may not see the status gets no next statuses either.
    let admin = token(&world);
    let blind = create_role(
        &world.ctx(),
        CreateRole {
            session_token: admin.clone(),
            name: "Blind".to_owned(),
            permissions: vec![Permission::SeeProjects, Permission::SeeIssues],
            two_step: false,
        },
    )
    .await?
    .into_value()
    .id;
    levels(&world, blind, &[Field::Title], FieldLevel::CanSee).await?;
    levels(&world, blind, &[Field::Status], FieldLevel::Hidden).await?;
    let dave = world.person_named("dave@example.test", "Dave Dunn").await?;
    assign_role(
        &world.ctx(),
        AssignRole {
            session_token: admin,
            subject: Subject::Person(dave.person.id),
            given: Given::Role(blind),
            project: None,
            applies_to: AppliesTo::Organization,
        },
    )
    .await?;
    let unseen = read(
        &world,
        &dave.session_token,
        "get_issue",
        json!({"issue": key}),
    )
    .await?;
    assert!(unseen.contains("Old title"), "{unseen}");
    assert!(!unseen.contains("next_statuses"), "{unseen}");
    assert!(!unseen.contains("in-progress"), "{unseen}");
    // A status kept from AI takes its next statuses with it.
    mark(&world, NeverSentTarget::Field(Field::Status)).await?;
    let kept = read(&world, &token(&world), "get_issue", json!({"issue": key})).await?;
    assert!(!kept.contains("next_statuses"), "{kept}");
    Ok(())
}

#[tokio::test]
async fn time_02_add_time_and_log_time_take_a_day_in_the_future() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    let key = issue_with(&world, "WEB", "Old title", "d", BTreeMap::new()).await?;
    // The clock of the tests stands on 2027-01-15: the next day is in the future (ADR-0035,
    // amendment). The screen and the API book it with *Add time*, the AI with `log_time`.
    let tomorrow = date(2027, 1, 16);
    add_time(
        &world.ctx(),
        AddTime {
            session_token: token(&world),
            issue: key.clone(),
            date: tomorrow,
            minutes: Minutes::new(60),
            details: EntryDetails::default(),
        },
    )
    .await?;
    let mut logged = call(
        &token(&world),
        "log_time",
        json!({"issue": key, "date": tomorrow.to_string(), "minutes": 30}),
    );
    logged.mode = ToolMode::Apply;
    call_tool(&world.ctx(), logged).await?;
    let booked: Vec<(Date, u32)> = world.store.read(|state| {
        state
            .time_entries
            .iter()
            .map(|entry| (entry.date, entry.minutes.get()))
            .collect()
    });
    assert_eq!(booked, [(tomorrow, 60), (tomorrow, 30)]);
    Ok(())
}
