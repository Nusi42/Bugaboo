//! An agent undoes its own runs (AI-38, ADR-0029, amendment M9): only when its person switched on
//! *May undo its own runs*, only the runs of its own attempts, through *Undo the run*, and recorded
//! as done by the agent.

use bb_core::ErrorCode;
use bb_core::agent_work::HandOverPower;
use serde_json::json;

use super::agent_tools::{previewed, used};
use super::agent_tools_more::version_of;
use super::agent_work::{Handed, handed_with, query, start_attempt};
use super::agents::refused;
use super::agents_switches::switched;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::issues::issue;

/// A hand-over that lets the agent change the title, with its attempt running.
pub(super) async fn handed_title(world: &World) -> Result<Handed, AppError> {
    let given = handed_with(world, |asked| {
        asked.powers = vec![HandOverPower::ChangeFields];
        asked.fields = vec!["title".to_owned()];
    })
    .await?;
    start_attempt(world, &given.key, given.issue).await?;
    Ok(given)
}

/// The agent changes the title in its attempt.
pub(super) async fn retitled_by_agent(world: &World, given: &Handed, title: &str) -> Outcome {
    let version = version_of(world, given).await?;
    used(
        world,
        &given.key,
        "update_issue",
        json!({ "issue": given.issue.to_string(), "version": version, "title": title }),
    )
    .await?;
    Ok(())
}

pub(super) async fn title_now(world: &World, given: &Handed) -> Result<String, AppError> {
    Ok(
        issue(&world.ctx(), query(&given.team.session_token, given.issue))
            .await?
            .into_value()
            .issue
            .title,
    )
}

pub(super) fn undo_input(given: &Handed) -> serde_json::Value {
    json!({ "issue": given.issue.to_string() })
}

pub(super) fn may_undo() -> bb_core::agent::AgentSwitches {
    bb_core::agent::AgentSwitches {
        may_hand_over: false,
        may_undo: true,
    }
}

#[tokio::test]
async fn ai_38_an_agent_undoes_its_own_run_only_when_its_switch_is_on() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed_title(&world).await?;
    let before = title_now(&world, &given).await?;
    retitled_by_agent(&world, &given, "A wrong title").await?;
    let refusal = used(&world, &given.key, "undo_my_run", undo_input(&given)).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    let preview = previewed(&world, &given.key, "undo_my_run", undo_input(&given)).await;
    assert!(refused(&preview.unwrap_err(), ErrorCode::Forbidden));
    assert_eq!(title_now(&world, &given).await?, "A wrong title");
    switched(&world, &given.team.session_token, given.agent, may_undo()).await?;
    let asked = previewed(&world, &given.key, "undo_my_run", undo_input(&given)).await?;
    assert!(asked.text.contains("goes_back: 1"), "{}", asked.text);
    assert_eq!(title_now(&world, &given).await?, "A wrong title");
    let done = used(&world, &given.key, "undo_my_run", undo_input(&given)).await?;
    assert!(done.text.contains("goes_back: 1"), "{}", done.text);
    assert_eq!(title_now(&world, &given).await?, before);
    Ok(())
}

#[tokio::test]
async fn ai_38_an_agent_never_undoes_a_run_of_its_person_or_of_another_agent() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed_title(&world).await?;
    switched(&world, &given.team.session_token, given.agent, may_undo()).await?;
    retitled_by_agent(&world, &given, "Changed by the agent").await?;
    // Another agent of the same person, allowed to undo, finds the run of Fixer not there.
    let session = given.team.session_token.clone();
    let helper = super::agents::an_agent(&world, &session, "Helper").await?;
    let helper_key = super::agents::key_of(&world, &session, helper).await?;
    switched(&world, &session, helper, may_undo()).await?;
    let fixers: Vec<_> = world
        .store
        .lock()
        .state
        .many_changes_runs
        .iter()
        .filter(|run| run.agent == Some(given.agent))
        .map(|run| run.id)
        .collect();
    assert!(!fixers.is_empty(), "the attempt of Fixer has a run");
    for run in fixers {
        let refusal = crate::runs::undo_run(
            &world.ctx(),
            crate::runs::UndoRun {
                session_token: helper_key.clone(),
                run,
                version: bb_core::run::UndoVersion(String::new()),
            },
        )
        .await;
        assert!(refused(&refusal.unwrap_err(), ErrorCode::NotFound));
    }
    // An issue that is not of its work has no run of its for it.
    let other = super::scenarios_issues::an_issue(&world, given.project).await?;
    let elsewhere = used(
        &world,
        &given.key,
        "undo_my_run",
        json!({ "issue": other.to_string() }),
    )
    .await;
    assert!(elsewhere.is_err());
    Ok(())
}

#[tokio::test]
async fn ai_38_an_undo_by_an_agent_is_recorded_as_done_by_the_agent() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed_title(&world).await?;
    switched(&world, &given.team.session_token, given.agent, may_undo()).await?;
    retitled_by_agent(&world, &given, "Changed by the agent").await?;
    used(&world, &given.key, "undo_my_run", undo_input(&given)).await?;
    let shared = world.store.lock();
    let undone = shared
        .events
        .iter()
        .rev()
        .find(|event| {
            event.entity.kind.as_str() == "change_run"
                && event.changes.iter().any(|change| change.field == "undone")
        })
        .ok_or("no event of the undo")?;
    assert!(matches!(
        undone.actor,
        bb_core::event::Actor::Agent { agent, .. } if agent == given.agent
    ));
    Ok(())
}

#[tokio::test]
async fn ai_21_preview_undo_my_run_puts_nothing_back() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed_title(&world).await?;
    switched(&world, &given.team.session_token, given.agent, may_undo()).await?;
    retitled_by_agent(&world, &given, "Changed by the agent").await?;
    let asked = previewed(&world, &given.key, "undo_my_run", undo_input(&given)).await?;
    assert!(asked.text.contains("goes_back: 1"), "{}", asked.text);
    assert_eq!(title_now(&world, &given).await?, "Changed by the agent");
    Ok(())
}
