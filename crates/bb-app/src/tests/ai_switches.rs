//! M7 WP-15: each kind of AI is its own switch, for the organization and for each project. The
//! assistant (`ai.enabled`), AI apps (`ai-apps.enabled`) and agents (`agents.enabled`) follow only
//! their own switch along the chain of a project; *Never sent to AI* holds for all three.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::ai::ToolCaller;
use bb_core::id::ProjectId;
use bb_core::settings::SettingValue;
use serde_json::json;

use super::ai_tools::{call, issue_with};
use super::assistant::ai_on;
use super::oauth::LOOPBACK;
use super::scenarios::{a_project, token};
use super::world::{Outcome, World};
use crate::ai::tools::call_tool;
use crate::ai::{ChangeAiSetup, ChangeProjectAi, change_ai_setup, change_project_ai};
use crate::ai::{NeverSentTarget, SetNeverSent, set_never_sent};
use crate::auth::{AppError, RegisterApp, RegisterOutcome, register_app};
use crate::projects::{CreateProject, create_project};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

const KINDS: [(ToolCaller, &str); 3] = [
    (ToolCaller::Agent, "agents.enabled"),
    (ToolCaller::AiApp, "ai-apps.enabled"),
    (ToolCaller::Assistant, "ai.enabled"),
];

fn short_name_of(world: &World, id: ProjectId) -> Result<String, String> {
    world
        .store
        .lock()
        .state
        .projects
        .iter()
        .find(|project| project.id == id)
        .map(|project| project.short_name.as_str().to_owned())
        .ok_or_else(|| "no such project".to_owned())
}

async fn set(world: &World, scope: SettingsScope, key: &str, on: bool) -> Outcome {
    // The assistant's own switch is changed on its own screens (it needs the agreement).
    if key == "ai.enabled" {
        match scope {
            SettingsScope::Project(project) => {
                let short = short_name_of(world, project)?;
                change_project_ai(
                    &world.ctx(),
                    ChangeProjectAi {
                        session_token: token(world),
                        project: short,
                        enabled: Some(on),
                    },
                )
                .await?;
            }
            _ => {
                change_ai_setup(
                    &world.ctx(),
                    ChangeAiSetup {
                        enabled: Some(on),
                        ..super::scenarios_ai::setup(world)
                    },
                )
                .await?;
            }
        }
        return Ok(());
    }
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(world),
            scope,
            key: key.to_owned(),
            value: SettingValue::Bool(on),
        },
    )
    .await?;
    Ok(())
}

/// Whether a tool of the kind reaches the issue.
async fn reaches(world: &World, caller: ToolCaller, issue: &str) -> Result<bool, String> {
    let mut query = call(&token(world), "get_issue", json!({ "issue": issue }));
    query.caller = caller;
    match call_tool(&world.ctx(), query).await {
        Ok(_) => Ok(true),
        Err(AppError::Refused(ErrorCode::NotFound)) => Ok(false),
        Err(other) => Err(format!("{caller:?}: {other:?}")),
    }
}

async fn who_reaches(world: &World, issue: &str) -> Result<Vec<bool>, String> {
    let mut reached = Vec::new();
    for (caller, _) in KINDS {
        reached.push(reaches(world, caller, issue).await?);
    }
    Ok(reached)
}

async fn a_world_with_an_issue() -> Result<(World, ProjectId, String), String> {
    let world = World::new().await?;
    let project = a_project(&world, "OPS").await?;
    ai_on(&world).await.map_err(|error| error.to_string())?;
    let issue = issue_with(&world, "OPS", "Pager", "plain", BTreeMap::default()).await?;
    Ok((world, project, issue))
}

#[tokio::test]
async fn m7_wp15_a_project_switch_shuts_out_only_its_own_kind() -> Outcome {
    let (world, project, issue) = a_world_with_an_issue().await?;
    assert_eq!(who_reaches(&world, &issue).await?, [true, true, true]);
    // Each kind off in the project, alone: the other two still reach it.
    for (at, (_, key)) in KINDS.iter().enumerate() {
        set(&world, SettingsScope::Project(project), key, false).await?;
        let mut expected = [true, true, true];
        expected[at] = false;
        assert_eq!(who_reaches(&world, &issue).await?, expected, "{key}");
        set(&world, SettingsScope::Project(project), key, true).await?;
        assert_eq!(who_reaches(&world, &issue).await?, [true, true, true]);
    }
    Ok(())
}

#[tokio::test]
async fn m7_wp15_a_switch_follows_the_nearest_value_along_the_project_chain() -> Outcome {
    let (world, parent, _) = a_world_with_an_issue().await?;
    let child = create_project(
        &world.ctx(),
        CreateProject {
            session_token: token(&world),
            parent: Some(parent),
            short_name: "NIGHT".to_owned(),
            name: "Night".to_owned(),
        },
    )
    .await
    .map_err(|error| format!("{error:?}"))?
    .into_value()
    .id;
    let night = issue_with(&world, "NIGHT", "Alarm", "plain", BTreeMap::default()).await?;
    // Off in the parent: the sub-project is off, for that kind only.
    set(
        &world,
        SettingsScope::Project(parent),
        "ai-apps.enabled",
        false,
    )
    .await?;
    assert_eq!(who_reaches(&world, &night).await?, [true, false, true]);
    // On again in the sub-project: it is on below a parent that is off.
    set(
        &world,
        SettingsScope::Project(child),
        "ai-apps.enabled",
        true,
    )
    .await?;
    assert_eq!(who_reaches(&world, &night).await?, [true, true, true]);
    // The same for the assistant and for agents.
    set(
        &world,
        SettingsScope::Project(parent),
        "agents.enabled",
        false,
    )
    .await?;
    set(&world, SettingsScope::Project(parent), "ai.enabled", false).await?;
    assert_eq!(who_reaches(&world, &night).await?, [false, true, false]);
    Ok(())
}

#[tokio::test]
async fn m7_wp15_never_sent_to_ai_holds_for_all_three_kinds() -> Outcome {
    let (world, _, issue) = a_world_with_an_issue().await?;
    set_never_sent(
        &world.ctx(),
        SetNeverSent {
            session_token: token(&world),
            target: NeverSentTarget::Project("OPS".to_owned()),
            marked: true,
            version: 0,
        },
    )
    .await?;
    assert_eq!(who_reaches(&world, &issue).await?, [false, false, false]);
    Ok(())
}

async fn register(world: &World) -> Result<bool, String> {
    let outcome = register_app(
        &world.ctx(),
        RegisterApp {
            name: "Claude Code".to_owned(),
            return_addresses: vec![LOOPBACK.to_owned()],
        },
    )
    .await
    .map_err(|error| format!("{error:?}"))?;
    Ok(matches!(outcome, RegisterOutcome::Registered(_)))
}

#[tokio::test]
async fn m7_wp15_the_assistant_off_leaves_ai_apps_and_agents_on_for_the_organization() -> Outcome {
    let world = World::new().await?;
    // The assistant is off at the start; apps and agents are on.
    assert!(register(&world).await?);
    assert!(world.ctx().agents_on_in_the_organization().await?);
    ai_on(&world).await?;
    set(&world, SettingsScope::Organization, "ai.enabled", false).await?;
    assert!(register(&world).await?, "apps do not need the assistant");
    assert!(world.ctx().agents_on_in_the_organization().await?);
    Ok(())
}

#[tokio::test]
async fn m7_wp15_ai_apps_off_leaves_agents_and_the_assistant_on_for_the_organization() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    set(
        &world,
        SettingsScope::Organization,
        "ai-apps.enabled",
        false,
    )
    .await?;
    assert!(!register(&world).await?);
    assert!(world.ctx().agents_on_in_the_organization().await?);
    set(&world, SettingsScope::Organization, "ai-apps.enabled", true).await?;
    set(&world, SettingsScope::Organization, "agents.enabled", false).await?;
    assert!(register(&world).await?, "apps do not need agents");
    assert!(!world.ctx().agents_on_in_the_organization().await?);
    Ok(())
}
