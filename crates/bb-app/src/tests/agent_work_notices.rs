//! What people hear of agent work (ADR-0033, amendment M7): the six kinds and who hears them.

use bb_core::id::UserId;
use bb_core::notification::NotificationKind;

use super::agent_work::handed;
use super::agent_work::report_attempt;
use super::agent_work_review::{ready, report};
use super::agent_work_rules::{made, payload_of, rule_world};
use super::world::{Outcome, World};
use crate::agent_work::{AskQuestion, EndAttempt, ask_question, end_attempt};

/// The kinds the notifications of a person have.
fn kinds_for(world: &World, person: UserId) -> Vec<NotificationKind> {
    world
        .store
        .lock()
        .state
        .notifications
        .iter()
        .filter(|row| row.recipient == person)
        .map(|row| row.kind)
        .collect()
}

/// The kinds of agent work among the notifications of a person.
fn agent_work_kinds(world: &World, person: UserId) -> Vec<NotificationKind> {
    kinds_for(world, person)
        .into_iter()
        .filter(|kind| *kind != NotificationKind::FollowedActivity)
        .collect()
}

#[tokio::test]
async fn ai_34_a_question_tells_the_person_who_answers_for_the_agent() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
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
    world.ctx().catch_up_notifications().await?;
    assert_eq!(
        kinds_for(&world, given.team.person.id),
        [NotificationKind::AgentAsks]
    );
    Ok(())
}

#[tokio::test]
async fn ai_35_a_result_that_is_ready_and_an_agent_that_gave_up_are_told() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    end_attempt(&world.ctx(), ready(&given.key, given.issue)).await?;
    world.ctx().catch_up_notifications().await?;
    // The comment of the agent is activity on an issue the person follows.
    assert_eq!(
        agent_work_kinds(&world, given.team.person.id),
        [NotificationKind::ReadyForReview]
    );
    // Another hand-over: the agent gives up.
    let second = handed_again(&world, &given).await?;
    report_attempt(&world.ctx(), report(&given.key, second, None, 5)).await?;
    end_attempt(
        &world.ctx(),
        EndAttempt {
            gave_up: Some("Cannot reproduce it".to_owned()),
            ..ready(&given.key, second)
        },
    )
    .await?;
    world.ctx().catch_up_notifications().await?;
    assert_eq!(
        agent_work_kinds(&world, given.team.person.id),
        [
            NotificationKind::ReadyForReview,
            NotificationKind::AgentGaveUp
        ]
    );
    Ok(())
}

async fn handed_again(
    world: &World,
    given: &super::agent_work::Handed,
) -> Result<bb_core::id::IssueId, crate::auth::AppError> {
    let issue = super::scenarios_issues::an_issue(world, given.project)
        .await
        .map_err(crate::auth::AppError::Internal)?;
    crate::agent_work::hand_over(
        &world.ctx(),
        super::agent_work::plain_hand_over(&given.team.session_token, given.agent, &[issue]),
    )
    .await?;
    Ok(issue)
}

#[tokio::test]
async fn ai_30_a_stopped_agent_and_a_rule_that_runs_out_soon_tell_the_person() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let rule = made(&world, &given).await?;
    world
        .ctx()
        .run_rule_about_to_run_out(&payload_of(&rule))
        .await?;
    world.ctx().stop_work_of_agent(given.agent).await?;
    world.ctx().catch_up_notifications().await?;
    let mut kinds = kinds_for(&world, given.team.person.id);
    kinds.sort_by_key(|kind| kind.code());
    assert_eq!(
        kinds,
        [
            NotificationKind::AgentStopped,
            NotificationKind::RuleRunsOutSoon
        ]
    );
    Ok(())
}
