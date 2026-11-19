//! What a person lets their agent do beyond its work, and the hand-over by an agent (AI-31,
//! ADR-0029, amendment M9): both switches start off, only the agent's person changes them, and an
//! agent hands over only issues of its own work, to its person's other agents, giving no more than
//! it has, in a chain of at most three steps.

use bb_core::ErrorCode;
use bb_core::agent::AgentSwitches;
use bb_core::agent_work::{HandOverPower, MOST_HAND_OVER_DEPTH};
use bb_core::id::AgentId;
use bb_policy::StartingRole;
use serde_json::json;

use super::agent_tools::{previewed, used};
use super::agent_work::{Handed, handed, handed_with, start_attempt};
use super::agents::{an_agent, key_of, refused};
use super::scenarios::{Added, a_project, measured, text, token};
use super::workflows_rules::member;
use super::world::{Outcome, World};
use crate::agent_work::{AgentWorkQuery, issue_agent_work};
use crate::agents::{
    AcceptAgent, AgentQuery, AgentView, HandAgent, SetAgentSwitches, accept_agent, agent,
    hand_agent, set_agent_switches,
};
use crate::auth::AppError;

fn both(may_hand_over: bool, may_undo: bool) -> AgentSwitches {
    AgentSwitches {
        may_hand_over,
        may_undo,
    }
}

/// The agent as its person reads it.
pub(super) async fn agent_seen(
    world: &World,
    session: &str,
    id: AgentId,
) -> Result<AgentView, AppError> {
    Ok(agent(
        &world.ctx(),
        AgentQuery {
            session_token: session.to_owned(),
            id,
        },
    )
    .await?
    .into_value())
}

/// Sets both switches of the agent as its person, on the version it has now.
pub(super) async fn switched(
    world: &World,
    session: &str,
    id: AgentId,
    switches: AgentSwitches,
) -> Result<AgentView, AppError> {
    let version = agent_seen(world, session, id).await?.version;
    Ok(set_agent_switches(
        &world.ctx(),
        SetAgentSwitches {
            session_token: session.to_owned(),
            id,
            switches,
            version,
        },
    )
    .await?
    .into_value())
}

pub(crate) async fn set_agent_switches_scenario(world: &World) -> Added {
    a_project(world, "WEB").await?;
    let fixer = an_agent(world, &token(world), "Fixer")
        .await
        .map_err(text)?;
    measured(
        &world.store,
        set_agent_switches(
            &world.ctx(),
            SetAgentSwitches {
                session_token: token(world),
                id: fixer,
                switches: both(true, true),
                version: 1,
            },
        ),
    )
    .await
}

/// A handed-over issue with its attempt running, and a second agent of the same person.
struct TwoAgents {
    given: Handed,
    helper: AgentId,
    helper_key: String,
}

async fn two_agents(world: &World) -> Result<TwoAgents, AppError> {
    two_agents_with(world, |_| {}).await
}

/// [`two_agents`], with the person's hand-over to the first agent changed first.
async fn two_agents_with(
    world: &World,
    change: impl FnOnce(&mut crate::agent_work::HandOver),
) -> Result<TwoAgents, AppError> {
    let given = handed_with(world, change).await?;
    start_attempt(world, &given.key, given.issue).await?;
    let helper = an_agent(world, &given.team.session_token, "Helper").await?;
    let helper_key = key_of(world, &given.team.session_token, helper).await?;
    Ok(TwoAgents {
        given,
        helper,
        helper_key,
    })
}

/// The agent lets go of the issue so another may take it.
async fn give_up(world: &World, key: &str, issue: bb_core::id::IssueId) -> Result<(), AppError> {
    used(
        world,
        key,
        "give_up",
        json!({ "issue": issue.to_string(), "reason": "Handed on" }),
    )
    .await
    .map(drop)
}

fn to_helper(issue: bb_core::id::IssueId) -> serde_json::Value {
    json!({ "agent": "Helper", "issues": [issue.to_string()], "note": "Please check the tests" })
}

/// The agents that have work on the issue, as its person reads it.
async fn agents_on(world: &World, given: &Handed) -> Result<Vec<AgentId>, AppError> {
    Ok(issue_agent_work(
        &world.ctx(),
        AgentWorkQuery {
            session_token: given.team.session_token.clone(),
            issue: given.issue.to_string(),
        },
    )
    .await?
    .into_value()
    .work
    .iter()
    .map(|work| work.agent)
    .collect())
}

#[tokio::test]
async fn ai_31_both_switches_of_a_new_agent_are_off() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    a_project(&world, "WEB").await?;
    let session = token(&world);
    let fixer = an_agent(&world, &session, "Fixer").await?;
    assert_eq!(
        agent_seen(&world, &session, fixer).await?.switches,
        AgentSwitches::default()
    );
    assert_eq!(AgentSwitches::default(), both(false, false));
    Ok(())
}

#[tokio::test]
async fn ai_31_the_switches_go_off_when_the_agent_is_handed_to_another_person() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let project = a_project(&world, "WEB").await?;
    let session = token(&world);
    let fixer = an_agent(&world, &session, "Fixer").await?;
    let on = switched(&world, &session, fixer, both(true, true)).await?;
    let team = member(
        &world,
        "team@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    hand_agent(
        &world.ctx(),
        HandAgent {
            session_token: session,
            id: fixer,
            to: team.person.id,
            version: on.version,
        },
    )
    .await?;
    accept_agent(
        &world.ctx(),
        AcceptAgent {
            session_token: team.session_token.clone(),
            id: fixer,
        },
    )
    .await?;
    // The new person has not chosen what the agent may do beyond its work.
    assert_eq!(
        agent_seen(&world, &team.session_token, fixer)
            .await?
            .switches,
        AgentSwitches::default()
    );
    Ok(())
}

#[tokio::test]
async fn ai_31_only_the_person_of_an_agent_changes_its_switches() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed(&world).await?;
    // The administrator sees every agent and may stop it, but does not widen what it does.
    let refusal = set_agent_switches(
        &world.ctx(),
        SetAgentSwitches {
            session_token: token(&world),
            id: given.agent,
            switches: both(true, true),
            version: 1,
        },
    )
    .await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::NotFound));
    // Nor does the agent itself.
    let own = set_agent_switches(
        &world.ctx(),
        SetAgentSwitches {
            session_token: given.key.clone(),
            id: given.agent,
            switches: both(true, true),
            version: 1,
        },
    )
    .await;
    assert!(own.is_err());
    let after = switched(
        &world,
        &given.team.session_token,
        given.agent,
        both(true, false),
    )
    .await?;
    assert_eq!(after.switches, both(true, false));
    Ok(())
}

#[tokio::test]
async fn ai_31_an_agent_hands_over_only_when_its_switch_is_on() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let two = two_agents(&world).await?;
    let given = &two.given;
    let refusal = used(&world, &given.key, "hand_over_work", to_helper(given.issue)).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    let preview = previewed(&world, &given.key, "hand_over_work", to_helper(given.issue)).await;
    assert!(refused(&preview.unwrap_err(), ErrorCode::Forbidden));
    assert_eq!(agents_on(&world, given).await?, vec![given.agent]);
    switched(
        &world,
        &given.team.session_token,
        given.agent,
        both(true, false),
    )
    .await?;
    // The preview writes nothing.
    previewed(&world, &given.key, "hand_over_work", to_helper(given.issue)).await?;
    assert_eq!(agents_on(&world, given).await?, vec![given.agent]);
    used(&world, &given.key, "hand_over_work", to_helper(given.issue)).await?;
    let mut on = agents_on(&world, given).await?;
    on.sort();
    let mut expected = vec![given.agent, two.helper];
    expected.sort();
    assert_eq!(on, expected);
    // The helper takes it once Fixer has let go of the issue: two agents never work on it at once.
    let blocked = used(
        &world,
        &two.helper_key,
        "take",
        json!({ "issue": given.issue.to_string() }),
    )
    .await;
    assert!(refused(&blocked.unwrap_err(), ErrorCode::AgentWorkNotReady));
    give_up(&world, &given.key, given.issue).await?;
    let take = used(
        &world,
        &two.helper_key,
        "take",
        json!({ "issue": given.issue.to_string() }),
    )
    .await?;
    assert!(take.text.contains("agent-working"), "{}", take.text);
    Ok(())
}

#[tokio::test]
async fn ai_31_an_agent_hands_over_only_to_another_agent_of_its_person() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let two = two_agents(&world).await?;
    let given = &two.given;
    switched(
        &world,
        &given.team.session_token,
        given.agent,
        both(true, false),
    )
    .await?;
    let to_itself = used(
        &world,
        &given.key,
        "hand_over_work",
        json!({ "agent": "Fixer", "issues": [given.issue.to_string()] }),
    )
    .await;
    assert!(refused(&to_itself.unwrap_err(), ErrorCode::InvalidRequest));
    // An agent of somebody else is not there for it.
    let stranger = an_agent(&world, &token(&world), "Stranger").await?;
    let elsewhere = used(
        &world,
        &given.key,
        "hand_over_work",
        json!({ "agent": stranger.to_string(), "issues": [given.issue.to_string()] }),
    )
    .await;
    assert!(refused(&elsewhere.unwrap_err(), ErrorCode::NotFound));
    Ok(())
}

#[tokio::test]
async fn ai_31_an_agent_hands_over_only_issues_of_its_own_work_and_gives_no_more_than_it_has()
-> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    // Its person approves each plan, gives it one attempt, half an hour and one day.
    let two = two_agents_with(&world, |asked| {
        asked.plan_needs_approval = true;
        asked.budget_attempts = Some(1);
        asked.budget_minutes = Some(30);
        asked.days = Some(1);
    })
    .await?;
    let given = &two.given;
    switched(
        &world,
        &given.team.session_token,
        given.agent,
        both(true, false),
    )
    .await?;
    // An issue of the project that was not handed to it.
    let other = super::scenarios_issues::an_issue(&world, given.project).await?;
    let not_mine = used(&world, &given.key, "hand_over_work", to_helper(other)).await;
    assert!(not_mine.is_err());
    // Its own hand-over lets it change nothing: it cannot let the helper change the status.
    let mut more = to_helper(given.issue);
    more["powers"] = json!([HandOverPower::ChangeStatus.code()]);
    let refusal = used(&world, &given.key, "hand_over_work", more).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    assert_eq!(agents_on(&world, given).await?, vec![given.agent]);
    // What it hands on keeps the approval of the plan, its budget and its end: the tool asks for
    // none of them.
    used(&world, &given.key, "hand_over_work", to_helper(given.issue)).await?;
    let work = issue_agent_work(
        &world.ctx(),
        AgentWorkQuery {
            session_token: given.team.session_token.clone(),
            issue: given.issue.to_string(),
        },
    )
    .await?
    .into_value()
    .work;
    let helpers = work
        .iter()
        .find(|each| each.agent == two.helper)
        .ok_or_else(|| AppError::Internal("the helper has no work".to_owned()))?;
    let budget = helpers
        .budget
        .as_ref()
        .ok_or_else(|| AppError::Internal("the helper's hand-over is gone".to_owned()))?;
    assert!(budget.plan_needs_approval);
    assert_eq!(budget.attempts, 1);
    assert_eq!(budget.minutes, Some(30));
    assert!(
        helpers
            .ends_at
            .is_some_and(|end| end <= given.hand_over.ends_at)
    );
    Ok(())
}

#[tokio::test]
async fn ai_31_a_hand_over_by_an_agent_is_recorded_as_done_by_the_agent() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let two = two_agents(&world).await?;
    let given = &two.given;
    switched(
        &world,
        &given.team.session_token,
        given.agent,
        both(true, false),
    )
    .await?;
    used(&world, &given.key, "hand_over_work", to_helper(given.issue)).await?;
    let shared = world.store.lock();
    let event = shared
        .events
        .iter()
        .rev()
        .find(|event| event.entity.kind.as_str() == "hand_over")
        .ok_or("no event of a hand-over")?;
    assert!(matches!(
        event.actor,
        bb_core::event::Actor::Agent { agent, .. } if agent == given.agent
    ));
    let stored = shared
        .state
        .work
        .hand_overs
        .iter()
        .find(|hand_over| hand_over.agent == two.helper)
        .ok_or("no hand-over to the helper")?;
    assert_eq!(stored.by_agent, Some(given.agent));
    assert_eq!(stored.depth, 1);
    assert_eq!(stored.person, given.team.person.id);
    Ok(())
}

#[tokio::test]
async fn ai_31_a_chain_of_hand_overs_stops_at_its_depth() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed(&world).await?;
    let session = given.team.session_token.clone();
    switched(&world, &session, given.agent, both(true, false)).await?;
    start_attempt(&world, &given.key, given.issue).await?;
    // A chain of agents, each handing the issue on to the next.
    let mut from_key = given.key.clone();
    let steps = usize::try_from(MOST_HAND_OVER_DEPTH)?;
    for step in 1..=steps + 1 {
        let name = format!("Step {step}");
        let next = an_agent(&world, &session, &name).await?;
        let next_key = key_of(&world, &session, next).await?;
        switched(&world, &session, next, both(true, false)).await?;
        let asked = json!({ "agent": name, "issues": [given.issue.to_string()] });
        let answer = used(&world, &from_key, "hand_over_work", asked).await;
        if step <= steps {
            answer?;
            give_up(&world, &from_key, given.issue).await?;
            start_attempt(&world, &next_key, given.issue).await?;
        } else {
            assert!(refused(
                &answer.unwrap_err(),
                ErrorCode::AgentHandOverTooDeep
            ));
        }
        from_key = next_key;
    }
    Ok(())
}

#[tokio::test]
async fn ai_21_preview_hand_over_work_writes_nothing() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let two = two_agents(&world).await?;
    let given = &two.given;
    switched(
        &world,
        &given.team.session_token,
        given.agent,
        both(true, false),
    )
    .await?;
    let asked = previewed(&world, &given.key, "hand_over_work", to_helper(given.issue)).await?;
    assert!(asked.text.contains("Helper") || asked.text.contains(&two.helper.to_string()));
    assert_eq!(agents_on(&world, given).await?, vec![given.agent]);
    Ok(())
}
