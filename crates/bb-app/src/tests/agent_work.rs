//! Hand-overs and agent work (AI-31 to AI-35, ADR-0029).

use bb_core::ErrorCode;
use bb_core::agent_records::GivingState;
use bb_core::agent_work::{AgentWorkState, BriefPart};
use bb_core::id::{AgentId, HandOverId, IssueId, ProjectId};
use bb_policy::StartingRole;
use serde_json::json;

use super::agents::{an_agent, key_of, refused};
use super::scenarios::{a_project, token};
use super::scenarios_issues::an_issue;
use super::workflows_rules::{assign, member, starting_role};
use super::world::{Outcome, World};
use crate::agent_work::{AgentWorkQuery, HandOver, HandOverView, hand_over, issue_agent_work};
use crate::auth::{AppError, Started};
use crate::issues::{IssueQuery, issue};

/// The request that hands the issues over with nothing more than the defaults.
pub(super) fn plain_hand_over(session: &str, agent: AgentId, issues: &[IssueId]) -> HandOver {
    HandOver {
        session_token: session.to_owned(),
        agent,
        issues: issues.iter().map(ToString::to_string).collect(),
        note: "Please look at the login button".to_owned(),
        powers: Vec::new(),
        fields: Vec::new(),
        budget_attempts: None,
        budget_minutes: None,
        budget_cost: None,
        plan_needs_approval: false,
        accept_by_itself: false,
        days: None,
    }
}

/// A project, a team member of it, their agent with its key, one issue, and the agent handed it.
pub(super) struct Handed {
    pub(super) project: ProjectId,
    pub(super) team: Started,
    pub(super) agent: AgentId,
    pub(super) key: String,
    pub(super) issue: IssueId,
    pub(super) hand_over: HandOverView,
}

pub(super) async fn handed(world: &World) -> Result<Handed, AppError> {
    let project = a_project(world, "WEB").await.map_err(AppError::Internal)?;
    let team = member(
        world,
        "team@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    let agent = an_agent(world, &team.session_token, "Fixer").await?;
    let key = key_of(world, &team.session_token, agent).await?;
    let issue = an_issue(world, project).await.map_err(AppError::Internal)?;
    let hand_over = hand_over(
        &world.ctx(),
        plain_hand_over(&team.session_token, agent, &[issue]),
    )
    .await?
    .into_value();
    Ok(Handed {
        project,
        team,
        agent,
        key,
        issue,
        hand_over,
    })
}

pub(super) fn query(session: &str, issue: IssueId) -> IssueQuery {
    IssueQuery {
        session_token: session.to_owned(),
        issue: issue.to_string(),
    }
}

#[tokio::test]
async fn ai_31_a_handed_over_agent_sees_only_its_issues() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let other = an_issue(&world, given.project).await?;
    // The person sees both issues; the agent only the one it was given.
    assert!(
        issue(&world.ctx(), query(&given.team.session_token, other))
            .await
            .is_ok()
    );
    assert!(
        issue(&world.ctx(), query(&given.key, given.issue))
            .await
            .is_ok()
    );
    let refusal = issue(&world.ctx(), query(&given.key, other)).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::NotFound));
    Ok(())
}

#[tokio::test]
async fn ai_31_a_hand_over_needs_change_issues_on_each_issue() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    // A project where the person is only a Viewer: they see the issue, but may not hand it over.
    let quiet = a_project(&world, "QUIET").await?;
    let viewer_issue = an_issue(&world, quiet).await?;
    let viewer = &given.team;
    let role = starting_role(&world, StartingRole::Viewer).await?;
    assign(&world, viewer, role, quiet).await?;
    let refusal = hand_over(
        &world.ctx(),
        plain_hand_over(&viewer.session_token, given.agent, &[viewer_issue]),
    )
    .await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    // One issue that cannot be handed over stops the whole hand-over.
    let fresh = an_issue(&world, given.project).await?;
    let both = hand_over(
        &world.ctx(),
        plain_hand_over(
            &given.team.session_token,
            given.agent,
            &[fresh, viewer_issue],
        ),
    )
    .await;
    assert!(both.is_err());
    let work = issue_agent_work(
        &world.ctx(),
        AgentWorkQuery {
            session_token: given.team.session_token.clone(),
            issue: fresh.to_string(),
        },
    )
    .await?
    .into_value();
    assert!(work.work.is_empty(), "nothing of a refused hand-over stays");
    Ok(())
}

#[tokio::test]
async fn ai_31_the_brief_names_what_is_missing_and_does_not_stop_the_hand_over() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    // The test issue has no description, and no steps or *Done when*.
    let missing = &given.hand_over.issues[0].missing;
    assert_eq!(
        missing,
        &[
            BriefPart::Description,
            BriefPart::StepsToReproduce,
            BriefPart::DoneWhen
        ]
    );
    assert_eq!(given.hand_over.state, GivingState::Open);
    Ok(())
}

#[tokio::test]
async fn ai_31_an_agent_has_one_open_hand_over_for_an_issue() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let again = hand_over(
        &world.ctx(),
        plain_hand_over(&given.team.session_token, given.agent, &[given.issue]),
    )
    .await;
    assert!(refused(&again.unwrap_err(), ErrorCode::AgentWorkOpen));
    Ok(())
}

#[tokio::test]
async fn ai_33_the_issue_keeps_its_person_and_shows_worked_on_by() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let before = issue(&world.ctx(), query(&token(&world), given.issue))
        .await?
        .into_value();
    let work = issue_agent_work(
        &world.ctx(),
        AgentWorkQuery {
            session_token: given.team.session_token.clone(),
            issue: given.issue.to_string(),
        },
    )
    .await?
    .into_value();
    assert_eq!(work.worked_on_by.len(), 1);
    assert_eq!(work.worked_on_by[0].name, "Fixer");
    assert_eq!(work.work[0].state, AgentWorkState::WaitingForTheAgent);
    assert_eq!(work.work[0].brief, ["Please look at the login button"]);
    // The agent is never *Assigned to*: the issue is as it was.
    let after = issue(&world.ctx(), query(&token(&world), given.issue))
        .await?
        .into_value();
    assert_eq!(before.issue.assigned_to, after.issue.assigned_to);
    assert_eq!(before.issue.version, after.issue.version);
    Ok(())
}

#[tokio::test]
async fn ai_31_a_hand_over_runs_out_after_14_days() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let id: HandOverId = given.hand_over.id;
    // Not yet: the job of a hand-over that has not run out leaves it as it is.
    world.clock.advance(13 * 24 * 3600);
    world
        .ctx()
        .run_hand_over_runs_out(&json!({ "hand_over": id.to_string() }))
        .await?;
    let work = |world: &World| {
        let state = world.store.lock().state.work.clone();
        (state.hand_overs[0].state, state.works[0].state)
    };
    assert_eq!(
        work(&world),
        (GivingState::Open, AgentWorkState::WaitingForTheAgent)
    );
    world.clock.advance(2 * 24 * 3600);
    world
        .ctx()
        .run_hand_over_runs_out(&json!({ "hand_over": id.to_string() }))
        .await?;
    assert_eq!(work(&world), (GivingState::Ended, AgentWorkState::Stopped));
    // The agent has lost the issue with its hand-over.
    let refusal = issue(&world.ctx(), query(&given.key, given.issue)).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::NotFound));
    Ok(())
}

#[tokio::test]
async fn ai_33_open_work_becomes_stopped_when_the_person_loses_access() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    world.ctx().stop_work_of_agent(given.agent).await?;
    let stored = world.store.lock().state.work.clone();
    assert_eq!(stored.works[0].state, AgentWorkState::Stopped);
    assert_eq!(stored.hand_overs[0].state, GivingState::Ended);
    Ok(())
}

#[tokio::test]
async fn ai_33_open_work_becomes_stopped_when_the_agent_is_stopped() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    crate::agents::stop_all_agents(
        &world.ctx(),
        crate::agents::StopAllAgents {
            session_token: given.team.session_token.clone(),
        },
    )
    .await?;
    let stored = world.store.lock().state.work.clone();
    assert_eq!(stored.works[0].state, AgentWorkState::Stopped);
    assert_eq!(stored.hand_overs[0].state, GivingState::Ended);
    Ok(())
}
