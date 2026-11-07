//! Handing issues over to an agent (AI-31, ADR-0029, decision 3).

use std::collections::BTreeSet;

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::agent_records::{GivingState, HandOver as HandOverRecord, IssueWork};
use bb_core::agent_work::{
    AgentWorkState, BriefPart, MOST_ISSUES_OF_A_HAND_OVER, hand_over_end, issue_member,
    missing_in_brief,
};
use bb_core::event::{DomainEvent, EntityKind, RecordKind};
use bb_core::fields::FieldKey;
use bb_core::id::{AgentWorkId, EntityId, HandOverId, IssueId};
use bb_core::issue::Issue;
use bb_core::own_fields::FieldFor;
use bb_policy::{Authorized, Permission, Place};
use serde_json::json;

use super::members::{member, payload};
use super::queue::queue;
use super::types::{HandOver, HandOverView, HandedIssue};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, entity, internal, set_to};
use crate::input::clean_long_text;
use crate::jobs::{JobKind, QueuedJob};

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
    /// Refuses a hand-over that names a member an agent can never be given: one that is no member
    /// of an issue, an own field that is retired or not for issues, and any field that holds
    /// people (COMP-10).
    async fn ensure_nameable(&self, fields: &[String]) -> Result<(), AppError> {
        for field in fields {
            if issue_member::NAMEABLE.contains(&field.as_str()) {
                continue;
            }
            let usable = self.own_field_named(field).await?.is_some_and(|found| {
                found.field.applies_to == FieldFor::Issues
                    && found.field.retired_at.is_none()
                    && !found.field.kind.holds_people()
            });
            if !usable {
                return Err(ErrorCode::InvalidRequest.into());
            }
        }
        Ok(())
    }

    /// What the brief lacks for each issue: the description, *Steps to reproduce* and *Done when*,
    /// each when it is empty and the reader may see it. It is told and never stops a hand-over.
    /// The values of all the issues are read together (PERF-N1).
    async fn briefs_missing(
        &self,
        issues: &[(Issue, std::sync::Arc<bb_policy::Evaluation>)],
    ) -> Result<Vec<Vec<BriefPart>>, AppError> {
        let catalogue = self.field_catalogue().await?;
        let kind = EntityKind::from(RecordKind::Issue);
        let values = self
            .store()
            .field_values(
                kind,
                issues
                    .iter()
                    .map(|(issue, _)| EntityId::from(issue.id))
                    .collect(),
            )
            .await
            .map_err(|error| internal::<S>(&error))?;
        let named = |wanted: &str| {
            catalogue
                .iter()
                .filter(|definition| definition.field.name.trim().eq_ignore_ascii_case(wanted))
                .map(|definition| definition.field.id)
                .collect::<Vec<_>>()
        };
        let (steps, done_when) = (named(STEPS_FIELD), named(DONE_WHEN_FIELD));
        Ok(issues
            .iter()
            .map(|(issue, held)| {
                // The text of the fields of that name the reader may see; none when they see none.
                let text_of = |fields: &[bb_core::id::FieldId]| -> Option<String> {
                    let seen: Vec<_> = fields
                        .iter()
                        .filter(|field| {
                            held.field_level_for(Some(issue.issue_type), FieldKey::Own(**field))
                                >= FieldLevel::CanSee
                        })
                        .collect();
                    // No such field at all: the brief lacks it. One the reader may not see: unknown.
                    (seen.is_empty() == fields.is_empty()).then(|| {
                        values
                            .iter()
                            .filter(|held| {
                                held.entity == EntityId::from(issue.id)
                                    && seen.contains(&&held.field)
                            })
                            .flat_map(|held| held.cells.iter().filter_map(|cell| cell.text.clone()))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                };
                missing_in_brief(
                    &issue.description,
                    text_of(&steps).as_deref(),
                    text_of(&done_when).as_deref(),
                )
            })
            .collect())
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The work a hand-over gives, one piece for each distinct issue, and what each brief lacks.
    /// The person needs to change each issue and to create agents in its project, where agents
    /// are not switched off; one issue that fails stops the whole hand-over.
    async fn works_to_hand_over(
        &self,
        actor: &Actor,
        agent: &bb_core::agent::Agent,
        hand_over: HandOverId,
        addresses: &[String],
        now: jiff::Timestamp,
    ) -> Result<(Vec<IssueWork>, Vec<HandedIssue>), AppError> {
        let person = actor.person.id;
        let may_create = self
            .projects_where(person, Permission::CreateAgents)
            .await?;
        let off = self.projects_with_agents_off(person).await?;
        let mut works: Vec<IssueWork> = Vec::new();
        let mut issues = Vec::new();
        let mut seen: BTreeSet<IssueId> = BTreeSet::new();
        for address in addresses {
            let (issue, held) = self.visible_issue(actor, address).await?;
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
                id: AgentWorkId::generate(&mut self.ids()),
                issue: issue.id,
                hand_over,
                agent: agent.id,
                person,
                state: AgentWorkState::WaitingForTheAgent,
                gave_up: None,
                closed: false,
                accepted_by_itself: false,
                status_pending: false,
                made_at: now,
                version: 1,
            };
            works.push(work);
            issues.push((issue, held));
        }
        let missing = self.briefs_missing(&issues).await?;
        let handed = works
            .iter()
            .zip(missing)
            .map(|(work, missing)| HandedIssue {
                work: work.id,
                issue: work.issue,
                missing,
            })
            .collect();
        Ok((works, handed))
    }
}

/// The event of a hand-over and the job of its end.
fn hand_over_event<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    hand_over: &HandOverRecord,
    works: &[IssueWork],
) -> Result<(DomainEvent, QueuedJob), AppError> {
    let event = ctx.event(
        crate::auth::by(hand_over.person),
        entity(RecordKind::HandOver, hand_over.id),
        vec![
            set_to(member::AGENT, json!(hand_over.agent.to_string())),
            set_to(
                member::ISSUES,
                json!(
                    works
                        .iter()
                        .map(|w| w.issue.to_string())
                        .collect::<Vec<_>>()
                ),
            ),
            set_to(member::ENDS_AT, json!(hand_over.ends_at.to_string())),
            set_to(bb_core::version::FIELD, json!(1)),
        ],
    )?;
    let job = ctx.queued(
        JobKind::HandOverRunsOut,
        json!({ payload::HAND_OVER: hand_over.id.to_string() }),
        hand_over.ends_at,
    )?;
    Ok((event, job))
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
    ctx.ensure_nameable(&input.fields).await?;
    let note = note_of(&input.note)?;
    let now = ctx.now();
    let days = match input.days {
        Some(days) => days,
        None => ctx.hand_over_days().await?,
    };
    let ends_at =
        hand_over_end(now, Some(days)).map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?;
    let id = HandOverId::generate(&mut ctx.ids());
    let (works, handed) = ctx
        .works_to_hand_over(&actor, &agent, id, &input.issues, now)
        .await?;
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
        state: GivingState::Open,
        ends_at,
        made_at: now,
        version: 1,
    };
    let (event, job) = hand_over_event(ctx, &hand_over, &works)?;
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
        state: GivingState::Open,
        version: 1,
    };
    Ok(ctx
        .evaluate(person, Place::Organization)
        .await?
        .authorize(view))
}
