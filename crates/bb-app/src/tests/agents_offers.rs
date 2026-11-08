//! What becomes of the agents handed from one person to another, and of what they brought (AI-30,
//! AI-37): the offer can be taken back or declined, a stopped agent is offered to nobody, work accepted by
//! itself stays with the person it was given for, and nobody else changes the limits of an agent.

use bb_core::ErrorCode;
use bb_core::agent::{AgentLimits, AgentState};
use bb_core::agent_work::AgentWorkState;
use bb_core::id::AgentId;
use bb_policy::StartingRole;

use super::agent_work::{ground, plain_hand_over};
use super::agent_work_review::report;
use super::agents::refused;
use super::scenarios::token;
use super::scenarios_issues::an_issue;
use super::workflows_rules::member;
use super::world::{Outcome, World};
use crate::agent_work::{
    AskQuestion, MyAgentWorkQuery, ask_question, hand_over, my_agent_work, report_attempt,
};
use crate::agents::{
    AcceptAgent, AgentAction, ChangeAgent, HandAgent, SetAgentLimits, TakeBackOffer, accept_agent,
    change_agent, hand_agent, set_agent_limits, take_back_offer,
};
use crate::auth::{AppError, Started};

async fn another(world: &World, project: bb_core::id::ProjectId) -> Result<Started, AppError> {
    member(
        world,
        "other@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await
}

async fn offered_to(world: &World, who: &Started) -> Result<usize, AppError> {
    Ok(my_agent_work(
        &world.ctx(),
        MyAgentWorkQuery {
            session_token: who.session_token.clone(),
        },
    )
    .await?
    .into_value()
    .offered
    .len())
}

fn offer(from: &Started, agent: AgentId, to: &Started, version: i64) -> HandAgent {
    HandAgent {
        session_token: from.session_token.clone(),
        id: agent,
        to: to.person.id,
        version,
    }
}

fn take_back(who: &Started, agent: AgentId) -> TakeBackOffer {
    TakeBackOffer {
        session_token: who.session_token.clone(),
        id: agent,
    }
}

#[tokio::test]
async fn ai_30_an_offer_can_be_taken_back_or_declined_and_nobody_else_ends_it() -> Outcome {
    let world = World::new().await?;
    let given = ground(&world).await?;
    let other = another(&world, given.project).await?;
    let stranger = member(
        &world,
        "stranger@example.test",
        StartingRole::TeamMember,
        given.project,
    )
    .await?;
    hand_agent(&world.ctx(), offer(&given.team, given.agent, &other, 1)).await?;
    assert_eq!(offered_to(&world, &other).await?, 1);
    // Somebody it is not offered to, and is not the person's, finds nothing to end.
    let refusal = take_back_offer(&world.ctx(), take_back(&stranger, given.agent)).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::NotFound));
    // The person it is offered to declines it.
    take_back_offer(&world.ctx(), take_back(&other, given.agent)).await?;
    assert_eq!(offered_to(&world, &other).await?, 0);
    // Nothing is offered now, so there is nothing to take back, and accepting finds nothing.
    let again = take_back_offer(&world.ctx(), take_back(&given.team, given.agent)).await;
    assert!(refused(&again.unwrap_err(), ErrorCode::NotFound));
    let accepted = accept_agent(
        &world.ctx(),
        AcceptAgent {
            session_token: other.session_token.clone(),
            id: given.agent,
        },
    )
    .await;
    assert!(refused(&accepted.unwrap_err(), ErrorCode::NotFound));
    // The person responsible takes an offer back.
    let version = world.store.read(|state| {
        state
            .agents
            .iter()
            .find(|a| a.id == given.agent)
            .map(|a| a.version)
    });
    hand_agent(
        &world.ctx(),
        offer(&given.team, given.agent, &other, version.unwrap_or(0)),
    )
    .await?;
    take_back_offer(&world.ctx(), take_back(&given.team, given.agent)).await?;
    assert_eq!(offered_to(&world, &other).await?, 0);
    Ok(())
}

#[tokio::test]
async fn ai_30_a_stopped_agent_is_offered_to_nobody() -> Outcome {
    let world = World::new().await?;
    let given = ground(&world).await?;
    let other = another(&world, given.project).await?;
    hand_agent(&world.ctx(), offer(&given.team, given.agent, &other, 1)).await?;
    change_agent(
        &world.ctx(),
        ChangeAgent {
            session_token: given.team.session_token.clone(),
            id: given.agent,
            action: AgentAction::Stop,
            version: 2,
        },
    )
    .await?;
    assert_eq!(offered_to(&world, &other).await?, 0);
    let accepted = accept_agent(
        &world.ctx(),
        AcceptAgent {
            session_token: other.session_token.clone(),
            id: given.agent,
        },
    )
    .await;
    assert!(refused(&accepted.unwrap_err(), ErrorCode::NotFound));
    Ok(())
}

#[tokio::test]
async fn ai_37_work_accepted_by_itself_stays_with_the_person_it_was_given_for() -> Outcome {
    let world = World::new().await?;
    let given = ground(&world).await?;
    let other = another(&world, given.project).await?;
    hand_over(
        &world.ctx(),
        plain_hand_over(&given.team.session_token, given.agent, &[given.issue]),
    )
    .await?;
    // The work was accepted by itself and nobody has looked at it.
    world
        .store
        .lock()
        .state
        .work
        .works
        .iter_mut()
        .for_each(|work| {
            work.closed = true;
            work.accepted_by_itself = true;
        });
    let before = my_agent_work(
        &world.ctx(),
        MyAgentWorkQuery {
            session_token: given.team.session_token.clone(),
        },
    )
    .await?
    .into_value();
    assert_eq!(before.items.len(), 1);
    let version = world.store.read(|state| {
        state
            .agents
            .iter()
            .find(|a| a.id == given.agent)
            .map_or(0, |a| a.version)
    });
    hand_agent(
        &world.ctx(),
        offer(&given.team, given.agent, &other, version),
    )
    .await?;
    accept_agent(
        &world.ctx(),
        AcceptAgent {
            session_token: other.session_token.clone(),
            id: given.agent,
        },
    )
    .await?;
    let after = my_agent_work(
        &world.ctx(),
        MyAgentWorkQuery {
            session_token: other.session_token.clone(),
        },
    )
    .await?
    .into_value();
    assert!(
        after.items.is_empty(),
        "the new person does not inherit the rows"
    );
    // The person it was given for still sees it, though the agent is not theirs any more.
    let kept = my_agent_work(
        &world.ctx(),
        MyAgentWorkQuery {
            session_token: given.team.session_token.clone(),
        },
    )
    .await?
    .into_value();
    assert_eq!(kept.items.len(), 1);
    Ok(())
}

#[tokio::test]
async fn ai_37_what_waits_for_the_person_comes_before_work_that_is_newer() -> Outcome {
    let world = World::new().await?;
    let given = ground(&world).await?;
    let second = an_issue(&world, given.project).await?;
    hand_over(
        &world.ctx(),
        plain_hand_over(
            &given.team.session_token,
            given.agent,
            &[given.issue, second],
        ),
    )
    .await?;
    // The older issue has a question; the newer one only waits for its agent.
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    ask_question(
        &world.ctx(),
        AskQuestion {
            session_token: given.key.clone(),
            issue: given.issue.to_string(),
            text: "Which browser?".to_owned(),
        },
    )
    .await?;
    let found = my_agent_work(
        &world.ctx(),
        MyAgentWorkQuery {
            session_token: given.team.session_token.clone(),
        },
    )
    .await?
    .into_value();
    assert_eq!(found.items.len(), 2);
    assert_eq!(found.items[0].work.state, AgentWorkState::WaitingForYou);
    assert!(!found.truncated);
    Ok(())
}

#[tokio::test]
async fn ai_37_the_agents_handed_to_a_person_are_cut_at_the_most_and_say_so() -> Outcome {
    let world = World::new().await?;
    let given = ground(&world).await?;
    let other = another(&world, given.project).await?;
    // More agents are offered to the person than a list of agents holds.
    let template = world
        .store
        .read(|state| state.agents.iter().find(|a| a.id == given.agent).cloned())
        .ok_or("no agent")?;
    for number in 0..=bb_core::agent::MOST_AGENTS {
        let mut agent = template.clone();
        agent.id = AgentId::generate(&mut world.ctx().ids());
        agent.name = format!("Agent {number}");
        agent.offered_to = Some(other.person.id);
        world.store.lock().state.agents.push(agent);
    }
    let found = my_agent_work(
        &world.ctx(),
        MyAgentWorkQuery {
            session_token: other.session_token.clone(),
        },
    )
    .await?
    .into_value();
    assert_eq!(found.offered.len(), bb_core::agent::MOST_AGENTS);
    assert!(found.offered_truncated);
    Ok(())
}

#[tokio::test]
async fn ai_30_nobody_but_the_person_changes_the_limits_of_an_agent() -> Outcome {
    let world = World::new().await?;
    let given = ground(&world).await?;
    let asking = |session: &str| SetAgentLimits {
        session_token: session.to_owned(),
        id: given.agent,
        limits: AgentLimits {
            changes_a_minute: 10,
            issues_at_once: 2,
            attempts_on_one_issue: 3,
        },
        version: 1,
    };
    // Another person and an administrator find no such agent; the agent stays as it was.
    let other = another(&world, given.project).await?;
    let by_other = set_agent_limits(&world.ctx(), asking(&other.session_token)).await;
    assert!(refused(&by_other.unwrap_err(), ErrorCode::NotFound));
    let by_admin = set_agent_limits(&world.ctx(), asking(&token(&world))).await;
    assert!(refused(&by_admin.unwrap_err(), ErrorCode::NotFound));
    let kept = world.store.read(|state| {
        state
            .agents
            .iter()
            .find(|a| a.id == given.agent)
            .map(|a| (a.changes_a_minute, a.state))
    });
    assert_eq!(
        kept,
        Some((AgentLimits::default().changes_a_minute, AgentState::Active))
    );
    Ok(())
}

pub(crate) async fn take_back_offer_scenario(world: &World) -> super::scenarios::Added {
    let given = ground(world).await.map_err(super::scenarios::text)?;
    let other = another(world, given.project)
        .await
        .map_err(super::scenarios::text)?;
    hand_agent(&world.ctx(), offer(&given.team, given.agent, &other, 1))
        .await
        .map_err(super::scenarios::text)?;
    super::scenarios::measured(
        &world.store,
        take_back_offer(&world.ctx(), take_back(&given.team, given.agent)),
    )
    .await
}
