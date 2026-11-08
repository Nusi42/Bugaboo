//! The work of a person's own agents in one list (AI-37, *My agents*).

use bb_core::agent_work::{AgentWorkState, WaitsFor};

use super::agent_work::{handed, report_attempt};
use super::agent_work_review::{ready, report};
use super::agents::an_agent;
use super::scenarios::token;
use super::world::{Outcome, World};
use crate::agent_work::{
    AskQuestion, MyAgentWork, MyAgentWorkQuery, ask_question, end_attempt, my_agent_work,
};
use crate::auth::AppError;
use bb_policy::Authorized;

fn mine(session: &str) -> MyAgentWorkQuery {
    MyAgentWorkQuery {
        session_token: session.to_owned(),
    }
}

async fn listed(world: &World, session: &str) -> Result<Authorized<MyAgentWork>, AppError> {
    my_agent_work(&world.ctx(), mine(session)).await
}

#[tokio::test]
async fn ai_37_my_agents_lists_what_each_piece_of_work_waits_for() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    // Handed over, not taken yet: in the list, waiting for the agent, not for the person.
    let first = listed(&world, &given.team.session_token)
        .await?
        .into_value();
    assert_eq!(first.items.len(), 1);
    assert_eq!(
        first.items[0].work.state,
        AgentWorkState::WaitingForTheAgent
    );
    assert_eq!(first.items[0].waits_for, None);
    assert_eq!(first.items[0].issue.id, given.issue);
    // A question waits for an answer.
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
    let asked = listed(&world, &given.team.session_token)
        .await?
        .into_value();
    assert_eq!(asked.items[0].waits_for, Some(WaitsFor::Question));
    assert_eq!(asked.items[0].work.state, AgentWorkState::WaitingForYou);
    assert!(!asked.truncated);
    Ok(())
}

#[tokio::test]
async fn ai_37_my_agents_lists_a_result_that_waits_for_its_review() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    Box::pin(end_attempt(&world.ctx(), ready(&given.key, given.issue))).await?;
    let found = my_agent_work(&world.ctx(), mine(&given.team.session_token))
        .await?
        .into_value();
    assert_eq!(found.items[0].waits_for, Some(WaitsFor::Review));
    Ok(())
}

#[tokio::test]
async fn ai_37_my_agents_never_lists_the_work_of_another_persons_agent() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    // Someone else who has an agent of their own sees nothing of it.
    let other = an_agent(&world, &token(&world), "Other").await?;
    let _ = other;
    let found = my_agent_work(&world.ctx(), mine(&token(&world)))
        .await?
        .into_value();
    assert!(found.items.is_empty());
    let own = my_agent_work(&world.ctx(), mine(&given.team.session_token))
        .await?
        .into_value();
    assert_eq!(own.items.len(), 1);
    Ok(())
}
