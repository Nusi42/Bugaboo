//! Handing issues over to an agent (AI-31, ADR-0029, decision 3).

use std::collections::BTreeSet;

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::agent_records::{GivingState, HandOver as HandOverRecord, IssueWork, RuleMatch};
use bb_core::agent_work::{
    AgentWorkState, BriefPart, MOST_ISSUES_OF_A_HAND_OVER, hand_over_end, issue_member,
    missing_in_brief,
};
use bb_core::event::{DomainEvent, EntityKind, RecordKind};
use bb_core::fields::FieldKey;
use bb_core::id::{AgentWorkId, EntityId, HandOverId, IssueId, ProjectId, StandingRuleId, UserId};
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
use crate::projects::Why;

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
    pub(super) async fn ensure_nameable(&self, fields: &[String]) -> Result<(), AppError> {
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
    pub(super) async fn briefs_missing(
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

/// Where a person may give agents work: the projects with *Create agents that work for you* and
/// the projects where agents are switched off. Read once for a hand-over, or once for a look of a
/// rule at what it found, never for each issue.
pub(super) struct Reach {
    may_create: BTreeSet<ProjectId>,
    off: BTreeSet<ProjectId>,
    /// The fields the hand-overs may let the agent change, checked once.
    fields: Vec<String>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Reads where the person may give agents work, and checks once that `fields` are members an
    /// agent can be given.
    ///
    /// # Errors
    /// `request.invalid` for a field no agent can be given, an internal error when the store
    /// fails.
    pub(super) async fn hand_over_reach(
        &self,
        person: UserId,
        fields: Vec<String>,
    ) -> Result<Reach, AppError> {
        self.ensure_nameable(&fields).await?;
        Ok(Reach {
            fields,
            may_create: self
                .projects_where(person, Permission::CreateAgents)
                .await?,
            off: self.projects_with_agents_off(person).await?,
        })
    }

    /// The work a hand-over gives, one piece for each distinct issue, and what each brief lacks.
    /// The person needs to change each issue and to create agents in its project, where agents
    /// are not switched off; one issue that fails stops the whole hand-over.
    async fn works_to_hand_over(
        &self,
        actor: &Actor,
        agent: &bb_core::agent::Agent,
        reach: &Reach,
        hand_over: HandOverId,
        addresses: &[String],
        now: jiff::Timestamp,
    ) -> Result<(Vec<IssueWork>, Vec<HandedIssue>), AppError> {
        let person = actor.person.id;
        let (may_create, off) = (&reach.may_create, &reach.off);
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
                project: issue.project,
                hand_over,
                agent: agent.id,
                person,
                state: AgentWorkState::WaitingForTheAgent,
                gave_up: None,
                stopped_for: None,
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

/// The event of a hand-over and the job of its end. A hand-over that a standing rule made names
/// the rule.
fn hand_over_event<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    hand_over: &HandOverRecord,
    works: &[IssueWork],
    rule: Option<StandingRuleId>,
) -> Result<(DomainEvent, QueuedJob), AppError> {
    let mut changes = vec![
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
    ];
    changes.extend(rule.map(|rule| set_to(member::RULE, json!(rule.to_string()))));
    let event = ctx.event(
        crate::auth::by(hand_over.person),
        entity(RecordKind::HandOver, hand_over.id),
        changes,
    )?;
    let job = ctx.queued(
        JobKind::HandOverRunsOut,
        json!({ payload::HAND_OVER: hand_over.id.to_string() }),
        hand_over.ends_at,
    )?;
    Ok((event, job))
}

/// What a hand-over is made of besides the issues and the agent: what a person fills in in the
/// window, and what a standing rule that hands over holds for each issue it finds.
pub(super) struct Giving {
    pub(super) note: String,
    pub(super) powers: Vec<bb_core::agent_work::HandOverPower>,
    /// The most attempts on one issue; the limit of the agent when none is named.
    pub(super) budget_attempts: Option<i64>,
    pub(super) budget_minutes: Option<i64>,
    pub(super) budget_cost: Option<i64>,
    pub(super) plan_needs_approval: bool,
    pub(super) accept_by_itself: bool,
    pub(super) days: Option<i64>,
}

/// A hand-over that a standing rule makes: the rule, and the most open pieces of work its agent
/// may have, which the hand-over is judged against under the lock of the work of the agent.
#[derive(Clone, Copy)]
pub(super) struct ByRule {
    pub(super) rule: StandingRuleId,
    pub(super) most_open: usize,
}

/// What a hand-over writes, apart from its event: owned, so that the change that writes it can
/// carry it.
#[derive(Clone)]
pub(super) struct HandOverWrites {
    hand_over: HandOverRecord,
    works: Vec<IssueWork>,
    job: QueuedJob,
    by_rule: Option<ByRule>,
    now: jiff::Timestamp,
    why: Why,
}

impl HandOverWrites {
    /// Writes the hand-over, its work and the job of its end inside the change. For a rule the
    /// issues are written down as found first, and the change stops with `agent.limit-reached`
    /// when the agent has as many open pieces of work as it may work on at once: judged on what is
    /// stored now, under the lock of the work of the agent, so of two looks at the rules at the
    /// same moment the second finds the room taken.
    pub(super) async fn apply<S: AuthStore>(
        &self,
        transaction: &mut S::Transaction,
    ) -> Result<(), S::Error> {
        let (stored, now) = (&self.hand_over, self.now);
        if let Some(by) = &self.by_rule {
            S::lock_working_of(transaction, stored.agent).await?;
            let open = S::open_work_count_of_in(transaction, stored.agent, now).await?;
            if usize::try_from(open).unwrap_or(usize::MAX) >= by.most_open {
                return Err(self.why.stop::<S>(ErrorCode::AgentLimitReached));
            }
            for work in &self.works {
                let found = RuleMatch {
                    rule: by.rule,
                    issue: work.issue,
                    at: now,
                };
                if !S::insert_match(transaction, &found).await? {
                    return Err(S::aborted());
                }
            }
        }
        for work in &self.works {
            if S::open_work_of_in(transaction, work.agent, work.issue)
                .await?
                .is_some()
            {
                return Err(S::aborted());
            }
        }
        S::insert_hand_over(transaction, stored).await?;
        for work in &self.works {
            S::insert_work(transaction, work).await?;
        }
        queue::<S>(transaction, &self.job, now).await
    }

    /// What to answer for a change that failed: the reason it stopped itself, an issue the agent
    /// has open work on, or the failure.
    pub(super) fn answer<S: AuthStore>(&self, error: &S::Error) -> AppError {
        match self.why.said() {
            Some(code) => AppError::Refused(code),
            None => change_failed::<S>(error, Stops::both(ErrorCode::AgentWorkOpen)),
        }
    }
}

/// A hand-over judged and ready to be written: its event, what it writes, and the answer.
pub(super) struct HandOverPlan {
    pub(super) event: DomainEvent,
    pub(super) writes: HandOverWrites,
    pub(super) view: HandOverView,
}

impl HandOverPlan {
    /// Writes the hand-over, its work and the job of its end in one change with its event; the
    /// answer when it is written. The one place that writes a hand-over, for a person and for a
    /// standing rule.
    ///
    /// # Errors
    /// `agent.limit-reached` for a rule whose agent has no room left, `agent-work.already-open`
    /// for an issue the agent has open work on, an internal error when the store fails.
    pub(super) async fn record_change<S: AuthStore>(
        self,
        store: &S,
    ) -> Result<HandOverView, AppError> {
        let writes = self.writes.clone();
        store
            .record_change(&self.event, move |transaction| {
                Box::pin(async move { writes.apply::<S>(transaction).await })
            })
            .await
            .map_err(|error| self.writes.answer::<S>(&error))?;
        Ok(self.view)
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Judges the hand-over of `issues` to `agent` for the person of `actor`: the one place that
    /// builds a hand-over, for the window of a person and for a standing rule that hands over what
    /// it finds. The agent must be the person's and able to act, its fields must be ones an agent
    /// can be given and the budget within its limits. The person needs *Change issues* (or *Change
    /// the issues they reported*) on each issue and *Create agents that work for you* in its
    /// project; an agent has one open hand-over for an issue. The plan is written with
    /// [`HandOverPlan::record_change`].
    ///
    /// # Errors
    /// `resource.not-found` for an issue they cannot see, `access.forbidden` where they may not
    /// hand over, `request.invalid` for an agent that cannot act for them, a budget or an end out of range.
    pub(super) async fn plan_hand_over(
        &self,
        actor: &Actor,
        agent: &bb_core::agent::Agent,
        reach: &Reach,
        giving: Giving,
        issues: &[String],
        by_rule: Option<ByRule>,
    ) -> Result<HandOverPlan, AppError> {
        let person = actor.person.id;
        if !agent.works_for(person) {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let budget_attempts = giving
            .budget_attempts
            .unwrap_or(agent.attempts_on_one_issue);
        if !(1..=agent.attempts_on_one_issue).contains(&budget_attempts)
            || giving.budget_minutes.is_some_and(|minutes| minutes < 1)
            || giving.budget_cost.is_some_and(|cost| cost < 0)
        {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let now = self.now();
        let days = match giving.days {
            Some(days) => days,
            None => self.hand_over_days().await?,
        };
        let ends_at = hand_over_end(now, Some(days))
            .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?;
        let id = HandOverId::generate(&mut self.ids());
        let (works, handed) = self
            .works_to_hand_over(actor, agent, reach, id, issues, now)
            .await?;
        let hand_over = HandOverRecord {
            id,
            agent: agent.id,
            person,
            note: giving.note,
            powers: giving.powers,
            fields: reach.fields.clone(),
            budget_attempts,
            budget_minutes: giving.budget_minutes,
            budget_cost: giving.budget_cost,
            plan_needs_approval: giving.plan_needs_approval,
            accept_by_itself: giving.accept_by_itself,
            state: GivingState::Open,
            ends_at,
            made_at: now,
            version: 1,
        };
        let (event, job) =
            hand_over_event(self, &hand_over, &works, by_rule.as_ref().map(|by| by.rule))?;
        Ok(HandOverPlan {
            event,
            writes: HandOverWrites {
                hand_over,
                works,
                job,
                by_rule,
                now,
                why: Why::default(),
            },
            view: HandOverView {
                id,
                agent: agent.id,
                issues: handed,
                ends_at,
                state: GivingState::Open,
                version: 1,
            },
        })
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
    let actor = ctx.session_actor(&input.session_token).await?;
    let person = actor.person.id;
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
    let giving = Giving {
        note: note_of(&input.note)?,
        powers: input.powers,
        budget_attempts: input.budget_attempts,
        budget_minutes: input.budget_minutes,
        budget_cost: input.budget_cost,
        plan_needs_approval: input.plan_needs_approval,
        accept_by_itself: input.accept_by_itself,
        days: input.days,
    };
    let reach = ctx.hand_over_reach(person, input.fields).await?;
    let view = ctx
        .plan_hand_over(&actor, &agent, &reach, giving, &input.issues, None)
        .await?
        .record_change(ctx.store())
        .await?;
    Ok(ctx
        .evaluate(person, Place::Organization)
        .await?
        .authorize(view))
}
