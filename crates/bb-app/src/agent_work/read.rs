//! Reading the agent work on an issue (AI-33): who works on it, what the brief says, the attempts,
//! the questions and the reviews.

use bb_core::agent_records::{GivingState, IssueWork, ProposalState};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Authorized, Evaluation, PeopleContext};

use super::types::{
    AgentWorkQuery, AttemptView, IssueAgentWork, MOST_SHOWN_ON_AN_ISSUE, ProposalView,
    QuestionView, ResultView, ReviewView, WorkView, WorkedOnBy,
};
use crate::auth::{AppError, AuthStore, Ctx, internal};

impl<S: AuthStore> Ctx<'_, S> {
    /// One piece of work as the viewer may see it.
    pub(crate) async fn work_view(
        &self,
        viewer: &Evaluation,
        work: &IssueWork,
    ) -> Result<WorkView, AppError> {
        let store = self.store();
        let failed = |error| internal::<S>(&error);
        let hand_over = store
            .hand_over(work.hand_over)
            .await
            .map_err(failed)?
            .ok_or_else(|| AppError::Internal("a piece of work without a hand-over".to_owned()))?;
        let questions = store.questions_of(work.id).await.map_err(failed)?;
        let mut brief = Vec::new();
        if !hand_over.note.is_empty() {
            brief.push(hand_over.note.clone());
        }
        brief.extend(
            questions
                .iter()
                .filter_map(|question| question.answer.clone()),
        );
        let mut attempts = Vec::new();
        for attempt in store.attempts_of(work.id).await.map_err(failed)? {
            let results = store
                .results_of(attempt.id)
                .await
                .map_err(failed)?
                .into_iter()
                .map(|result| ResultView {
                    kind: result.kind,
                    label: result.label,
                    link: result.link,
                    outcome: result.outcome,
                })
                .collect();
            attempts.push(AttemptView {
                id: attempt.id,
                number: attempt.number,
                plan: attempt.plan,
                plan_approved: attempt.plan_approved,
                reported_minutes: attempt.reported_minutes,
                reported_cost: attempt.reported_cost,
                started_at: attempt.started_at,
                ended_at: attempt.ended_at,
                results,
            });
        }
        let mut reviews = Vec::new();
        for review in store.reviews_of(work.id).await.map_err(failed)? {
            let by = match review.by {
                Some(person) => Some(
                    self.person_ref(
                        viewer,
                        PeopleKind::ProjectMembers,
                        PeopleContext::Ordinary,
                        person,
                    )
                    .await?,
                ),
                None => None,
            };
            reviews.push(ReviewView {
                answer: review.answer,
                note: review.note,
                by,
                at: review.at,
            });
        }
        Ok(WorkView {
            id: work.id,
            hand_over: work.hand_over,
            agent: work.agent,
            state: work.state,
            open: hand_over.state == GivingState::Open && work.state.is_open(),
            brief,
            attempts,
            questions: questions
                .into_iter()
                .map(|question| QuestionView {
                    id: question.id,
                    text: question.text,
                    answer: question.answer,
                    asked_at: question.asked_at,
                })
                .collect(),
            reviews,
            gave_up: work.gave_up.clone(),
            accepted_by_itself: work.accepted_by_itself,
            choose_next_status: work.status_pending,
            version: work.version,
        })
    }
}

/// The agent work on an issue the actor may see: who works on it now, all the work, and the
/// proposals of standing rules that wait for a person. The issue keeps its person; an agent is
/// shown as *worked on by*, never as *Assigned to*.
///
/// # Errors
/// `resource.not-found` for an issue that does not exist or that the actor may not see.
pub async fn issue_agent_work<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AgentWorkQuery,
) -> Result<Authorized<IssueAgentWork>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let failed = |error| internal::<S>(&error);
    let mut all = ctx.store().work_on_issue(issue.id).await.map_err(failed)?;
    let mut truncated = all.len() > MOST_SHOWN_ON_AN_ISSUE;
    all.truncate(MOST_SHOWN_ON_AN_ISSUE);
    let mut work = Vec::with_capacity(all.len());
    let mut worked_on_by: Vec<WorkedOnBy> = Vec::new();
    for found in &all {
        let view = ctx.work_view(&held, found).await?;
        if view.open && !worked_on_by.iter().any(|shown| shown.agent == found.agent) {
            let agent = ctx
                .store()
                .agent(found.agent)
                .await
                .map_err(failed)?
                .ok_or(AppError::Refused(bb_core::ErrorCode::NotFound))?;
            worked_on_by.push(WorkedOnBy {
                agent: agent.id,
                name: agent.name,
                person: ctx
                    .person_ref(
                        &held,
                        PeopleKind::ProjectMembers,
                        PeopleContext::Ordinary,
                        agent.person,
                    )
                    .await?,
            });
        }
        work.push(view);
    }
    let mut waiting = ctx
        .store()
        .proposals_on(issue.id, Some(ProposalState::Open))
        .await
        .map_err(failed)?;
    truncated |= waiting.len() > MOST_SHOWN_ON_AN_ISSUE;
    waiting.truncate(MOST_SHOWN_ON_AN_ISSUE);
    let proposals = waiting
        .into_iter()
        .map(|proposal| ProposalView {
            id: proposal.id,
            rule: proposal.rule,
            agent: proposal.agent,
            field: proposal.field,
            before: proposal.before,
            after: proposal.after,
            state: proposal.state,
            made_at: proposal.made_at,
        })
        .collect();
    Ok(held.authorize(IssueAgentWork {
        worked_on_by,
        work,
        proposals,
        truncated,
    }))
}
