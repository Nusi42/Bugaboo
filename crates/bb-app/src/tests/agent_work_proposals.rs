//! The proposals of standing rules: which fields a rule proposes, the decision on a proposal that
//! cannot be carried out, and opening a proposal again (AI-32, ADR-0029).

use bb_core::ErrorCode;
use bb_core::agent_records::{GivingState, ProposalState};
use bb_core::agent_work::RuleField;
use bb_policy::Authorized;
use serde_json::json;

use super::agent_work_rules::{RuleWorld, found_by_search, rule_request, rule_world};
use super::agents::refused;
use super::world::{Outcome, World};
use crate::agent_work::{
    DecideProposal, ProposeChange, create_standing_rule, decide_proposal, propose_change,
};
use crate::auth::AppError;

/// A rule that works with these fields, the issue it found, and the proposal the agent made.
async fn proposed(
    world: &World,
    given: &RuleWorld,
    fields: Vec<RuleField>,
    field: RuleField,
    after: serde_json::Value,
) -> Result<Result<crate::agent_work::ProposalView, AppError>, AppError> {
    let rule = create_standing_rule(&world.ctx(), rule_request(given, fields))
        .await?
        .into_value();
    found_by_search(world);
    world
        .ctx()
        .run_rule_matches(&json!({ "project": given.project.to_string() }))
        .await?;
    Ok(propose_change(
        &world.ctx(),
        ProposeChange {
            session_token: given.key.clone(),
            rule: rule.id,
            issue: given.issue.to_string(),
            field,
            after,
        },
    )
    .await
    .map(Authorized::into_value))
}

#[tokio::test]
async fn ai_32_a_rule_proposes_only_the_fields_it_names() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let refusal = proposed(
        &world,
        &given,
        vec![RuleField::Labels],
        RuleField::Type,
        json!("x"),
    )
    .await?;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    // The field it names is proposed; a rule that names none proposes any field of the list.
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    proposed(
        &world,
        &given,
        vec![RuleField::Labels],
        RuleField::Labels,
        json!("x"),
    )
    .await??;
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    proposed(&world, &given, Vec::new(), RuleField::Type, json!("x")).await??;
    Ok(())
}

#[tokio::test]
async fn ai_32_a_proposal_that_cannot_be_carried_out_is_open_again() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    // A type that is no ID: the change fails after the proposal was claimed.
    let proposal = proposed(
        &world,
        &given,
        Vec::new(),
        RuleField::Type,
        json!("nonsense"),
    )
    .await??;
    let decided = decide_proposal(
        &world.ctx(),
        DecideProposal {
            session_token: given.team.session_token.clone(),
            proposal: proposal.id,
            apply: true,
        },
    )
    .await;
    assert!(refused(&decided.unwrap_err(), ErrorCode::InvalidRequest));
    let stored = world.store.lock().state.work.proposals.clone();
    assert_eq!(stored[0].state, ProposalState::Open);
    assert!(stored[0].decided_by.is_none());
    // It can be discarded after: it was not left claimed.
    let discarded = decide_proposal(
        &world.ctx(),
        DecideProposal {
            session_token: given.team.session_token.clone(),
            proposal: proposal.id,
            apply: false,
        },
    )
    .await?
    .into_value();
    assert_eq!(discarded.state, ProposalState::Discarded);
    Ok(())
}

#[tokio::test]
async fn ai_32_a_proposal_that_asks_for_something_not_applied_yet_is_open_again() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    // *Duplicate of* is not carried out yet: the decision to apply it fails after the claim.
    let proposal = proposed(
        &world,
        &given,
        Vec::new(),
        RuleField::DuplicateOf,
        json!("x"),
    )
    .await??;
    let decide = |apply| DecideProposal {
        session_token: given.team.session_token.clone(),
        proposal: proposal.id,
        apply,
    };
    let applied = decide_proposal(&world.ctx(), decide(true)).await;
    assert!(refused(&applied.unwrap_err(), ErrorCode::InvalidRequest));
    assert_eq!(
        world.store.lock().state.work.proposals[0].state,
        ProposalState::Open
    );
    let events_before = world.store.lock().events.len();
    let discarded = decide_proposal(&world.ctx(), decide(false)).await?;
    assert_eq!(discarded.into_value().state, ProposalState::Discarded);
    assert!(world.store.lock().events.len() > events_before);
    Ok(())
}

#[tokio::test]
async fn ai_31_ending_a_hand_over_that_keeps_changing_fails_for_the_job_to_run_again() -> Outcome {
    let world = World::new().await?;
    let given = super::agent_work::handed(&world).await?;
    world.clock.advance(15 * 24 * 3600);
    world.store.lock().state.hand_overs_cannot_end = true;
    let id = json!({ "hand_over": given.hand_over.id.to_string() });
    let failed = world.ctx().run_hand_over_runs_out(&id).await;
    assert!(matches!(failed, Err(AppError::Internal(_))), "{failed:?}");
    assert_eq!(
        world.store.lock().state.work.hand_overs[0].state,
        GivingState::Open
    );
    // The job runs again when the hand-over holds still, and ends it.
    world.store.lock().state.hand_overs_cannot_end = false;
    world.ctx().run_hand_over_runs_out(&id).await?;
    assert_eq!(
        world.store.lock().state.work.hand_overs[0].state,
        GivingState::Ended
    );
    Ok(())
}

#[tokio::test]
async fn ai_32_opening_a_proposal_again_that_nobody_claimed_writes_no_event() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    proposed(&world, &given, Vec::new(), RuleField::Type, json!("x")).await??;
    let (rule, proposal) = {
        let shared = world.store.lock();
        (
            shared.state.work.rules[0].clone(),
            shared.state.work.proposals[0].clone(),
        )
    };
    let before = world.store.lock().events.len();
    // The proposal is open: not claimed by the person, so nothing is opened and nothing is told.
    world
        .ctx()
        .reopen_proposal(&rule, &proposal, given.team.person.id)
        .await?;
    assert_eq!(world.store.lock().events.len(), before);
    assert_eq!(
        world.store.lock().state.work.proposals[0].state,
        ProposalState::Open
    );
    Ok(())
}

#[tokio::test]
async fn ai_32_a_proposal_on_the_issue_names_its_agent_without_any_hand_over() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let made = proposed(&world, &given, Vec::new(), RuleField::Labels, json!("x")).await??;
    assert_eq!(made.agent_name, "Fixer");
    let on_the_issue = crate::agent_work::issue_agent_work(
        &world.ctx(),
        crate::agent_work::AgentWorkQuery {
            session_token: given.team.session_token.clone(),
            issue: given.issue.to_string(),
        },
    )
    .await?
    .into_value();
    assert!(on_the_issue.work.is_empty(), "a rule hands nothing over");
    assert_eq!(on_the_issue.proposals.len(), 1);
    assert_eq!(on_the_issue.proposals[0].agent_name, "Fixer");
    Ok(())
}
