//! Standing rules (AI-32, ADR-0029, decision 3): a saved search an agent works on. A rule proposes
//! by default, or hands over each issue it finds (`rules_hand_over.rs`); it never changes an issue
//! by itself. It runs out after 30 days unless set otherwise (at most 90, or never where the
//! organization allows it) and is renewed with one click.

use bb_core::ErrorCode;
use bb_core::agent_records::{GivingState, Proposal, ProposalState, StandingRule as RuleRecord};
use bb_core::agent_work::{EndError, RuleField, RuleWay, rule_end_asked, rule_warning_at};
use bb_core::event::RecordKind;
use bb_core::id::{AgentNoteId, StandingRuleId};
use bb_core::paging::{Cut, page_len, read_len};
use bb_policy::{Authorized, Permission, Place};
use serde_json::json;

use super::members::{member, payload};
use super::queue::queue;
use super::store::RuleCursor;
use super::types::{
    ChangeStandingRule, CreateStandingRule, DecideProposal, ProposalView, ProposeChange,
    StandingRulePage, StandingRuleView, StandingRulesQuery,
};
use crate::access::Actor;
use crate::auth::nothing_if_aborted;
use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, entity, internal, set_to};
use crate::input::cursor_or_invalid;
use crate::issues::{ChangeIssue, change_issue};
use crate::jobs::{JobKind, QueuedJob};
use crate::tags::{AddLabel, AddTag, add_label, add_tag};

pub(super) fn view_of(rule: &RuleRecord) -> StandingRuleView {
    StandingRuleView {
        id: rule.id,
        agent: rule.agent,
        search: rule.search,
        project: rule.project,
        way: rule.way,
        powers: rule.powers.clone(),
        fields: rule.fields.clone(),
        accept_by_itself: rule.accept_by_itself,
        creates_in: rule.creates_in,
        state: rule.state,
        ends_at: rule.ends_at,
        version: rule.version,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The jobs that go with the end of a rule: it runs out, and three days before it tells.
    pub(super) fn rule_jobs(&self, rule: &RuleRecord) -> Result<Vec<QueuedJob>, AppError> {
        // A rule that never runs out has no end to run out at, and nothing to warn of.
        let Some(ends_at) = rule.ends_at else {
            return Ok(Vec::new());
        };
        let end = json!({
            payload::RULE: rule.id.to_string(),
            payload::ENDS_AT: ends_at.to_string(),
        });
        let mut jobs = vec![self.queued(JobKind::RuleRunsOut, end.clone(), ends_at)?];
        if let Some(at) = rule_warning_at(ends_at).filter(|at| *at > self.now()) {
            jobs.push(self.queued(JobKind::RuleAboutToRunOut, end, at)?);
        }
        Ok(jobs)
    }

    /// The end of a rule as its maker asked for it: the days (the usual number of the organization
    /// when none are named) or none at all where the organization allows rules that never run out.
    async fn end_asked(
        &self,
        days: Option<i64>,
        never: bool,
    ) -> Result<Option<jiff::Timestamp>, AppError> {
        let days = match (days, never) {
            (None, false) => Some(self.rule_days().await?),
            (days, _) => days,
        };
        let allowed = self.rules_never_run_out().await?;
        rule_end_asked(self.now(), days, never, allowed).map_err(|error| match error {
            EndError::NotAllowed => AppError::Refused(ErrorCode::Forbidden),
            EndError::OutOfRange => AppError::Refused(ErrorCode::InvalidRequest),
        })
    }

    /// Refuses a rule that could not be made now, for the making and for a renewal: the agent is
    /// the person's and works, the person has *Create agents that work for you* in the project of
    /// the rule (and in the project it creates in) where agents are on, and can open the saved
    /// search.
    async fn ensure_rule_may_work(&self, actor: &Actor, rule: &RuleRecord) -> Result<(), AppError> {
        let agent = self
            .store()
            .agent(rule.agent)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|found| found.person == actor.person.id)
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        if !agent.may_act() {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let allowed = self
            .projects_where(agent.person, Permission::CreateAgents)
            .await?;
        let off = self.projects_with_agents_off(agent.person).await?;
        let in_reach =
            |project: bb_core::id::ProjectId| allowed.contains(&project) && !off.contains(&project);
        if !in_reach(rule.project) || !rule.creates_in.is_none_or(in_reach) {
            return Err(ErrorCode::Forbidden.into());
        }
        // A rule that proposes holds nothing of a hand-over; one that hands over names members an
        // agent can be given (COMP-10: never a field that holds people).
        match rule.way {
            RuleWay::Propose => {
                if !rule.powers.is_empty() || !rule.fields.is_empty() || rule.accept_by_itself {
                    return Err(ErrorCode::InvalidRequest.into());
                }
            }
            RuleWay::HandOver => self.ensure_nameable(&rule.fields).await?,
        }
        // The person must be able to open the search; anybody else's is one answer for all.
        self.open_search(actor, rule.search).await?;
        Ok(())
    }

    /// The rule, when the person made it: anybody else, an administrator too, is told there is no
    /// such rule. (An administrator may read the rules of an agent, not change them.)
    async fn rule_of(
        &self,
        person: bb_core::id::UserId,
        id: StandingRuleId,
    ) -> Result<RuleRecord, AppError> {
        self.store()
            .standing_rule(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|rule| rule.person == person)
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }
}

/// Makes a standing rule for one of the person's agents over a saved search they may open. The
/// person needs *Create agents that work for you* in the project of the rule.
///
/// # Errors
/// `resource.not-found` for an agent that is not the person's, a search they cannot open or a
/// project they do not see, `access.forbidden` where they may not give agents work,
/// `request.invalid` for an end beyond 90 days or powers on a rule that proposes, `access.forbidden`
/// for a rule that never runs out where the organization does not allow it.
pub async fn create_standing_rule<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateStandingRule,
) -> Result<Authorized<StandingRuleView>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let person = actor.person.id;
    let now = ctx.now();
    let ends_at = ctx.end_asked(input.days, input.never_ends).await?;
    let mut fields = input.fields;
    fields.sort();
    fields.dedup();
    let mut powers = input.powers;
    powers.sort();
    powers.dedup();
    let rule = RuleRecord {
        id: StandingRuleId::generate(&mut ctx.ids()),
        agent: input.agent,
        person,
        search: input.search,
        project: input.project,
        way: input.way,
        powers,
        fields,
        accept_by_itself: input.accept_by_itself,
        creates_in: input.creates_in,
        state: GivingState::Open,
        ends_at,
        made_at: now,
        version: 1,
    };
    ctx.ensure_rule_may_work(&actor, &rule).await?;
    let mut jobs = ctx.rule_jobs(&rule)?;
    jobs.push(ctx.queued(
        JobKind::RuleMatches,
        json!({ payload::PROJECT: rule.project.to_string() }),
        now,
    )?);
    let event = ctx.event(
        crate::auth::by(person),
        entity(RecordKind::StandingRule, rule.id),
        vec![
            set_to(member::AGENT, json!(rule.agent.to_string())),
            set_to(member::SEARCH, json!(rule.search.to_string())),
            set_to(member::PROJECT, json!(rule.project.to_string())),
            set_to(member::WAY, json!(rule.way.code())),
            set_to(member::FIELDS, json!(rule.fields)),
            set_to(member::ENDS_AT, json!(ends_at.map(|end| end.to_string()))),
            set_to(bb_core::version::FIELD, json!(1)),
        ],
    )?;
    let stored = rule.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::insert_standing_rule(transaction, &stored).await?;
                for job in &jobs {
                    queue::<S>(transaction, job, now).await?;
                }
                Ok(())
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    Ok(ctx
        .evaluate(person, Place::Organization)
        .await?
        .authorize(view_of(&rule)))
}

/// Renews a standing rule (a new end, 30 days from now unless set, at most 90, or none where the
/// organization allows it) or stops it.
///
/// # Errors
/// `resource.not-found` for a rule that is not the person's, `request.invalid` for an end beyond
/// 90 days or a rule that ended, `edit.version-changed` when the rule changed meanwhile.
pub async fn change_standing_rule<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeStandingRule,
) -> Result<Authorized<StandingRuleView>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let person = actor.person.id;
    let found = ctx.rule_of(person, input.rule).await?;
    if found.version != input.version {
        return Err(AppError::stale(
            entity(RecordKind::StandingRule, found.id),
            Place::Organization,
            person,
        ));
    }
    if found.state != GivingState::Open {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let now = ctx.now();
    if input.renew {
        ctx.ensure_rule_may_work(&actor, &found).await?;
    }
    let (ends_at, ended) = if input.renew {
        (ctx.end_asked(input.days, input.never_ends).await?, false)
    } else {
        (Some(found.ends_at.map_or(now, |end| end.min(now))), true)
    };
    let next = RuleRecord {
        ends_at,
        state: if ended {
            GivingState::Ended
        } else {
            GivingState::Open
        },
        version: found.version + 1,
        ..found.clone()
    };
    let jobs = if ended {
        Vec::new()
    } else {
        ctx.rule_jobs(&next)?
    };
    let event = ctx.event(
        crate::auth::by(person),
        entity(RecordKind::StandingRule, found.id),
        vec![
            set_to(member::STATE, json!(next.state.code())),
            set_to(member::ENDS_AT, json!(ends_at.map(|end| end.to_string()))),
            bb_core::version::counted_up(found.version),
        ],
    )?;
    let (id, version) = (found.id, found.version);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if !S::set_rule_end(transaction, id, ends_at, ended, version).await? {
                    return Err(S::aborted());
                }
                for job in &jobs {
                    queue::<S>(transaction, job, now).await?;
                }
                Ok(())
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)))?;
    Ok(ctx
        .evaluate(person, Place::Organization)
        .await?
        .authorize(view_of(&next)))
}

/// The standing rules of one of the person's agents, a page at a time. An administrator reads those
/// of any agent.
///
/// # Errors
/// `resource.not-found` for an agent that is not the person's (and the reader no administrator),
/// `request.invalid` for a cursor that is not one.
pub async fn standing_rules<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: StandingRulesQuery,
) -> Result<Authorized<StandingRulePage>, AppError> {
    let person = ctx.session_actor(&input.session_token).await?.person.id;
    let administrator = ctx.administers_agents(person).await?;
    ctx.store()
        .agent(input.agent)
        .await
        .map_err(|error| internal::<S>(&error))?
        .filter(|found| found.person == person || administrator)
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let len = page_len(Some(input.limit));
    let after = cursor_or_invalid(input.cursor.as_deref(), RuleCursor::parse)?;
    let page = Cut::of(
        ctx.store()
            .rules_of_agent(input.agent, after, read_len(len))
            .await
            .map_err(|error| internal::<S>(&error))?,
        len,
    );
    let next_cursor = page.next_cursor(|last| RuleCursor::of(last).text());
    let items = page.items.iter().map(view_of).collect();
    Ok(ctx
        .evaluate(person, Place::Organization)
        .await?
        .authorize(StandingRulePage { items, next_cursor }))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// A proposal with the name of its agent.
    pub(super) async fn proposal_view(
        &self,
        proposal: &Proposal,
    ) -> Result<ProposalView, AppError> {
        self.proposal_views(std::slice::from_ref(proposal))
            .await?
            .pop()
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }

    /// Judges a proposal of an agent under one of its open rules: the one place that decides if it
    /// may be made, for the use case and for its preview.
    async fn plan_proposal(
        &self,
        input: &ProposeChange,
    ) -> Result<(Actor, Proposal, std::sync::Arc<bb_policy::Evaluation>), AppError> {
        let actor = self.actor(&input.session_token).await?;
        let Some(agent) = actor.agent() else {
            return Err(ErrorCode::Forbidden.into());
        };
        let (found, held) = self.visible_issue(&actor, &input.issue).await?;
        let rule = self
            .store()
            .standing_rule(input.rule)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|rule| rule.agent == agent)
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        if !rule.works_at(self.now()) {
            return Err(ErrorCode::AgentWorkNotReady.into());
        }
        // A rule that hands over gives the agent the issue itself: it changes it in its attempts.
        if rule.way != RuleWay::Propose {
            return Err(ErrorCode::Forbidden.into());
        }
        if !self.chain_of(found.project).await?.contains(&rule.project) {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        // Only an issue the rule found: the agent reads others only through a hand-over.
        if self
            .store()
            .known_matches(rule.id, vec![found.id])
            .await
            .map_err(|error| internal::<S>(&error))?
            .is_empty()
        {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        let before = match input.field {
            RuleField::Type => json!(found.issue_type.to_string()),
            _ => serde_json::Value::Null,
        };
        let proposal = Proposal {
            id: AgentNoteId::generate(&mut self.ids()),
            rule: rule.id,
            agent,
            issue: found.id,
            field: input.field,
            before,
            after: input.after.clone(),
            state: ProposalState::Open,
            decided_by: None,
            made_at: self.now(),
        };
        Ok((actor, proposal, held))
    }

    /// What a proposal would record, without saving anything: the field, the value now and the
    /// value proposed.
    ///
    /// # Errors
    /// What `propose_change` refuses.
    pub(crate) async fn preview_propose_change(
        &self,
        input: &ProposeChange,
    ) -> Result<Vec<bb_core::event::FieldChange>, AppError> {
        let (_, proposal, _) = self.plan_proposal(input).await?;
        Ok(vec![bb_core::event::FieldChange {
            field: proposal.field.code().to_owned(),
            before: (!proposal.before.is_null()).then_some(proposal.before),
            after: Some(proposal.after),
        }])
    }
}

/// An agent proposes a change under one of its open rules to an issue the rule found. Nothing
/// changes: the proposal is a record on the issue's agent work until a person applies or discards
/// it.
///
/// # Errors
/// `resource.not-found` for a rule that is not the agent's or an issue the rule did not find,
/// `agent-work.not-ready` for a rule that ended, `request.invalid` for a value that cannot be.
pub async fn propose_change<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ProposeChange,
) -> Result<Authorized<ProposalView>, AppError> {
    let (actor, proposal, held) = ctx.plan_proposal(&input).await?;
    let event = ctx.event(
        crate::auth::by(actor.person.id),
        entity(RecordKind::StandingRule, proposal.rule),
        vec![
            set_to(member::ISSUE, json!(proposal.issue.to_string())),
            set_to(member::PROPOSAL, json!(proposal.id.to_string())),
        ],
    )?;
    let stored = proposal.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::insert_proposal(transaction, &stored).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    Ok(held.authorize(ctx.proposal_view(&proposal).await?))
}

/// Applies or discards a proposal. Applying changes the issue as the person, through the one
/// change of an issue; nothing changed before.
///
/// # Errors
/// `resource.not-found` for a proposal of a rule that is not the person's, `agent-work.not-ready`
/// for one that was decided, and the refusals of the change itself.
pub async fn decide_proposal<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DecideProposal,
) -> Result<Authorized<ProposalView>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let person = actor.person.id;
    let proposal = ctx
        .store()
        .proposal(input.proposal)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let rule = ctx.rule_of(person, proposal.rule).await?;
    if proposal.state != ProposalState::Open {
        return Err(ErrorCode::AgentWorkNotReady.into());
    }
    let state = if input.apply {
        ProposalState::Applied
    } else {
        ProposalState::Discarded
    };
    let event = ctx.event(
        crate::auth::by(person),
        entity(RecordKind::StandingRule, rule.id),
        vec![
            set_to(member::ISSUE, json!(proposal.issue.to_string())),
            set_to(member::PROPOSAL, json!(proposal.id.to_string())),
            set_to(member::STATE, json!(state.code())),
        ],
    )?;
    let id = proposal.id;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::set_proposal_state(transaction, id, state, person).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::AgentWorkNotReady)))?;
    // The proposal is claimed: of two decisions at the same moment one is told it was decided.
    // What it asks for is done after, and a proposal that cannot be carried out is open again.
    if input.apply
        && let Err(error) = ctx.apply_proposal(&input.session_token, &proposal).await
    {
        ctx.reopen_proposal(&rule, &proposal, person).await?;
        return Err(error);
    }
    let mut decided = proposal;
    decided.state = state;
    decided.decided_by = Some(person);
    let (_, held) = ctx
        .visible_issue(&actor, &decided.issue.to_string())
        .await?;
    Ok(held.authorize(ctx.proposal_view(&decided).await?))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Opens a proposal again that was claimed by `person` and could not be carried out.
    pub(crate) async fn reopen_proposal(
        &self,
        rule: &RuleRecord,
        proposal: &Proposal,
        person: bb_core::id::UserId,
    ) -> Result<(), AppError> {
        let event = self.event(
            crate::auth::by(person),
            entity(RecordKind::StandingRule, rule.id),
            vec![
                set_to(member::ISSUE, json!(proposal.issue.to_string())),
                set_to(member::PROPOSAL, json!(proposal.id.to_string())),
                set_to(member::STATE, json!(ProposalState::Open.code())),
            ],
        )?;
        let id = proposal.id;
        // The event is written with the change, and only when a proposal that this person claimed
        // was opened: a proposal that is not (any more) claimed by them aborts the change and
        // writes nothing.
        let result = self
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    if S::reopen_proposal(transaction, id, person).await? {
                        Ok(())
                    } else {
                        Err(S::aborted())
                    }
                })
            })
            .await;
        nothing_if_aborted::<S>(result)
    }

    /// Applies a proposal as the person, through the use case that changes that part of an issue
    /// (the one the assistant's *Apply* ends in, too).
    async fn apply_proposal(&self, token: &str, proposal: &Proposal) -> Result<(), AppError> {
        match proposal.field {
            RuleField::Type => {
                let actor = self.actor(token).await?;
                let (current, _) = self
                    .visible_issue(&actor, &proposal.issue.to_string())
                    .await?;
                let issue_type = proposal
                    .after
                    .as_str()
                    .and_then(|text| text.parse().ok())
                    .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
                change_issue(
                    self,
                    ChangeIssue::issue_type(
                        token,
                        proposal.issue.to_string(),
                        current.version,
                        issue_type,
                    ),
                )
                .await
                .map(|_| ())
            }
            // The use cases of the labels and tags of an issue, as the person.
            RuleField::Labels => {
                let label = proposal
                    .after
                    .as_str()
                    .and_then(|text| text.parse().ok())
                    .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
                add_label(
                    self,
                    AddLabel {
                        session_token: token.to_owned(),
                        issue: proposal.issue.to_string(),
                        label,
                    },
                )
                .await
                .map(|_| ())
            }
            RuleField::Tags => {
                let name = proposal
                    .after
                    .as_str()
                    .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
                add_tag(
                    self,
                    AddTag {
                        session_token: token.to_owned(),
                        issue: proposal.issue.to_string(),
                        name: name.to_owned(),
                    },
                )
                .await
                .map(|_| ())
            }
            // *Found in release* is not stored on an issue yet, and *Duplicate of* is a relation of
            // a kind the organization names: neither is applied before they are (M7-F72).
            RuleField::DuplicateOf | RuleField::FoundInRelease => {
                Err(ErrorCode::InvalidRequest.into())
            }
        }
    }
}
