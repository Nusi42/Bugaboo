//! The agent side of agent work and the review (AI-33 to AI-35, ADR-0029).

use bb_core::ErrorCode;
use bb_core::agent_records::GivingState;
use bb_core::agent_work::AgentWorkState;
use bb_core::id::{IssueId, ProjectId};

use super::agent_work::{Ground, Handed, ground, handed, handed_with, plain_hand_over, query};
use super::agents::refused;
use super::scenarios_issues::an_issue;
use super::world::{Outcome, World};
use crate::agent_work::{AgentWorkQuery, hand_over, issue_agent_work};
use crate::auth::AppError;
use crate::issues::issue;

use bb_core::agent_records::ResultKind;
use bb_core::agent_work::{CheckResult, ReviewAnswer};
use bb_core::settings::SettingValue;

use super::scenarios::token as admin_token;
use crate::agent_work::{
    AnswerQuestion, ApprovePlan, AskQuestion, AttachResult, EndAttempt, LookAtWork, ReportAttempt,
    ReviewWork, WorkView, answer_question, approve_plan, ask_question, attach_result, end_attempt,
    look_at_work, report_attempt, review_work,
};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

pub(super) fn report(key: &str, issue: IssueId, plan: Option<&str>, minutes: i64) -> ReportAttempt {
    ReportAttempt {
        session_token: key.to_owned(),
        issue: issue.to_string(),
        plan: plan.map(str::to_owned),
        minutes,
        cost: 120,
    }
}

pub(super) fn ready(key: &str, issue: IssueId) -> EndAttempt {
    EndAttempt {
        session_token: key.to_owned(),
        issue: issue.to_string(),
        gave_up: None,
    }
}

pub(super) fn check(key: &str, issue: IssueId, outcome: CheckResult) -> AttachResult {
    AttachResult {
        session_token: key.to_owned(),
        issue: issue.to_string(),
        kind: ResultKind::Check,
        label: "Unit tests".to_owned(),
        link: None,
        outcome: Some(outcome),
    }
}

pub(super) fn review(session: &str, work: &WorkView, answer: ReviewAnswer) -> ReviewWork {
    ReviewWork {
        session_token: session.to_owned(),
        work: work.id,
        verdict: answer,
        note: None,
        status: None,
        version: work.version,
    }
}

pub(super) async fn show(world: &World, given: &Handed) -> Result<WorkView, AppError> {
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
    .remove(0))
}

#[tokio::test]
async fn ai_34_a_question_waits_for_an_answer_and_the_answer_joins_the_brief() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    let asked = ask_question(
        &world.ctx(),
        AskQuestion {
            session_token: given.key.clone(),
            issue: given.issue.to_string(),
            text: "Which browser?".to_owned(),
        },
    )
    .await?
    .into_value();
    assert_eq!(asked.state, AgentWorkState::WaitingForYou);
    // The agent cannot go on while the question waits.
    let blocked = report_attempt(&world.ctx(), report(&given.key, given.issue, None, 6)).await;
    assert!(refused(&blocked.unwrap_err(), ErrorCode::AgentWorkNotReady));
    let answered = answer_question(
        &world.ctx(),
        AnswerQuestion {
            session_token: given.team.session_token.clone(),
            work: asked.id,
            question: asked.questions[0].id,
            reply: "Firefox".to_owned(),
            version: asked.version,
        },
    )
    .await?
    .into_value();
    assert_eq!(answered.state, AgentWorkState::AgentWorking);
    assert_eq!(
        answered.brief,
        ["Please look at the login button", "Firefox"]
    );
    // Somebody who does not answer for the agent is told there is no such work.
    let stranger = answer_question(
        &world.ctx(),
        AnswerQuestion {
            session_token: admin_token(&world),
            work: asked.id,
            question: asked.questions[0].id,
            reply: "x".to_owned(),
            version: answered.version,
        },
    )
    .await;
    assert!(refused(&stranger.unwrap_err(), ErrorCode::NotFound));
    Ok(())
}

#[tokio::test]
async fn ai_34_with_plan_needs_approval_an_attempt_waits_for_approve_plan() -> Outcome {
    let world = World::new().await?;
    let Ground {
        team,
        agent,
        key,
        issue: issue_id,
        ..
    } = ground(&world).await?;
    let mut asked = plain_hand_over(&team.session_token, agent, &[issue_id]);
    asked.plan_needs_approval = true;
    hand_over(&world.ctx(), asked).await?;
    // Steps without a plan are refused; the plan makes the work wait for a person.
    assert!(
        report_attempt(&world.ctx(), report(&key, issue_id, None, 1))
            .await
            .is_err()
    );
    let plan = report(&key, issue_id, Some("Fix the click"), 2);
    let waiting = report_attempt(&world.ctx(), plan).await?.into_value();
    assert_eq!(waiting.state, AgentWorkState::WaitingForYou);
    assert!(
        report_attempt(&world.ctx(), report(&key, issue_id, None, 3))
            .await
            .is_err()
    );
    let approved = approve_plan(
        &world.ctx(),
        ApprovePlan {
            session_token: team.session_token.clone(),
            work: waiting.id,
            version: waiting.version,
            note: None,
        },
    )
    .await?
    .into_value();
    assert_eq!(approved.state, AgentWorkState::AgentWorking);
    assert!(approved.attempts[0].plan_approved);
    assert!(
        report_attempt(&world.ctx(), report(&key, issue_id, None, 4))
            .await
            .is_ok()
    );
    Ok(())
}

#[tokio::test]
async fn ai_31_a_hand_over_ends_at_accept() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    end_attempt(&world.ctx(), ready(&given.key, given.issue)).await?;
    let work = show(&world, &given).await?;
    assert_eq!(work.state, AgentWorkState::ReadyForReview);
    let accepted = review_work(
        &world.ctx(),
        review(&given.team.session_token, &work, ReviewAnswer::Accept),
    )
    .await?
    .into_value();
    assert!(!accepted.open);
    let ended = world.store.lock().state.work.hand_overs[0].state;
    assert_eq!(ended, GivingState::Ended);
    let refusal = issue(&world.ctx(), query(&given.key, given.issue)).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::NotFound));
    Ok(())
}

#[tokio::test]
async fn ai_35_send_back_starts_another_attempt_with_the_note_in_the_brief() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    end_attempt(&world.ctx(), ready(&given.key, given.issue)).await?;
    let work = show(&world, &given).await?;
    let without_note = review(&given.team.session_token, &work, ReviewAnswer::SendBack);
    assert!(review_work(&world.ctx(), without_note).await.is_err());
    let mut back = review(&given.team.session_token, &work, ReviewAnswer::SendBack);
    back.note = Some("Also check Safari".to_owned());
    let sent = review_work(&world.ctx(), back).await?.into_value();
    assert_eq!(sent.state, AgentWorkState::WaitingForTheAgent);
    assert!(sent.open);
    let again = report_attempt(&world.ctx(), report(&given.key, given.issue, None, 7))
        .await?
        .into_value();
    assert_eq!(again.attempts.len(), 2);
    Ok(())
}

async fn finish_with_check(
    world: &World,
    given: &Handed,
    outcome: CheckResult,
) -> Result<(), AppError> {
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    attach_result(&world.ctx(), check(&given.key, given.issue, outcome)).await?;
    end_attempt(&world.ctx(), ready(&given.key, given.issue)).await?;
    Ok(())
}

async fn allow_by_itself(world: &World, project: ProjectId) -> Outcome {
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: admin_token(world),
            scope: SettingsScope::Project(project),
            key: "agents.accept-by-itself".to_owned(),
            value: SettingValue::Bool(true),
        },
    )
    .await?;
    Ok(())
}

async fn handed_to_accept_by_itself(world: &World) -> Result<Handed, AppError> {
    handed_with(world, |asked| asked.accept_by_itself = true).await
}

#[tokio::test]
async fn ai_35_accept_by_itself_is_off_at_the_start() -> Outcome {
    let world = World::new().await?;
    let given = handed_to_accept_by_itself(&world).await?;
    finish_with_check(&world, &given, CheckResult::Passed).await?;
    let work = show(&world, &given).await?;
    // The hand-over allows it and the check passed, but the project never switched it on.
    assert_eq!(work.state, AgentWorkState::ReadyForReview);
    assert!(!work.accepted_by_itself);
    assert!(work.open);
    Ok(())
}

#[tokio::test]
async fn ai_35_an_attempt_without_a_check_or_with_a_failed_one_waits_for_a_person() -> Outcome {
    let world = World::new().await?;
    let given = handed_to_accept_by_itself(&world).await?;
    allow_by_itself(&world, given.project).await?;
    finish_with_check(&world, &given, CheckResult::Failed).await?;
    let work = show(&world, &given).await?;
    assert!(work.open && !work.accepted_by_itself);
    // No check at all: the same.
    let second = an_issue(&world, given.project).await?;
    let mut again = plain_hand_over(&given.team.session_token, given.agent, &[second]);
    again.accept_by_itself = true;
    hand_over(&world.ctx(), again).await?;
    report_attempt(&world.ctx(), report(&given.key, second, None, 1)).await?;
    let ended = end_attempt(&world.ctx(), ready(&given.key, second))
        .await?
        .into_value();
    assert!(ended.open && !ended.accepted_by_itself);
    Ok(())
}

#[tokio::test]
async fn ai_35_a_passed_check_is_accepted_by_itself_when_the_project_allows_it() -> Outcome {
    let world = World::new().await?;
    let given = handed_to_accept_by_itself(&world).await?;
    allow_by_itself(&world, given.project).await?;
    finish_with_check(&world, &given, CheckResult::Passed).await?;
    let work = show(&world, &given).await?;
    assert!(work.accepted_by_itself && !work.open);
    // It waits under *Accepted by itself* until a person looked at it.
    let looked = look_at_work(
        &world.ctx(),
        LookAtWork {
            session_token: given.team.session_token.clone(),
            work: work.id,
        },
    )
    .await?
    .into_value();
    assert!(!looked.accepted_by_itself);
    Ok(())
}

#[tokio::test]
async fn ai_33_an_agent_never_makes_a_time_entry() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 90)).await?;
    // What the agent reports stays on its attempt, marked as its own.
    let work = show(&world, &given).await?;
    assert_eq!(work.attempts[0].reported_minutes, 90);
    assert!(world.store.lock().state.time_entries.is_empty());
    // And it cannot book time itself.
    let booked = crate::time::add_time(
        &world.ctx(),
        crate::time::AddTime {
            session_token: given.key.clone(),
            issue: given.issue.to_string(),
            date: jiff::civil::date(2027, 1, 15),
            minutes: bb_core::time::Minutes::new(30),
            details: crate::time::EntryDetails::default(),
        },
    )
    .await;
    assert!(booked.is_err());
    assert!(world.store.lock().state.time_entries.is_empty());
    Ok(())
}

#[tokio::test]
async fn ai_34_the_attempts_are_limited_by_the_budget() -> Outcome {
    let world = World::new().await?;
    let Ground {
        team,
        agent,
        key,
        issue: issue_id,
        ..
    } = ground(&world).await?;
    let mut asked = plain_hand_over(&team.session_token, agent, &[issue_id]);
    asked.budget_attempts = Some(1);
    hand_over(&world.ctx(), asked).await?;
    report_attempt(&world.ctx(), report(&key, issue_id, None, 1)).await?;
    end_attempt(&world.ctx(), ready(&key, issue_id)).await?;
    let work = issue_agent_work(
        &world.ctx(),
        AgentWorkQuery {
            session_token: team.session_token.clone(),
            issue: issue_id.to_string(),
        },
    )
    .await?
    .into_value()
    .work
    .remove(0);
    let mut back = review(&team.session_token, &work, ReviewAnswer::SendBack);
    back.note = Some("Again".to_owned());
    review_work(&world.ctx(), back).await?;
    let refusal = report_attempt(&world.ctx(), report(&key, issue_id, None, 1)).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::AgentWorkBudget));
    Ok(())
}
