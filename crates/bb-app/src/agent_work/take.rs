//! Taking a handed-over issue (AI-36, ADR-0029, decision 9): all or nothing. The agent takes the work
//! it was handed with the version of the issue it read; an attempt starts, with its run, and the work
//! is *Agent working*. Of two agents who take the same issue at the same moment one gets it: the
//! taking is judged under a lock of the issue, on what is stored then.
//!
//! The limits of the agent are judged here and in the first report (`attempt_to_report`), the one
//! place that starts an attempt: issues at once, and attempts on one issue.

use bb_core::ErrorCode;
use bb_core::agent::LimitKind;
use bb_core::agent_records::Attempt;
use bb_core::agent_work::AgentWorkState;
use bb_core::event::{FieldChange, RecordKind};
use bb_policy::{Authorized, Place};
use serde_json::json;

use super::attempt::Mine;
use super::members::member;
use super::plan::{Plan, set, state_change};
use super::types::{TakeWork, WorkView};
use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, entity, internal};

/// What taking an issue would do: everything judged before anything is written.
pub(super) struct TakePlan {
    mine: Mine,
    attempt: Attempt,
}

impl TakePlan {
    /// The fields taking changes, as a preview lists them.
    pub(super) fn changes(&self) -> Vec<FieldChange> {
        vec![
            state_change(self.mine.work.state, AgentWorkState::AgentWorking),
            set(member::ATTEMPT, json!(self.attempt.number)),
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
        if self.another_agent_works_on(&mine).await? {
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
        let (attempt, _) = self.attempt_to_report(&mine, &attempts, mode).await?;
        Ok(TakePlan { mine, attempt })
    }

    /// Whether the issue is being worked on by another agent: one of its pieces of work is in an
    /// attempt.
    async fn another_agent_works_on(&self, mine: &Mine) -> Result<bool, AppError> {
        Ok(self
            .store()
            .work_on_issue(mine.work.issue, None, 50)
            .await
            .map_err(|error| internal::<S>(&error))?
            .iter()
            .any(|work| work.id != mine.work.id && work.state.in_attempt()))
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
    let TakePlan { mine, mut attempt } = ctx.plan_take(&input, Plan::Writing).await?;
    attempt.run = Some(ctx.open_attempt_run(&input.session_token, &mine).await?);
    let next = AgentWorkState::AgentWorking;
    let event = ctx.work_event(mine.actor.person.id, &mine.work, next)?;
    let (work, stored) = (mine.work.clone(), attempt);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // Judged again under the lock of the issue, on what is stored now: of two agents
                // who take it at the same moment the second finds the first one in its attempt.
                S::lock_taking_of(transaction, work.issue).await?;
                let others = S::work_on_issue_in(transaction, work.issue, None, 50).await?;
                if others
                    .iter()
                    .any(|other| other.id != work.id && other.state.in_attempt())
                {
                    return Err(S::aborted());
                }
                S::insert_attempt(transaction, &stored).await?;
                if S::set_work_state(transaction, work.id, next, None, false, work.version).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::AgentWorkNotReady)))?;
    ctx.work_after(&mine, mine.work.id).await
}
