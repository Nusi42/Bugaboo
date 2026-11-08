//! What the agent itself does on an issue it was given (AI-33, AI-34, ADR-0029, decisions 4 and
//! 5): report an attempt, ask, attach a result, say it is ready or give up. The routes the tools of
//! agents call are these use cases.
//!
//! What an agent reports is its own word: minutes and cost stay on the attempt and are never a time
//! entry (M7-Q7).

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::agent_records::{Attempt, AttemptResult, HandOver, IssueWork, Question, ResultKind};
use bb_core::agent_work::{AgentWorkState, CheckResult};
use bb_core::event::{DomainEvent, FieldChange, RecordKind};
use bb_core::id::{AgentNoteId, AgentWorkId, AttemptId};
use bb_core::run::RunEnd;
use bb_core::version::counted_up;
use bb_policy::{Authorized, Evaluation};
use serde_json::json;

use super::members::{member, waiting_for};
use super::plan::state_change;
use super::stop::{Closing, close_work_of, closing_again};
use super::types::{AskQuestion, AttachResult, EndAttempt, ReportAttempt, WorkView};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, entity, internal, set_to};
use crate::input::{clean_long_text, clean_text};

/// The longest a question, a plan or a reason may be.
const TEXT_MAX: usize = bb_core::text::LONG_TEXT_MAX;
/// The longest a label of a result may be.
const LABEL_MAX: usize = 200;
/// The longest a link of a result may be.
const LINK_MAX: usize = 2000;

/// The work an agent asks about: its open work on the issue, with the hand-over that gave it.
pub(super) struct Mine {
    pub(super) actor: Actor,
    pub(super) held: Arc<Evaluation>,
    pub(super) work: IssueWork,
    pub(super) hand_over: HandOver,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The open work of the agent behind the key on the issue.
    pub(super) async fn my_work(&self, token: &str, address: &str) -> Result<Mine, AppError> {
        let actor = self.actor(token).await?;
        let Some(agent) = actor.agent() else {
            return Err(ErrorCode::Forbidden.into());
        };
        let (issue, held) = self.visible_issue(&actor, address).await?;
        let work = self
            .store()
            .open_work_of(agent, issue.id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let hand_over = self
            .store()
            .hand_over(work.hand_over)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        Ok(Mine {
            actor,
            held,
            work,
            hand_over,
        })
    }

    /// The event of a change of the state of a piece of work.
    pub(super) fn work_event(
        &self,
        by: bb_core::id::UserId,
        work: &IssueWork,
        state: AgentWorkState,
    ) -> Result<DomainEvent, AppError> {
        self.work_event_waiting(by, work, state, None)
    }

    /// The event of a change to a piece of work that leaves its state as it is (an attached result,
    /// a look): the version counts up, so a stale reader of the work is told, and nobody is told
    /// of a new state.
    pub(super) fn work_event_unchanged(
        &self,
        by: bb_core::id::UserId,
        work: &IssueWork,
    ) -> Result<DomainEvent, AppError> {
        self.event(
            crate::auth::by(by),
            entity(RecordKind::AgentWork, work.id),
            vec![
                set_to(member::ISSUE, json!(work.issue.to_string())),
                set_to(member::AGENT, json!(work.agent.to_string())),
                set_to(member::HAND_OVER, json!(work.hand_over.to_string())),
                counted_up(work.version),
            ],
        )
    }

    /// The same, saying what the work waits for when it waits for a person: a question or a plan
    /// (the reader of notifications tells them apart by it).
    pub(super) fn work_event_waiting(
        &self,
        by: bb_core::id::UserId,
        work: &IssueWork,
        state: AgentWorkState,
        waiting: Option<&str>,
    ) -> Result<DomainEvent, AppError> {
        let mut changes = vec![
            set_to(member::ISSUE, json!(work.issue.to_string())),
            set_to(member::AGENT, json!(work.agent.to_string())),
            set_to(member::HAND_OVER, json!(work.hand_over.to_string())),
            set_to(member::WORK_STATE, json!(state.code())),
            counted_up(work.version),
        ];
        if let Some(waiting) = waiting {
            changes.push(set_to(member::WAITING_FOR, json!(waiting)));
        }
        self.event(
            crate::auth::by(by),
            entity(RecordKind::AgentWork, work.id),
            changes,
        )
    }

    /// The work as the actor sees it after a change.
    pub(super) async fn work_after(
        &self,
        mine: &Mine,
        id: AgentWorkId,
    ) -> Result<Authorized<WorkView>, AppError> {
        let work = self
            .store()
            .issue_work(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let view = self.work_view(&mine.held, &work).await?;
        Ok(mine.held.authorize(view))
    }
}

/// The attempt that has not ended, if the agent has one on this work.
fn running(attempts: &[Attempt]) -> Option<&Attempt> {
    attempts.last().filter(|attempt| attempt.ended_at.is_none())
}

/// Whether an attempt may start or go on: the hand-over still gives work, and the work is not
/// waiting for a person or at its end.
fn may_work(state: AgentWorkState) -> bool {
    matches!(
        state,
        AgentWorkState::WaitingForTheAgent | AgentWorkState::AgentWorking
    )
}

/// Refuses a report that would take the time or the cost the agent reports past the budget of the
/// hand-over.
fn ensure_within_budget(
    hand_over: &HandOver,
    attempts: &[Attempt],
    current: AttemptId,
    input: &ReportAttempt,
) -> Result<(), AppError> {
    let others = || attempts.iter().filter(move |each| each.id != current);
    let minutes = others().map(|each| each.reported_minutes).sum::<i64>() + input.minutes;
    let cost = others().map(|each| each.reported_cost).sum::<i64>() + input.cost;
    if hand_over.budget_minutes.is_some_and(|most| minutes > most)
        || hand_over.budget_cost.is_some_and(|most| cost > most)
    {
        return Err(ErrorCode::AgentWorkBudget.into());
    }
    Ok(())
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The agent behind the work, as it is stored now (its limits).
    pub(super) async fn agent_of(&self, mine: &Mine) -> Result<bb_core::agent::Agent, AppError> {
        self.store()
            .agent(mine.work.agent)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }
}

/// What a report would do: everything that is judged before anything is written.
pub(super) struct ReportPlan {
    pub(super) mine: Mine,
    pub(super) attempt: Attempt,
    pub(super) next: AgentWorkState,
    pub(super) waiting: Option<&'static str>,
}

impl ReportPlan {
    /// The fields the report changes, as a preview lists them.
    pub(super) fn changes(&self) -> Vec<FieldChange> {
        let mut changes = vec![state_change(self.mine.work.state, self.next)];
        if let Some(plan) = &self.attempt.plan {
            changes.push(set_to(member::PLAN, json!(plan)));
        }
        changes.push(set_to(
            member::MINUTES,
            json!(self.attempt.reported_minutes),
        ));
        changes.push(set_to(member::COST, json!(self.attempt.reported_cost)));
        changes
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Judges a report. Both the use case and the preview start here, so what is shown is what
    /// would be saved.
    pub(super) async fn plan_report(&self, input: &ReportAttempt) -> Result<ReportPlan, AppError> {
        let mine = self.my_work(&input.session_token, &input.issue).await?;
        if !may_work(mine.work.state) {
            return Err(ErrorCode::AgentWorkNotReady.into());
        }
        if input.minutes < 0 || input.cost < 0 {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let plan = input
            .plan
            .as_deref()
            .filter(|plan| !plan.trim().is_empty())
            .map(clean_long_text)
            .transpose()?;
        let attempts = self
            .store()
            .attempts_of(mine.work.id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        // Only *take* starts an attempt (same plan, same lock, same limits); a report goes to the one
        // that runs.
        let mut attempt = running(&attempts)
            .cloned()
            .ok_or(AppError::Refused(ErrorCode::AgentWorkNotReady))?;
        ensure_within_budget(&mine.hand_over, &attempts, attempt.id, input)?;
        if plan.is_some() {
            attempt.plan = plan;
        }
        attempt.reported_minutes = input.minutes;
        attempt.reported_cost = input.cost;
        // With *Plan needs approval* the first step is the plan; the work then waits for a person.
        let needs_plan = mine.hand_over.plan_needs_approval && !attempt.plan_approved;
        if needs_plan && attempt.plan.is_none() {
            return Err(ErrorCode::AgentWorkNotReady.into());
        }
        let next = if needs_plan {
            AgentWorkState::WaitingForYou
        } else {
            AgentWorkState::AgentWorking
        };
        Ok(ReportPlan {
            mine,
            attempt,
            next,
            waiting: needs_plan.then_some(waiting_for::PLAN),
        })
    }

    /// What a report would change, without saving anything.
    ///
    /// # Errors
    /// What `report_attempt` refuses.
    pub(crate) async fn preview_report_attempt(
        &self,
        input: &ReportAttempt,
    ) -> Result<Vec<FieldChange>, AppError> {
        Ok(self.plan_report(input).await?.changes())
    }
}

/// Reports the attempt that runs: the plan, and the time and cost the agent reports. An attempt is
/// started only by `take_work`. With *Plan needs approval* the plan comes first and the
/// work waits as *Waiting for you* until a person approves it.
///
/// # Errors
/// `resource.not-found` for an issue the agent was not given, `agent-work.not-ready` for work that
/// waits for a person or has ended, `agent-work.budget-reached` when the hand-over's attempts,
/// minutes or cost are used, `agent.limit-reached` when the agent's own limit of attempts on one
/// issue is, `request.invalid` for a number or text that cannot be.
pub async fn report_attempt<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ReportAttempt,
) -> Result<Authorized<WorkView>, AppError> {
    let ReportPlan {
        mine,
        attempt,
        next,
        waiting,
    } = ctx.plan_report(&input).await?;
    let event = ctx.work_event_waiting(mine.actor.person.id, &mine.work, next, waiting)?;
    let (work, stored) = (mine.work.clone(), attempt);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::update_attempt(transaction, &stored).await?;
                if work.state == next
                    || S::set_work_state(transaction, work.id, next, None, false, work.version)
                        .await?
                {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)))?;
    ctx.work_after(&mine, mine.work.id).await
}

/// What a question would do.
pub(super) struct AskPlan {
    mine: Mine,
    next: AgentWorkState,
    question: Question,
}

impl AskPlan {
    /// The fields the question changes, as a preview lists them.
    pub(super) fn changes(&self) -> Vec<FieldChange> {
        vec![
            state_change(self.mine.work.state, self.next),
            set_to(member::QUESTION, json!(self.question.text)),
        ]
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    async fn plan_ask(&self, input: &AskQuestion) -> Result<AskPlan, AppError> {
        let mine = self.my_work(&input.session_token, &input.issue).await?;
        let next = mine
            .work
            .state
            .after_question()
            .ok_or(AppError::Refused(ErrorCode::AgentWorkNotReady))?;
        let question = Question {
            id: AgentNoteId::generate(&mut self.ids()),
            work: mine.work.id,
            text: clean_text(&input.text, TEXT_MAX)?,
            answer: None,
            answered_by: None,
            asked_at: self.now(),
        };
        Ok(AskPlan {
            mine,
            next,
            question,
        })
    }

    /// What a question would change, without saving anything.
    ///
    /// # Errors
    /// What `ask_question` refuses.
    pub(crate) async fn preview_ask_question(
        &self,
        input: &AskQuestion,
    ) -> Result<Vec<FieldChange>, AppError> {
        Ok(self.plan_ask(input).await?.changes())
    }
}

/// Asks a question: the work waits as *Waiting for you* until a person answers.
///
/// # Errors
/// `resource.not-found` for an issue the agent was not given, `agent-work.not-ready` for work that
/// is not open or waits for its review, `request.invalid` for a question that cannot be.
pub async fn ask_question<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AskQuestion,
) -> Result<Authorized<WorkView>, AppError> {
    let AskPlan {
        mine,
        next,
        question,
    } = ctx.plan_ask(&input).await?;
    let event = ctx.work_event_waiting(
        mine.actor.person.id,
        &mine.work,
        next,
        Some(waiting_for::QUESTION),
    )?;
    let (work, stored) = (mine.work.clone(), question);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::insert_question(transaction, &stored).await?;
                if S::set_work_state(transaction, work.id, next, None, false, work.version).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)))?;
    ctx.work_after(&mine, mine.work.id).await
}

/// What an attached result would do.
pub(super) struct AttachPlan {
    mine: Mine,
    result: AttemptResult,
}

impl AttachPlan {
    /// The fields the result changes, as a preview lists them.
    pub(super) fn changes(&self) -> Vec<FieldChange> {
        let mut changes = vec![set_to(member::RESULT, json!(self.result.label))];
        if let Some(link) = &self.result.link {
            changes.push(set_to(member::LINK, json!(link)));
        }
        if let Some(outcome) = self.result.outcome {
            changes.push(set_to(member::OUTCOME, json!(outcome.code())));
        }
        changes
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    async fn plan_attach(&self, input: &AttachResult) -> Result<AttachPlan, AppError> {
        let mine = self.my_work(&input.session_token, &input.issue).await?;
        let attempts = self
            .store()
            .attempts_of(mine.work.id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let attempt = running(&attempts)
            .filter(|_| mine.work.state.is_open())
            .ok_or(AppError::Refused(ErrorCode::AgentWorkNotReady))?;
        let label = clean_text(&input.label, LABEL_MAX)?;
        let (link, outcome) = match input.kind {
            ResultKind::Link => {
                let link = clean_text(input.link.as_deref().unwrap_or_default(), LINK_MAX)?;
                // Only a web address: it is shown to people as a link.
                if !bb_core::agent_work::is_web_address(&link) {
                    return Err(ErrorCode::InvalidRequest.into());
                }
                (Some(link), None)
            }
            ResultKind::Check => (
                None,
                Some(
                    input
                        .outcome
                        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?,
                ),
            ),
        };
        let result = AttemptResult {
            id: AgentNoteId::generate(&mut self.ids()),
            attempt: attempt.id,
            kind: input.kind,
            label,
            link,
            outcome,
        };
        Ok(AttachPlan { mine, result })
    }

    /// What an attached result would change, without saving anything.
    ///
    /// # Errors
    /// What `attach_result` refuses.
    pub(crate) async fn preview_attach_result(
        &self,
        input: &AttachResult,
    ) -> Result<Vec<FieldChange>, AppError> {
        Ok(self.plan_attach(input).await?.changes())
    }
}

/// Attaches a result to the attempt that runs: a link, or a check with *passed* or *failed*.
/// Bugaboo does not run or verify checks (M7-Q8).
///
/// # Errors
/// `resource.not-found` for an issue the agent was not given, `agent-work.not-ready` for work with
/// no attempt that runs, `request.invalid` for a link without an address or a check without an
/// outcome.
pub async fn attach_result<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AttachResult,
) -> Result<Authorized<WorkView>, AppError> {
    let AttachPlan { mine, result } = ctx.plan_attach(&input).await?;
    let event = ctx.work_event_unchanged(mine.actor.person.id, &mine.work)?;
    let (work, stored) = (mine.work.clone(), result);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::insert_result(transaction, &stored).await?;
                // The work did not change state, only its version counts up, so a stale reader of
                // the work is told.
                if S::set_work_state(
                    transaction,
                    work.id,
                    work.state,
                    work.gave_up.as_deref(),
                    work.accepted_by_itself,
                    work.version,
                )
                .await?
                {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)))?;
    ctx.work_after(&mine, mine.work.id).await
}

/// What the end of an attempt would do.
pub(super) struct EndPlan {
    mine: Mine,
    attempt: Attempt,
    reason: Option<String>,
    next: AgentWorkState,
    by_itself: bool,
}

impl EndPlan {
    /// The fields the end changes, as a preview lists them.
    pub(super) fn changes(&self) -> Vec<FieldChange> {
        let mut changes = vec![state_change(self.mine.work.state, self.next)];
        if let Some(reason) = &self.reason {
            changes.push(set_to(member::GAVE_UP, json!(reason)));
        }
        if self.by_itself {
            changes.push(set_to(member::ACCEPTED_BY_ITSELF, json!(true)));
        }
        changes
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    async fn plan_end(&self, input: &EndAttempt) -> Result<EndPlan, AppError> {
        let mine = self.my_work(&input.session_token, &input.issue).await?;
        if mine.work.state != AgentWorkState::AgentWorking {
            return Err(ErrorCode::AgentWorkNotReady.into());
        }
        let attempts = self
            .store()
            .attempts_of(mine.work.id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut attempt = running(&attempts)
            .cloned()
            .ok_or(AppError::Refused(ErrorCode::AgentWorkNotReady))?;
        let reason = input
            .gave_up
            .as_deref()
            .map(|reason| clean_text(reason, TEXT_MAX))
            .transpose()?;
        attempt.ended_at = Some(self.now());
        let next = if reason.is_some() {
            AgentWorkState::AgentGaveUp
        } else {
            AgentWorkState::ReadyForReview
        };
        let results = self
            .store()
            .results_of(attempt.id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let checks: Vec<CheckResult> = results.iter().filter_map(|result| result.outcome).collect();
        let by_itself = reason.is_none() && self.accepts_by_itself_in(&mine, &checks).await?;
        Ok(EndPlan {
            mine,
            attempt,
            reason,
            next,
            by_itself,
        })
    }

    /// What the end of an attempt would change, without saving anything.
    ///
    /// # Errors
    /// What `end_attempt` refuses.
    pub(crate) async fn preview_end_attempt(
        &self,
        input: &EndAttempt,
    ) -> Result<Vec<FieldChange>, AppError> {
        Ok(self.plan_end(input).await?.changes())
    }
}

/// Says the attempt is ready for review, or gives up with a reason.
///
/// # Errors
/// `resource.not-found` for an issue the agent was not given, `agent-work.not-ready` for work with
/// no attempt that runs or that waits for a person, `request.invalid` for a reason that cannot be.
pub async fn end_attempt<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: EndAttempt,
) -> Result<Authorized<WorkView>, AppError> {
    let EndPlan {
        mine,
        attempt,
        reason,
        next,
        by_itself,
    } = ctx.plan_end(&input).await?;
    // The end is claimed first, on the version the agent read: of two requests that end the same
    // attempt at the same moment one wins, and only the winner writes the summary or moves the
    // status.
    let event = ctx.work_event(mine.actor.person.id, &mine.work, next)?;
    let (work, stored) = (mine.work.clone(), attempt.clone());
    let reason_kept = reason.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::update_attempt(transaction, &stored).await?;
                if S::set_work_state(
                    transaction,
                    work.id,
                    next,
                    reason_kept.as_deref(),
                    false,
                    work.version,
                )
                .await?
                {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)))?;
    let gave_up = reason.is_some();
    let after = ctx.finish_ended_attempt(&input.session_token, &mine, by_itself, gave_up);
    let ended = if gave_up {
        RunEnd::Failed
    } else {
        RunEnd::Done
    };
    let finished = after.await;
    ctx.close_attempt_run(&input.session_token, &mine, &attempt, ended)
        .await?;
    finished?;
    ctx.work_after(&mine, mine.work.id).await
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What the winner of the end of an attempt does next, as the agent, while it still reaches the
    /// issue: the short comment, and after an accept by itself the move of the status and the end
    /// of the work.
    async fn finish_ended_attempt(
        &self,
        token: &str,
        mine: &Mine,
        by_itself: bool,
        gave_up: bool,
    ) -> Result<(), AppError> {
        let failed = |error| internal::<S>(&error);
        let attempt = self
            .store()
            .attempts_of(mine.work.id)
            .await
            .map_err(failed)?
            .pop()
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        if gave_up {
            return Ok(());
        }
        self.write_summary(token, mine, &attempt).await?;
        if !by_itself {
            return Ok(());
        }
        let status_pending = !self.move_after_accept(token, mine).await?;
        let work = IssueWork {
            version: mine.work.version + 1,
            ..mine.work.clone()
        };
        let event = self.work_event_unchanged(mine.actor.person.id, &work)?;
        let hand_over = mine.hand_over.clone();
        let closing = Closing {
            work: work.id,
            version: work.version,
            accepted_by_itself: true,
            status_pending,
        };
        closing_again::<S, _, _>(|| {
            let hand_over = hand_over.id;
            self.store().record_change(&event, move |transaction| {
                Box::pin(async move { close_work_of::<S>(transaction, hand_over, closing).await })
            })
        })
        .await
    }

    /// Whether the attempt is accepted by itself (M7-Q8): the project allows it, the hand-over did
    /// not turn it off, and it has at least one check and every check passed.
    async fn accepts_by_itself_in(
        &self,
        mine: &Mine,
        checks: &[CheckResult],
    ) -> Result<bool, AppError> {
        let project = self
            .store()
            .issue(mine.work.issue)
            .await
            .map_err(|error| internal::<S>(&error))?
            .map(|issue| issue.project);
        let project_allows = match project {
            Some(project) => self.project_accepts_by_itself(project).await?,
            None => false,
        };
        Ok(bb_core::agent_work::accepts_by_itself(
            project_allows,
            mine.hand_over.accept_by_itself,
            checks,
        ))
    }
}
