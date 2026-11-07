//! What the person does with the work of an agent: answer a question, approve a plan, and review
//! the result (AI-34, AI-35, ADR-0029, decisions 5 and 6).

use bb_core::ErrorCode;
use bb_core::agent_records::{IssueWork, Question, Review};
use bb_core::agent_work::{AgentWorkState, ReviewAnswer};
use bb_core::event::RecordKind;
use bb_core::id::AgentNoteId;
use bb_core::workflow::StatusKey;
use bb_policy::{Authorized, Evaluation, Permission, Place};
use serde_json::json;

use super::members::member;
use super::stop::close_work_of;
use super::types::{AnswerQuestion, ApprovePlan, LookAtWork, ReviewWork, WorkView};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, entity, internal, set_to};
use crate::input::{clean_long_text, clean_text};
use crate::issues::{ChangeIssue, change_issue};

/// The longest an answer or a note may be.
const TEXT_MAX: usize = bb_core::text::LONG_TEXT_MAX;

/// The person's view of the work they act on.
struct Acting {
    actor: Actor,
    held: std::sync::Arc<Evaluation>,
    work: IssueWork,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The work, when the person answers for it: the person responsible for the agent or the one
    /// who handed over. Anybody else is told there is no such work.
    async fn work_to_answer(
        &self,
        token: &str,
        id: bb_core::id::AgentWorkId,
        version: Option<i64>,
    ) -> Result<Acting, AppError> {
        let record = self.active(token).await?;
        let person = record.person.id;
        let actor = Actor {
            person: record.person,
            key: None,
        };
        let failed = |error| internal::<S>(&error);
        let work = self
            .store()
            .issue_work(id)
            .await
            .map_err(failed)?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let people = self.store().people_of_work(id).await.map_err(failed)?;
        let seen = self.seen_issue_by_id(&actor, work.issue).await?;
        let (Some((_, held)), true) = (seen, people.contains(&person)) else {
            return Err(AppError::Refused(ErrorCode::NotFound));
        };
        if version.is_some_and(|version| version != work.version) {
            return Err(AppError::stale(
                entity(RecordKind::AgentWork, work.id),
                Place::Organization,
                person,
            ));
        }
        Ok(Acting { actor, held, work })
    }

    async fn after_review(&self, acting: &Acting) -> Result<Authorized<WorkView>, AppError> {
        let work = self
            .store()
            .issue_work(acting.work.id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let view = self.work_view(&acting.held, &work).await?;
        Ok(acting.held.authorize(view))
    }
}

/// The state the work goes back to when nothing waits for a person any more.
fn state_after_answers(running_attempt: bool) -> AgentWorkState {
    if running_attempt {
        AgentWorkState::AgentWorking
    } else {
        AgentWorkState::WaitingForTheAgent
    }
}

/// Answers a question. The answer joins the brief, so the next attempt and any other agent read it;
/// the work goes on when nothing else waits for the person.
///
/// # Errors
/// `resource.not-found` for work the person does not answer for, `agent-work.not-ready` for a
/// question that was answered already, `request.invalid` for an answer that cannot be,
/// `edit.version-changed` when the work changed meanwhile.
pub async fn answer_question<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AnswerQuestion,
) -> Result<Authorized<WorkView>, AppError> {
    let acting = ctx
        .work_to_answer(&input.session_token, input.work, Some(input.version))
        .await?;
    let answer = clean_text(&input.reply, TEXT_MAX)?;
    let failed = |error| internal::<S>(&error);
    let question: Question = ctx
        .store()
        .question(input.question)
        .await
        .map_err(failed)?
        .filter(|found| found.work == acting.work.id)
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    if question.answer.is_some() || acting.work.state != AgentWorkState::WaitingForYou {
        return Err(ErrorCode::AgentWorkNotReady.into());
    }
    let questions = ctx
        .store()
        .questions_of(acting.work.id)
        .await
        .map_err(failed)?;
    let attempts = ctx
        .store()
        .attempts_of(acting.work.id)
        .await
        .map_err(failed)?;
    let open_attempt = attempts.last().filter(|attempt| attempt.ended_at.is_none());
    let other_questions = questions
        .iter()
        .any(|other| other.id != question.id && other.answer.is_none());
    let plan_waits = open_attempt
        .is_some_and(|attempt| attempt.plan.is_some() && !attempt.plan_approved)
        && attempts.last().is_some()
        && ctx.plan_needs_approval_of(acting.work.hand_over).await?;
    let next = if other_questions || plan_waits {
        AgentWorkState::WaitingForYou
    } else {
        state_after_answers(open_attempt.is_some())
    };
    let person = acting.actor.person.id;
    let event = ctx.work_event(person, &acting.work, next)?;
    let (work, id) = (acting.work.clone(), question.id);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::set_answer(transaction, id, &answer, person).await?;
                if S::set_work_state(transaction, work.id, next, None, false, work.version).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)))?;
    ctx.after_review(&acting).await
}

/// *Approve plan*: the attempt may start. A note instead is the answer to the plan: it joins the
/// brief, the plan stays unapproved and the agent reports a new one.
///
/// # Errors
/// `resource.not-found` for work the person does not answer for, `agent-work.not-ready` for work
/// that waits for no plan, `request.invalid` for a note that cannot be.
pub async fn approve_plan<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ApprovePlan,
) -> Result<Authorized<WorkView>, AppError> {
    let acting = ctx
        .work_to_answer(&input.session_token, input.work, Some(input.version))
        .await?;
    let failed = |error| internal::<S>(&error);
    let attempts = ctx
        .store()
        .attempts_of(acting.work.id)
        .await
        .map_err(failed)?;
    let mut attempt = attempts
        .last()
        .filter(|attempt| attempt.ended_at.is_none() && attempt.plan.is_some())
        .cloned()
        .filter(|_| acting.work.state == AgentWorkState::WaitingForYou)
        .ok_or(AppError::Refused(ErrorCode::AgentWorkNotReady))?;
    let note = input
        .note
        .as_deref()
        .filter(|note| !note.trim().is_empty())
        .map(clean_long_text)
        .transpose()?;
    let person = acting.actor.person.id;
    let now = ctx.now();
    let (next, reply) = match note {
        // The plan is the question, the note its answer.
        Some(note) => (
            AgentWorkState::WaitingForYou,
            Some(Question {
                id: AgentNoteId::generate(&mut ctx.ids()),
                work: acting.work.id,
                text: attempt.plan.clone().unwrap_or_default(),
                answer: Some(note),
                answered_by: Some(person),
                asked_at: now,
            }),
        ),
        None => (AgentWorkState::AgentWorking, None),
    };
    if reply.is_none() {
        attempt.plan_approved = true;
    } else {
        // The plan is answered: a new plan replaces it.
        attempt.plan = None;
    }
    let event = ctx.work_event(person, &acting.work, next)?;
    let work = acting.work.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::update_attempt(transaction, &attempt).await?;
                if let Some(reply) = &reply {
                    S::insert_question(transaction, reply).await?;
                }
                if S::set_work_state(transaction, work.id, next, None, false, work.version).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)))?;
    ctx.after_review(&acting).await
}

/// Marks work that was accepted by itself as looked at.
///
/// # Errors
/// `resource.not-found` for work the person does not answer for.
pub async fn look_at_work<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: LookAtWork,
) -> Result<Authorized<WorkView>, AppError> {
    let acting = ctx
        .work_to_answer(&input.session_token, input.work, None)
        .await?;
    let event = ctx.work_event(acting.actor.person.id, &acting.work, acting.work.state)?;
    let id = acting.work.id;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::mark_work_seen(transaction, id).await })
        })
        .await
        .map_err(|error| internal::<S>(&error))?;
    ctx.after_review(&acting).await
}

/// The review of a result: *Accept* ends the hand-over and moves the status the reviewer picked,
/// as the workflow's own change; *Send back* starts another attempt with the note in the brief.
///
/// # Errors
/// `resource.not-found` for work the person does not answer for, `agent-work.not-ready` for work
/// that is not *Ready for review*, `request.invalid` for a *Send back* without a note or a note
/// that cannot be, and the refusals of the change of the status.
pub async fn review_work<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ReviewWork,
) -> Result<Authorized<WorkView>, AppError> {
    let acting = ctx
        .work_to_answer(&input.session_token, input.work, Some(input.version))
        .await?;
    if acting.work.state != AgentWorkState::ReadyForReview {
        return Err(ErrorCode::AgentWorkNotReady.into());
    }
    let person = acting.actor.person.id;
    let note = input
        .note
        .as_deref()
        .filter(|note| !note.trim().is_empty())
        .map(clean_long_text)
        .transpose()?;
    let (next, ends) = match input.verdict {
        ReviewAnswer::Accept => (AgentWorkState::ReadyForReview, true),
        ReviewAnswer::SendBack if note.is_some() => (AgentWorkState::WaitingForTheAgent, false),
        ReviewAnswer::SendBack | ReviewAnswer::ThrowAway => {
            return Err(ErrorCode::InvalidRequest.into());
        }
    };
    if ends {
        ctx.move_status(&input.session_token, &acting, input.status)
            .await?;
    }
    let review = Review {
        id: AgentNoteId::generate(&mut ctx.ids()),
        work: acting.work.id,
        answer: input.verdict,
        note,
        by: Some(person),
        at: ctx.now(),
    };
    let failed = |error| internal::<S>(&error);
    let hand_over = ctx
        .store()
        .hand_over(acting.work.hand_over)
        .await
        .map_err(failed)?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let works = ctx
        .store()
        .works_of_hand_over(hand_over.id)
        .await
        .map_err(failed)?;
    let event = ctx.event(
        crate::auth::by(person),
        entity(RecordKind::AgentWork, acting.work.id),
        vec![
            set_to(member::ISSUE, json!(acting.work.issue.to_string())),
            set_to(member::AGENT, json!(acting.work.agent.to_string())),
            set_to(member::REVIEW, json!(review.answer.code())),
            set_to(member::WORK_STATE, json!(next.code())),
            bb_core::version::counted_up(acting.work.version),
        ],
    )?;
    let work = acting.work.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::insert_review(transaction, &review).await?;
                if !S::set_work_state(transaction, work.id, next, None, false, work.version).await?
                {
                    return Err(S::aborted());
                }
                if ends {
                    close_work_of::<S>(transaction, &hand_over, &works, work.id, false).await?;
                }
                Ok(())
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)))?;
    ctx.after_review(&acting).await
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether the hand-over asks for the approval of each plan.
    async fn plan_needs_approval_of(
        &self,
        hand_over: bb_core::id::HandOverId,
    ) -> Result<bool, AppError> {
        Ok(self
            .store()
            .hand_over(hand_over)
            .await
            .map_err(|error| internal::<S>(&error))?
            .is_some_and(|found| found.plan_needs_approval))
    }

    /// Moves the status of the issue to the one the reviewer picked, as the reviewer, through the
    /// one change of an issue (the workflow judges it).
    async fn move_status(
        &self,
        token: &str,
        acting: &Acting,
        status: Option<StatusKey>,
    ) -> Result<(), AppError> {
        let Some(status) = status else {
            return Ok(());
        };
        let (issue, held) = self
            .visible_issue(&acting.actor, &acting.work.issue.to_string())
            .await?;
        if !held.allows(Permission::ChangeIssues) && !held.may_change_issue(issue.reported_by) {
            return Err(ErrorCode::Forbidden.into());
        }
        change_issue(
            self,
            ChangeIssue {
                session_token: token.to_owned(),
                issue: issue.id.to_string(),
                version: issue.version,
                issue_type: None,
                title: None,
                status: Some(status),
                priority: None,
                estimated_time: None,
                time_left: None,
                assigned_to: None,
                fields: std::collections::BTreeMap::default(),
            },
        )
        .await
        .map(|_| ())
    }
}
