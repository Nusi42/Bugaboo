//! Handing issues over to an agent (AI-31, ADR-0029, decision 3).

use std::collections::BTreeSet;

use bb_core::ErrorCode;
use bb_core::agent_records::{GrantState, HandOver as HandOverRecord, IssueWork};
use bb_core::agent_work::{
    AgentWorkState, BriefPart, MOST_ISSUES_OF_A_HAND_OVER, hand_over_end, missing_in_brief,
};
use bb_core::event::{EntityKind, RecordKind};
use bb_core::id::{AgentWorkId, EntityId, HandOverId, IssueId};
use bb_core::issue::Issue;
use bb_policy::{Authorized, Permission, Place};
use serde_json::json;

use super::members::{member, payload};
use super::queue::queue;
use super::types::{HandOver, HandOverView, HandedIssue};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, entity, internal, set_to};
use crate::input::clean_long_text;
use crate::jobs::JobKind;

/// The names of the fields a brief takes from the issue, as people name them.
const STEPS_FIELD: &str = "steps to reproduce";
const DONE_WHEN_FIELD: &str = "done when";

/// The note of a brief: empty is allowed.
fn note_of(text: &str) -> Result<String, AppError> {
    if text.trim().is_empty() {
        Ok(String::new())
    } else {
        clean_long_text(text)
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What the brief lacks for an issue: the description, *Steps to reproduce* and *Done when*,
    /// each when it is empty. It is told and never stops a hand-over.
    pub(crate) async fn brief_missing(&self, issue: &Issue) -> Result<Vec<BriefPart>, AppError> {
        let catalogue = self.field_catalogue().await?;
        let kind = EntityKind::from(RecordKind::Issue);
        let values = self
            .store()
            .field_values(kind, vec![EntityId::from(issue.id)])
            .await
            .map_err(|error| internal::<S>(&error))?;
        let text_of = |wanted: &str| -> String {
            catalogue
                .iter()
                .filter(|definition| definition.field.name.trim().eq_ignore_ascii_case(wanted))
                .flat_map(|definition| {
                    values
                        .iter()
                        .filter(move |held| held.field == definition.field.id)
                })
                .flat_map(|held| held.cells.iter().filter_map(|cell| cell.text.clone()))
                .collect::<Vec<_>>()
                .join("\n")
        };
        Ok(missing_in_brief(
            &issue.description,
            &text_of(STEPS_FIELD),
            &text_of(DONE_WHEN_FIELD),
        ))
    }
}

/// Hands one or several issues over to one of the person's agents. The person needs *Change
/// issues* (or *Change the issues they reported*) on each issue and *Create agents that work for
/// you* in its project; an agent has one open hand-over for an issue.
///
/// # Errors
/// `resource.not-found` for an agent that is not the person's or an issue they cannot see,
/// `access.forbidden` where they may not hand over, `agent-work.already-open` for an issue the
/// agent has already, `request.invalid` for a list of issues that is empty or too long, a budget or
/// an end out of range.
pub async fn hand_over<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: HandOver,
) -> Result<Authorized<HandOverView>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let actor = Actor {
        person: record.person,
        key: None,
    };
    if input.issues.is_empty() || input.issues.len() > MOST_ISSUES_OF_A_HAND_OVER {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let agent = ctx
        .store()
        .agent(input.agent)
        .await
        .map_err(|error| internal::<S>(&error))?
        .filter(|found| found.person == person)
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    if !agent.may_act() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let budget_attempts = input.budget_attempts.unwrap_or(agent.attempts_on_one_issue);
    if !(1..=agent.attempts_on_one_issue).contains(&budget_attempts)
        || input.budget_minutes.is_some_and(|minutes| minutes < 1)
        || input.budget_cost.is_some_and(|cost| cost < 0)
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let note = note_of(&input.note)?;
    let now = ctx.now();
    let ends_at =
        hand_over_end(now, input.days).map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?;
    let may_create = ctx.projects_where(person, Permission::CreateAgents).await?;
    let off = ctx.projects_with_agents_off(person).await?;
    let id = HandOverId::generate(&mut ctx.ids());
    let mut works: Vec<IssueWork> = Vec::new();
    let mut handed: Vec<HandedIssue> = Vec::new();
    let mut seen: BTreeSet<IssueId> = BTreeSet::new();
    for address in &input.issues {
        let (issue, held) = ctx.visible_issue(&actor, address).await?;
        if !seen.insert(issue.id) {
            continue;
        }
        if !held.may_change_issue(issue.reported_by)
            || !may_create.contains(&issue.project)
            || off.contains(&issue.project)
        {
            return Err(ErrorCode::Forbidden.into());
        }
        let work = IssueWork {
            id: AgentWorkId::generate(&mut ctx.ids()),
            issue: issue.id,
            hand_over: id,
            agent: agent.id,
            person,
            state: AgentWorkState::WaitingForTheAgent,
            gave_up: None,
            accepted_by_itself: false,
            made_at: now,
            version: 1,
        };
        handed.push(HandedIssue {
            work: work.id,
            issue: issue.id,
            missing: ctx.brief_missing(&issue).await?,
        });
        works.push(work);
    }
    let hand_over = HandOverRecord {
        id,
        agent: agent.id,
        person,
        note,
        powers: input.powers,
        fields: input.fields,
        budget_attempts,
        budget_minutes: input.budget_minutes,
        budget_cost: input.budget_cost,
        plan_needs_approval: input.plan_needs_approval,
        accept_by_itself: input.accept_by_itself,
        state: GrantState::Open,
        ends_at,
        made_at: now,
        version: 1,
    };
    let event = ctx.event(
        crate::auth::by(person),
        entity(RecordKind::HandOver, id),
        vec![
            set_to(member::AGENT, json!(agent.id.to_string())),
            set_to(
                member::ISSUES,
                json!(
                    works
                        .iter()
                        .map(|w| w.issue.to_string())
                        .collect::<Vec<_>>()
                ),
            ),
            set_to(member::ENDS_AT, json!(ends_at.to_string())),
            set_to(bb_core::version::FIELD, json!(1)),
        ],
    )?;
    let job = ctx.queued(
        JobKind::HandOverRunsOut,
        json!({ payload::HAND_OVER: id.to_string() }),
        ends_at,
    )?;
    let stored = hand_over.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                for work in &works {
                    if S::open_work_of_in(transaction, work.agent, work.issue)
                        .await?
                        .is_some()
                    {
                        return Err(S::aborted());
                    }
                }
                S::insert_hand_over(transaction, &stored).await?;
                for work in &works {
                    S::insert_work(transaction, work).await?;
                }
                queue::<S>(transaction, &job, now).await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::AgentWorkOpen)))?;
    let view = HandOverView {
        id,
        agent: agent.id,
        issues: handed,
        ends_at,
        state: GrantState::Open,
        version: 1,
    };
    Ok(ctx
        .evaluate(person, Place::Organization)
        .await?
        .authorize(view))
}
