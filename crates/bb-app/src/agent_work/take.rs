//! Taking a handed-over issue (AI-36, ADR-0029, decision 9): all or nothing, and the only way an
//! attempt starts. The agent takes the work it was handed with the version of the issue it read; an
//! attempt starts, with its run, and the work is *Agent working*.
//!
//! Of two agents who take the same issue at the same moment one gets it, and an agent never works on
//! more issues than its limit says: both are judged again inside the change that starts the attempt,
//! under the lock of the issue and then the lock of the agent, on what is stored then. The limits of
//! the agent that belong to starting an attempt (issues at once, attempts on one issue) are judged
//! here and nowhere else.

use bb_core::ErrorCode;
use bb_core::agent::LimitKind;
use bb_core::agent_records::Attempt;
use bb_core::agent_work::AgentWorkState;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::AttemptId;
use bb_core::run::RunEnd;
use bb_policy::{Authorized, Place};
use serde_json::json;

use super::attempt::Mine;
use super::members::member;
use super::plan::{Plan, state_change};
use super::types::{TakeWork, WorkView};
use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, entity, internal, set_to};
use crate::projects::Why;

/// What taking an issue would do: everything judged before anything is written.
pub(super) struct TakePlan {
    mine: Mine,
    agent: bb_core::agent::Agent,
    attempt: Attempt,
}

impl TakePlan {
    /// The fields taking changes, as a preview lists them.
    pub(super) fn changes(&self) -> Vec<FieldChange> {
        vec![
            state_change(self.mine.work.state, AgentWorkState::AgentWorking),
            set_to(member::ATTEMPT, json!(self.attempt.number)),
        ]
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Judges the taking of an issue.
    async fn plan_take(&self, input: &TakeWork, mode: Plan) -> Result<TakePlan, AppError> {
        let mine = self.my_work(&input.session_token, &input.issue).await?;
        if mine.work.state != AgentWorkState::WaitingForTheAgent {
            return Err(ErrorCode::AgentWorkNotReady.into());
        }
        let issue = self
            .store()
            .issue(mine.work.issue)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        if input.version.is_some_and(|read| read != issue.version) {
            return Err(AppError::stale(
                entity(RecordKind::Issue, issue.id),
                Place::Project(issue.project),
                mine.actor.person.id,
            ));
        }
        if self
            .store()
            .another_in_attempt(mine.work.issue, mine.work.id)
            .await
            .map_err(|error| internal::<S>(&error))?
        {
            return Err(ErrorCode::AgentWorkNotReady.into());
        }
        let agent = self.agent_of(&mine).await?;
        let working = self
            .store()
            .working_issues_of(agent.id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        self.ensure_agent_room(
            &agent,
            LimitKind::IssuesAtOnce,
            usize::try_from(working).unwrap_or(usize::MAX),
            mode == Plan::Writing,
        )
        .await?;
        let attempts = self
            .store()
            .attempts_of(mine.work.id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        self.ensure_agent_room(
            &agent,
            LimitKind::AttemptsOnOneIssue,
            attempts.len(),
            mode == Plan::Writing,
        )
        .await?;
        let held = i64::try_from(attempts.len()).unwrap_or(i64::MAX);
        if held >= mine.hand_over.budget_attempts {
            return Err(ErrorCode::AgentWorkBudget.into());
        }
        let attempt = Attempt {
            id: AttemptId::generate(&mut self.ids()),
            work: mine.work.id,
            number: held + 1,
            plan: None,
            plan_approved: false,
            reported_minutes: 0,
            reported_cost: 0,
            started_at: self.now(),
            ended_at: None,
            run: None,
        };
        Ok(TakePlan {
            mine,
            agent,
            attempt,
        })
    }

    /// What taking an issue would change, without saving anything. A tool that goes on to take it
    /// asks with `writing`, so a limit that is reached pauses the agent here as it would there.
    ///
    /// # Errors
    /// What `take_work` refuses.
    pub(crate) async fn preview_take_work(
        &self,
        input: &TakeWork,
        writing: bool,
    ) -> Result<Vec<FieldChange>, AppError> {
        let mode = if writing {
            Plan::Writing
        } else {
            Plan::Preview
        };
        Ok(self.plan_take(input, mode).await?.changes())
    }
}

/// Takes an issue that was handed over: an attempt starts, and the work is *Agent working*.
///
/// # Errors
/// `resource.not-found` for an issue the agent was not given, `agent-work.not-ready` for work that
/// is not waiting for the agent or an issue another agent is working on, `edit.version-changed`
/// (with what changed) when the issue is not at the version the agent read,
/// `agent-work.budget-reached` when the hand-over's attempts are used, `agent.limit-reached` (and
/// the agent is paused) when it works on as many issues as it may or has made as many attempts on
/// this one as it may.
pub async fn take_work<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: TakeWork,
) -> Result<Authorized<WorkView>, AppError> {
    let TakePlan {
        mine,
        agent,
        mut attempt,
    } = ctx.plan_take(&input, Plan::Writing).await?;
    let run = ctx.open_attempt_run(&input.session_token, &mine).await?;
    attempt.run = Some(run);
    let next = AgentWorkState::AgentWorking;
    let event = ctx.work_event(mine.actor.person.id, &mine.work, next)?;
    let (work, stored, closing) = (mine.work.clone(), attempt.clone(), attempt);
    let limits = agent.limits();
    let why = Why::default();
    let stopped = why.clone();
    let made = ctx
        .store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // Judged again under the lock of the issue and then the lock of the agent, on what is
                // stored now: of two agents who take the same issue at the same moment the second
                // finds the first in its attempt, and two takes of one agent cannot both pass its
                // limit of issues at once.
                S::lock_taking_of(transaction, work.issue).await?;
                S::lock_working_of(transaction, work.agent).await?;
                if S::another_in_attempt_in(transaction, work.issue, work.id).await? {
                    return Err(stopped.stop::<S>(ErrorCode::AgentWorkNotReady));
                }
                let working = S::working_issues_of_in(transaction, work.agent).await?;
                if !limits.has_room(
                    LimitKind::IssuesAtOnce,
                    usize::try_from(working).unwrap_or(usize::MAX),
                ) {
                    return Err(stopped.stop::<S>(ErrorCode::AgentLimitReached));
                }
                S::insert_attempt(transaction, &stored).await?;
                if S::set_work_state(transaction, work.id, next, None, false, work.version).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await;
    if let Err(error) = made {
        // The run was opened for an attempt that did not start: it does not stay *Running*.
        let _ = ctx
            .close_attempt_run(&input.session_token, &mine, &closing, RunEnd::Failed)
            .await;
        return Err(match why.said() {
            Some(ErrorCode::AgentLimitReached) => {
                ctx.pause_for_limit(&agent, LimitKind::IssuesAtOnce).await
            }
            Some(code) => AppError::Refused(code),
            None => change_failed::<S>(&error, Stops::both(ErrorCode::AgentWorkNotReady)),
        });
    }
    ctx.work_after(&mine, mine.work.id).await
}
