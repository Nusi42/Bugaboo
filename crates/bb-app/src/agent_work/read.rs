//! Reading the agent work on an issue (AI-33): who works on it, what the brief says, the attempts,
//! the questions and the reviews.

use std::sync::Arc;

use bb_core::agent_records::{HandOver, IssueWork, ProposalState};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Authorized, Evaluation, PeopleContext};

use super::store::WorkDetails;
use super::types::{
    AgentWorkQuery, AttemptView, IssueAgentWork, MOST_SHOWN_ON_AN_ISSUE, ProposalView,
    QuestionView, ResultView, ReviewView, WorkBudget, WorkView, WorkedOnBy,
};
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The brief the agent works from: the note, the answers, then what a person wrote when they sent
/// a result back.
fn brief_of(work: &IssueWork, hand_over: &HandOver, details: &WorkDetails) -> Vec<String> {
    let mut brief = Vec::new();
    if !hand_over.note.is_empty() {
        brief.push(hand_over.note.clone());
    }
    brief.extend(
        details
            .questions
            .iter()
            .filter(|question| question.work == work.id)
            .filter_map(|question| question.answer.clone()),
    );
    brief.extend(
        details
            .reviews
            .iter()
            .filter(|review| {
                review.work == work.id
                    && review.answer == bb_core::agent_work::ReviewAnswer::SendBack
            })
            .filter_map(|review| review.note.clone()),
    );
    brief
}

/// The attempts of a piece of work with what each attached, oldest first.
fn attempts_of(work: &IssueWork, details: &WorkDetails) -> Vec<AttemptView> {
    details
        .attempts
        .iter()
        .filter(|attempt| attempt.work == work.id)
        .map(|attempt| AttemptView {
            id: attempt.id,
            number: attempt.number,
            plan: attempt.plan.clone(),
            plan_approved: attempt.plan_approved,
            reported_minutes: attempt.reported_minutes,
            reported_cost: attempt.reported_cost,
            started_at: attempt.started_at,
            ended_at: attempt.ended_at,
            run: attempt.run,
            results: details
                .results
                .iter()
                .filter(|result| result.attempt == attempt.id)
                .map(|result| ResultView {
                    kind: result.kind,
                    label: result.label.clone(),
                    link: result.link.clone(),
                    outcome: result.outcome,
                })
                .collect(),
        })
        .collect()
}

impl<S: AuthStore> Ctx<'_, S> {
    /// One piece of work as the viewer may see it.
    pub(crate) async fn work_view(
        &self,
        viewer: &Evaluation,
        work: &IssueWork,
    ) -> Result<WorkView, AppError> {
        self.work_views(viewer, std::slice::from_ref(work))
            .await?
            .pop()
            .ok_or_else(|| AppError::Internal("a piece of work without a view".to_owned()))
    }

    /// Pieces of work as the viewer may see them: whatever belongs to them is read together, so
    /// a page costs the same few statements however many pieces it holds (PERF-N1).
    pub(crate) async fn work_views(
        &self,
        viewer: &Evaluation,
        works: &[IssueWork],
    ) -> Result<Vec<WorkView>, AppError> {
        let each: Vec<(&Evaluation, &IssueWork)> =
            works.iter().map(|work| (viewer, work)).collect();
        self.work_views_each(&each).await
    }

    /// The same for pieces that each have a viewer of their own (the rights of the person in the
    /// project of the issue): still one read of everything that belongs to them.
    pub(crate) async fn work_views_each(
        &self,
        works: &[(&Evaluation, &IssueWork)],
    ) -> Result<Vec<WorkView>, AppError> {
        let details = self
            .store()
            .details_of_works(works.iter().map(|(_, work)| (*work).clone()).collect())
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut agent_ids: Vec<bb_core::id::AgentId> =
            works.iter().map(|(_, work)| work.agent).collect();
        agent_ids.sort_unstable();
        agent_ids.dedup();
        let agents = self
            .store()
            .agents_by_ids(agent_ids)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut views = Vec::with_capacity(works.len());
        for (viewer, work) in works {
            views.push(self.work_view_in(viewer, work, &details, &agents).await?);
        }
        Ok(views)
    }

    /// The view of proposals, each with the name of its agent (a rule hands nothing over, so the
    /// name cannot come from the work): the one place a `ProposalView` is made.
    pub(super) async fn proposal_views(
        &self,
        waiting: &[bb_core::agent_records::Proposal],
    ) -> Result<Vec<ProposalView>, AppError> {
        let mut agents: Vec<bb_core::id::AgentId> =
            waiting.iter().map(|proposal| proposal.agent).collect();
        agents.sort_unstable();
        agents.dedup();
        let named = self
            .store()
            .agents_by_ids(agents)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut views = Vec::with_capacity(waiting.len());
        for proposal in waiting {
            let agent_name = named
                .iter()
                .find(|found| found.id == proposal.agent)
                .map(|found| found.name.clone())
                .ok_or(AppError::Refused(bb_core::ErrorCode::NotFound))?;
            views.push(ProposalView {
                id: proposal.id,
                rule: proposal.rule,
                agent: proposal.agent,
                agent_name,
                field: proposal.field,
                before: proposal.before.clone(),
                after: proposal.after.clone(),
                state: proposal.state,
                made_at: proposal.made_at,
            });
        }
        Ok(views)
    }

    async fn work_view_in(
        &self,
        viewer: &Evaluation,
        work: &IssueWork,
        details: &WorkDetails,
        agents: &[bb_core::agent::Agent],
    ) -> Result<WorkView, AppError> {
        let hand_over: &HandOver = details
            .hand_overs
            .iter()
            .find(|found| found.id == work.hand_over)
            .ok_or_else(|| AppError::Internal("a piece of work without a hand-over".to_owned()))?;
        let agent = agents
            .iter()
            .find(|found| found.id == work.agent)
            .ok_or_else(|| AppError::Internal("a piece of work without an agent".to_owned()))?;
        let person_of = |person| {
            self.person_ref(
                viewer,
                PeopleKind::ProjectMembers,
                PeopleContext::Ordinary,
                person,
            )
        };
        let agent_person = person_of(agent.person).await?;
        let handed_over_by = person_of(hand_over.person).await?;
        let questions: Vec<_> = details
            .questions
            .iter()
            .filter(|question| question.work == work.id)
            .collect();
        let brief = brief_of(work, hand_over, details);
        let attempts = attempts_of(work, details);
        let mut question_views = Vec::new();
        for question in &questions {
            let answered_by = match question.answered_by {
                Some(person) => Some(person_of(person).await?),
                None => None,
            };
            question_views.push(QuestionView {
                id: question.id,
                text: question.text.clone(),
                answer: question.answer.clone(),
                answered_by,
                answered_at: question.answered_at,
                asked_at: question.asked_at,
            });
        }
        let mut reviews = Vec::new();
        for review in details
            .reviews
            .iter()
            .filter(|review| review.work == work.id)
        {
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
                note: review.note.clone(),
                by,
                at: review.at,
            });
        }
        Ok(WorkView {
            id: work.id,
            hand_over: work.hand_over,
            agent: work.agent,
            agent_name: agent.name.clone(),
            agent_person,
            handed_over_by,
            handed_over_at: hand_over.made_at,
            ends_at: hand_over.ends_at,
            budget: WorkBudget {
                attempts: hand_over.budget_attempts,
                minutes: hand_over.budget_minutes,
                cost: hand_over.budget_cost,
                plan_needs_approval: hand_over.plan_needs_approval,
            },
            state: work.state,
            open: work.is_open_under(hand_over, self.now()),
            brief,
            attempts,
            questions: question_views,
            reviews,
            gave_up: work.gave_up.clone(),
            stopped_for: work.stopped_for,
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
    // An agent sees its own work on the issue, never that of another agent.
    let only = actor.agent();
    let mut all = ctx
        .store()
        .work_on_issue(issue.id, only, MOST_SHOWN_ON_AN_ISSUE as u64 + 1)
        .await
        .map_err(failed)?;
    let mut truncated = all.len() > MOST_SHOWN_ON_AN_ISSUE;
    all.truncate(MOST_SHOWN_ON_AN_ISSUE);
    let views = ctx.work_views(&held, &all).await?;
    let mut open_agents: Vec<bb_core::id::AgentId> = Vec::new();
    for (found, view) in all.iter().zip(&views) {
        if view.open && !open_agents.contains(&found.agent) {
            open_agents.push(found.agent);
        }
    }
    let agents = ctx
        .store()
        .agents_by_ids(open_agents.clone())
        .await
        .map_err(failed)?;
    let mut worked_on_by: Vec<WorkedOnBy> = Vec::new();
    for id in open_agents {
        let agent = agents
            .iter()
            .find(|found| found.id == id)
            .ok_or(AppError::Refused(bb_core::ErrorCode::NotFound))?;
        worked_on_by.push(WorkedOnBy {
            agent: agent.id,
            name: agent.name.clone(),
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
    let work = views;
    let mut waiting = ctx
        .store()
        .proposals_on(
            issue.id,
            Some(ProposalState::Open),
            only,
            MOST_SHOWN_ON_AN_ISSUE as u64 + 1,
        )
        .await
        .map_err(failed)?;
    truncated |= waiting.len() > MOST_SHOWN_ON_AN_ISSUE;
    waiting.truncate(MOST_SHOWN_ON_AN_ISSUE);
    let proposals = ctx.proposal_views(&waiting).await?;
    let project = ctx
        .store()
        .project(issue.project)
        .await
        .map_err(failed)?
        .ok_or(AppError::Refused(bb_core::ErrorCode::NotFound))?;
    let hand_over_days = ctx.hand_over_days().await?;
    Ok(held.authorize(IssueAgentWork {
        hand_over_days,
        readable_id: bb_core::readable_id::ReadableId::new(project.short_name, issue.code.clone())
            .to_string(),
        worked_on_by,
        work,
        proposals,
        truncated,
        brief_missing: ctx
            .briefs_missing(&[(issue.clone(), Arc::clone(&held))])
            .await?
            .pop()
            .unwrap_or_default(),
    }))
}
