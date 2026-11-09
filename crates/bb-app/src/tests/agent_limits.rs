//! The limits of an agent (AI-37, ADR-0029, decision 9), at the edge and in parallel.

use bb_core::ErrorCode;
use bb_core::agent::{AgentState, LimitKind};
use bb_core::agent_work::{AgentWorkState, HandOverPower};
use bb_core::id::IssueId;
use serde_json::json;

use super::agent_tools::{previewed, used};
use super::agent_tools_more::version_of;
use super::agent_work::{Handed, handed, handed_with, plain_hand_over, query, start_attempt};
use super::agents::refused;
use super::scenarios_issues::an_issue;
use super::world::{Outcome, World};
use crate::agent_work::hand_over;
use crate::auth::AppError;
use crate::issues::{ChangeIssue, change_issue, issue};

/// An agent handed two issues, and allowed to work on one at a time.
async fn two_issues_and_a_limit_of_one(world: &World) -> Result<(Handed, IssueId), AppError> {
    let first = handed(world).await?;
    let second = an_issue(world, first.project)
        .await
        .map_err(AppError::Internal)?;
    hand_over(
        &world.ctx(),
        plain_hand_over(&first.team.session_token, first.agent, &[second]),
    )
    .await?;
    limit(world, LimitKind::IssuesAtOnce, 1);
    Ok((first, second))
}

/// Lowers a limit of the agent in the store, as its person would on its page.
pub(super) fn limit(world: &World, kind: LimitKind, value: i64) {
    let mut shared = world.store.lock();
    for agent in &mut shared.state.agents {
        match kind {
            LimitKind::ChangesAMinute => agent.changes_a_minute = value,
            LimitKind::IssuesAtOnce => agent.issues_at_once = value,
            LimitKind::AttemptsOnOneIssue => agent.attempts_on_one_issue = value,
        }
    }
}

pub(super) fn the_agent(world: &World) -> bb_core::agent::Agent {
    world.store.lock().state.agents[0].clone()
}

#[tokio::test]
async fn ai_37_reaching_a_limit_pauses_the_agent_with_a_sentence_and_no_change_halfway() -> Outcome
{
    let world = World::new().await?;
    let given = handed(&world).await?;
    limit(&world, LimitKind::ChangesAMinute, 3);
    let issue_text = given.issue.to_string();
    // The first three changes are within the limit, at the edge of it.
    used(&world, &given.key, "take", json!({"issue": issue_text})).await?;
    for minutes in [1, 2] {
        used(
            &world,
            &given.key,
            "report",
            json!({"issue": issue_text, "minutes": minutes, "cost_cents": 0}),
        )
        .await?;
    }
    assert_eq!(the_agent(&world).state, AgentState::Active);
    // Reads count nothing.
    for _ in 0..5 {
        used(&world, &given.key, "changes_since", json!({})).await?;
        previewed(
            &world,
            &given.key,
            "report",
            json!({"issue": issue_text, "minutes": 9, "cost_cents": 0}),
        )
        .await?;
    }
    assert_eq!(the_agent(&world).state, AgentState::Active);
    // The fourth does not start: nothing of it is written, and the agent is paused.
    let events = world.store.lock().events.len();
    let attempts = format!("{:?}", world.store.lock().state.work.attempts);
    let over = used(
        &world,
        &given.key,
        "report",
        json!({"issue": issue_text, "minutes": 3, "cost_cents": 0}),
    )
    .await;
    assert!(refused(&over.unwrap_err(), ErrorCode::AgentLimitReached));
    let agent = the_agent(&world);
    assert_eq!(agent.state, AgentState::Paused);
    assert_eq!(agent.paused_for, Some(LimitKind::ChangesAMinute));
    // Only the pause itself was written.
    assert_eq!(world.store.lock().events.len(), events + 1);
    assert_eq!(
        attempts,
        format!("{:?}", world.store.lock().state.work.attempts)
    );
    // The sentence names the agent and the limit.
    assert_eq!(
        LimitKind::ChangesAMinute
            .sentence(&agent.name, agent.changes_a_minute)
            .render(),
        "Fixer paused: 3 changes in a minute"
    );
    // A paused agent is refused at its next request, whatever it asks.
    let after = used(&world, &given.key, "changes_since", json!({})).await;
    assert!(refused(&after.unwrap_err(), ErrorCode::NotSignedIn));
    // Its person is told.
    world.ctx().catch_up_notifications().await?;
    let told = world
        .store
        .lock()
        .state
        .notifications
        .iter()
        .filter(|row| {
            row.recipient == given.team.person.id
                && row.kind == bb_core::notification::NotificationKind::AgentStopped
        })
        .count();
    assert_eq!(told, 1, "the person was not told");
    // Resuming starts the agent from nothing.
    crate::agents::change_agent(
        &world.ctx(),
        crate::agents::ChangeAgent {
            session_token: given.team.session_token.clone(),
            id: agent.id,
            action: crate::agents::AgentAction::Resume,
            version: agent.version,
        },
    )
    .await?;
    assert_eq!(the_agent(&world).paused_for, None);
    used(
        &world,
        &given.key,
        "report",
        json!({"issue": issue_text, "minutes": 4, "cost_cents": 0}),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn ai_37_an_agent_works_on_as_many_issues_at_once_as_it_may_and_no_more() -> Outcome {
    let world = World::new().await?;
    let (first, second) = two_issues_and_a_limit_of_one(&world).await?;
    used(
        &world,
        &first.key,
        "take",
        json!({"issue": first.issue.to_string()}),
    )
    .await?;
    let over = used(
        &world,
        &first.key,
        "take",
        json!({"issue": second.to_string()}),
    )
    .await;
    assert!(refused(&over.unwrap_err(), ErrorCode::AgentLimitReached));
    let agent = the_agent(&world);
    assert_eq!(agent.state, AgentState::Paused);
    assert_eq!(agent.paused_for, Some(LimitKind::IssuesAtOnce));
    // The work on the second issue did not start.
    let started = world
        .store
        .lock()
        .state
        .work
        .works
        .iter()
        .filter(|work| work.issue == second && work.state != AgentWorkState::WaitingForTheAgent)
        .count();
    assert_eq!(started, 0);
    Ok(())
}

#[tokio::test]
async fn ai_37_an_agent_makes_as_many_attempts_on_an_issue_as_it_may_and_no_more() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    limit(&world, LimitKind::AttemptsOnOneIssue, 1);
    let address = given.issue.to_string();
    used(&world, &given.key, "take", json!({"issue": address})).await?;
    used(&world, &given.key, "submit", json!({"issue": address})).await?;
    // The person sends it back; a second attempt would pass the limit.
    let work = super::agent_work_review::show(&world, &given).await?;
    crate::agent_work::review_work(
        &world.ctx(),
        crate::agent_work::ReviewWork {
            note: Some("Try the other way".to_owned()),
            ..super::agent_work_review::review(
                &given.team.session_token,
                &work,
                bb_core::agent_work::ReviewAnswer::SendBack,
            )
        },
    )
    .await?;
    let over = used(&world, &given.key, "take", json!({"issue": address})).await;
    assert!(refused(&over.unwrap_err(), ErrorCode::AgentLimitReached));
    assert_eq!(
        the_agent(&world).paused_for,
        Some(LimitKind::AttemptsOnOneIssue)
    );
    Ok(())
}

#[tokio::test]
async fn ai_37_a_preview_that_finds_a_limit_pauses_nobody() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    limit(&world, LimitKind::IssuesAtOnce, 0);
    let over = previewed(
        &world,
        &given.key,
        "take",
        json!({"issue": given.issue.to_string()}),
    )
    .await;
    assert!(refused(&over.unwrap_err(), ErrorCode::AgentLimitReached));
    assert_eq!(the_agent(&world).state, AgentState::Active);
    Ok(())
}

#[tokio::test]
async fn ai_37_the_changes_of_an_agent_over_the_api_count_against_its_limits() -> Outcome {
    let world = World::new().await?;
    let given = handed_with(&world, |asked| {
        asked.powers = vec![HandOverPower::ChangeFields];
        asked.fields = vec!["title".to_owned()];
    })
    .await?;
    start_attempt(&world, &given.key, given.issue).await?;
    limit(&world, LimitKind::ChangesAMinute, 1);
    let mut counts = Vec::new();
    for title in ["One", "Two"] {
        let version = version_of(&world, &given).await?;
        // The use cases of the API count a change when it is made through `AuthApi`: the table of
        // the use cases says which inputs change something.
        let ctx = world.ctx();
        ctx.expect_change();
        counts.push(
            change_issue(
                &ctx,
                ChangeIssue {
                    session_token: given.key.clone(),
                    issue: given.issue.to_string(),
                    version,
                    issue_type: None,
                    title: Some(title.to_owned()),
                    status: None,
                    priority: None,
                    estimated_time: None,
                    time_left: None,
                    assigned_to: None,
                    fields: std::collections::BTreeMap::default(),
                },
            )
            .await,
        );
        // `AuthApi` settles a request that was made; here it is done by hand.
        ctx.settle_agent_change(counts.last().is_some_and(Result::is_ok));
    }
    assert!(counts[0].is_ok());
    assert!(refused(
        counts[1].as_ref().unwrap_err(),
        ErrorCode::AgentLimitReached
    ));
    assert_eq!(the_agent(&world).state, AgentState::Paused);
    Ok(())
}

#[tokio::test]
async fn ai_37_a_change_that_is_refused_counts_nothing() -> Outcome {
    let world = World::new().await?;
    // Nothing ticked: every change of a field is refused.
    let given = handed(&world).await?;
    limit(&world, LimitKind::ChangesAMinute, 1);
    for title in ["One", "Two", "Three"] {
        let version = version_of(&world, &given).await?;
        let ctx = world.ctx();
        ctx.expect_change();
        let refusal = change_issue(
            &ctx,
            ChangeIssue {
                session_token: given.key.clone(),
                issue: given.issue.to_string(),
                version,
                issue_type: None,
                title: Some(title.to_owned()),
                status: None,
                priority: None,
                estimated_time: None,
                time_left: None,
                assigned_to: None,
                fields: std::collections::BTreeMap::default(),
            },
        )
        .await;
        // `AuthApi` settles a request that was refused; here it is done by hand.
        ctx.settle_agent_change(refusal.is_ok());
        let refusal = refusal.unwrap_err();
        assert!(refused(&refusal, ErrorCode::Forbidden), "{refusal:?}");
    }
    assert_eq!(the_agent(&world).state, AgentState::Active);
    Ok(())
}

#[tokio::test]
async fn ai_37_an_agent_cannot_run_a_second_issue_by_reporting_on_it() -> Outcome {
    let world = World::new().await?;
    let (first, second) = two_issues_and_a_limit_of_one(&world).await?;
    used(
        &world,
        &first.key,
        "take",
        json!({"issue": first.issue.to_string()}),
    )
    .await?;
    let sneaked = used(
        &world,
        &first.key,
        "report",
        json!({"issue": second.to_string(), "minutes": 1, "cost_cents": 0}),
    )
    .await;
    assert!(refused(&sneaked.unwrap_err(), ErrorCode::AgentWorkNotReady));
    assert_eq!(the_agent(&world).state, AgentState::Active);
    let running = world
        .store
        .lock()
        .state
        .work
        .works
        .iter()
        .filter(|work| work.state.in_attempt())
        .count();
    assert_eq!(running, 1);
    Ok(())
}

#[tokio::test]
async fn ai_37_two_takes_at_the_same_moment_pass_a_limit_of_one_issue_once() -> Outcome {
    let world = World::new().await?;
    let (first, second) = two_issues_and_a_limit_of_one(&world).await?;
    let (one, other) = tokio::join!(
        used(
            &world,
            &first.key,
            "take",
            json!({"issue": first.issue.to_string()})
        ),
        used(
            &world,
            &first.key,
            "take",
            json!({"issue": second.to_string()})
        ),
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(other.is_ok()), 1);
    let running = world
        .store
        .lock()
        .state
        .work
        .works
        .iter()
        .filter(|work| work.state.in_attempt())
        .count();
    assert_eq!(running, 1);
    // The run of a take that lost does not stay running.
    let open_runs = world
        .store
        .lock()
        .state
        .many_changes_runs
        .iter()
        .filter(|run| {
            matches!(
                run.status,
                bb_core::many_changes::ManyChangeStatus::Queued
                    | bb_core::many_changes::ManyChangeStatus::Running
            )
        })
        .count();
    assert_eq!(open_runs, 1, "only the run of the attempt that started");
    Ok(())
}

#[tokio::test]
async fn ai_37_parallel_changes_cannot_pass_the_limit_of_a_minute() -> Outcome {
    let world = World::new().await?;
    let given = handed_with(&world, |asked| {
        asked.powers = vec![HandOverPower::ChangeFields];
        asked.fields = vec!["title".to_owned()];
    })
    .await?;
    limit(&world, LimitKind::ChangesAMinute, 2);
    // Five requests with a change are checked one after the other and none of them has made its
    // change yet (nothing is settled): each takes its place in the minute when it is checked, so
    // only two get in.
    let mut ctxs = Vec::new();
    let mut answers = Vec::new();
    for _ in 0..5 {
        let ctx = world.ctx();
        ctx.expect_change();
        answers.push(issue(&ctx, query(&given.key, given.issue)).await);
        ctxs.push(ctx);
    }
    // Every call is counted when it is checked: at most two got in, whatever else they met.
    let counted = answers.iter().filter(|made| made.is_ok()).count();
    assert!(counted <= 2, "{counted} passed a limit of 2");
    assert_eq!(the_agent(&world).state, AgentState::Paused);
    Ok(())
}
