//! Standing rules (AI-32, ADR-0029, decision 3): a saved search an agent works on with narrow
//! powers. A rule proposes by default; it changes only the fields it names from a short list, and
//! never a field that holds people and never the status. It runs out after 30 days unless set
//! otherwise (at most 90) and is renewed with one click.

use bb_core::ErrorCode;
use bb_core::agent_records::{
    GivingState, Proposal, ProposalState, RuleMatch, StandingRule as RuleRecord,
};
use bb_core::agent_work::{RuleField, rule_end, rule_warning_at};
use bb_core::event::{Actor as EventActor, RecordKind};
use bb_core::id::{AgentNoteId, IssueId, StandingRuleId};
use bb_core::paging::{Cut, page_len, read_len};
use bb_policy::{Authorized, Permission, Place};
use serde_json::json;

use super::members::{member, payload};
use super::queue::queue;
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
use crate::search::SearchScope;
use crate::tags::{AddLabel, AddTag, add_label, add_tag};

/// How many issues one look at a rule reads at once.
const MATCH_PAGE: u32 = 100;

fn view_of(rule: &RuleRecord) -> StandingRuleView {
    StandingRuleView {
        id: rule.id,
        agent: rule.agent,
        search: rule.search,
        project: rule.project,
        fields: rule.fields.clone(),
        creates_in: rule.creates_in,
        state: rule.state,
        ends_at: rule.ends_at,
        version: rule.version,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The jobs that go with the end of a rule: it runs out, and three days before it tells.
    fn rule_jobs(&self, rule: &RuleRecord) -> Result<Vec<QueuedJob>, AppError> {
        let end = json!({
            payload::RULE: rule.id.to_string(),
            payload::ENDS_AT: rule.ends_at.to_string(),
        });
        let mut jobs = vec![self.queued(JobKind::RuleRunsOut, end.clone(), rule.ends_at)?];
        if let Some(at) = rule_warning_at(rule.ends_at).filter(|at| *at > self.now()) {
            jobs.push(self.queued(JobKind::RuleAboutToRunOut, end, at)?);
        }
        Ok(jobs)
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
        // The person must be able to open the search; anybody else's is one answer for all.
        self.open_search(actor, rule.search).await?;
        Ok(())
    }

    /// The rule, when the person made it (or an administrator may see it): anybody else is told
    /// there is no such rule.
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
/// `request.invalid` for an end beyond 90 days.
pub async fn create_standing_rule<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateStandingRule,
) -> Result<Authorized<StandingRuleView>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let person = actor.person.id;
    let now = ctx.now();
    let days = match input.days {
        Some(days) => days,
        None => ctx.rule_days().await?,
    };
    let ends_at =
        rule_end(now, Some(days)).map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?;
    let mut fields = input.fields;
    fields.sort();
    fields.dedup();
    let rule = RuleRecord {
        id: StandingRuleId::generate(&mut ctx.ids()),
        agent: input.agent,
        person,
        search: input.search,
        project: input.project,
        fields,
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
            set_to(
                member::FIELDS,
                json!(rule.fields.iter().map(|f| f.code()).collect::<Vec<_>>()),
            ),
            set_to(member::ENDS_AT, json!(ends_at.to_string())),
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

/// Renews a standing rule (a new end, 30 days from now unless set, at most 90) or stops it.
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
        let days = match input.days {
            Some(days) => days,
            None => ctx.rule_days().await?,
        };
        (
            rule_end(now, Some(days)).map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?,
            false,
        )
    } else {
        (found.ends_at.min(now), true)
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
            set_to(member::ENDS_AT, json!(ends_at.to_string())),
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

/// The standing rules of one of the person's agents, a page at a time.
///
/// # Errors
/// `resource.not-found` for an agent that is not the person's, `request.invalid` for a cursor that
/// is not one.
pub async fn standing_rules<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: StandingRulesQuery,
) -> Result<Authorized<StandingRulePage>, AppError> {
    let person = ctx.session_actor(&input.session_token).await?.person.id;
    ctx.store()
        .agent(input.agent)
        .await
        .map_err(|error| internal::<S>(&error))?
        .filter(|found| found.person == person)
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let len = page_len(Some(input.limit));
    let after = cursor_or_invalid(input.cursor.as_deref(), |cursor| {
        cursor.parse::<StandingRuleId>().ok()
    })?;
    let page = Cut::of(
        ctx.store()
            .rules_of_agent(input.agent, after, read_len(len))
            .await
            .map_err(|error| internal::<S>(&error))?,
        len,
    );
    let next_cursor = page.next_cursor(|last| last.id.to_string());
    let items = page.items.iter().map(view_of).collect();
    Ok(ctx
        .evaluate(person, Place::Organization)
        .await?
        .authorize(StandingRulePage { items, next_cursor }))
}

impl<S: AuthStore> Ctx<'_, S> {
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
        if rule.state != GivingState::Open || rule.ends_at <= self.now() {
            return Err(ErrorCode::AgentWorkNotReady.into());
        }
        if !self.chain_of(found.project).await?.contains(&rule.project) {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        // A rule that names fields proposes about those; one that names none proposes about any
        // field of the short list.
        if !rule.fields.is_empty() && !rule.fields.contains(&input.field) {
            return Err(ErrorCode::Forbidden.into());
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
    Ok(held.authorize(proposal_view(&proposal)))
}

fn proposal_view(proposal: &Proposal) -> ProposalView {
    ProposalView {
        id: proposal.id,
        rule: proposal.rule,
        agent: proposal.agent,
        field: proposal.field,
        before: proposal.before.clone(),
        after: proposal.after.clone(),
        state: proposal.state,
        made_at: proposal.made_at,
    }
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
    Ok(held.authorize(proposal_view(&decided)))
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

    /// The work of the job of a rule that ran out: ends the rule if it is the end the job was
    /// queued for. A rule that was renewed, stopped or is not there leaves nothing to do.
    ///
    /// # Errors
    /// `request.invalid` for a payload that names no rule, an internal error when the store fails.
    pub(crate) async fn run_rule_runs_out(&self, job: &serde_json::Value) -> Result<(), AppError> {
        let Some(rule) = self.rule_of_job(job).await? else {
            return Ok(());
        };
        if rule.ends_at > self.now() {
            return Ok(());
        }
        let event = self.event(
            EventActor::System,
            entity(RecordKind::StandingRule, rule.id),
            vec![
                set_to(member::STATE, json!(GivingState::Ended.code())),
                bb_core::version::counted_up(rule.version),
            ],
        )?;
        let (id, ends_at, version) = (rule.id, rule.ends_at, rule.version);
        let result = self
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    if S::set_rule_end(transaction, id, ends_at, true, version).await? {
                        Ok(())
                    } else {
                        Err(S::aborted())
                    }
                })
            })
            .await;
        nothing_if_aborted::<S>(result)
    }

    /// The work of the job three days before a rule runs out: writes the event the notification
    /// *A standing rule runs out soon* is built from, once, if the rule still ends then.
    ///
    /// # Errors
    /// `request.invalid` for a payload that names no rule, an internal error when the store fails.
    pub(crate) async fn run_rule_about_to_run_out(
        &self,
        job: &serde_json::Value,
    ) -> Result<(), AppError> {
        let Some(rule) = self.rule_of_job(job).await? else {
            return Ok(());
        };
        let event = self.event(
            EventActor::System,
            entity(RecordKind::StandingRule, rule.id),
            vec![
                set_to(member::STATE, json!("about-to-run-out")),
                set_to(member::AGENT, json!(rule.agent.to_string())),
                set_to(member::ENDS_AT, json!(rule.ends_at.to_string())),
            ],
        )?;
        self.store()
            .record_change(&event, move |_| Box::pin(async move { Ok(()) }))
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// The rule a job names, when it is still open and still ends when the job was queued for.
    async fn rule_of_job(&self, job: &serde_json::Value) -> Result<Option<RuleRecord>, AppError> {
        let id: StandingRuleId = job
            .get(payload::RULE)
            .and_then(serde_json::Value::as_str)
            .and_then(|text| text.parse().ok())
            .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
        let ends: Option<jiff::Timestamp> = job
            .get(payload::ENDS_AT)
            .and_then(serde_json::Value::as_str)
            .and_then(|text| text.parse().ok());
        Ok(self
            .store()
            .standing_rule(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|rule| rule.state == GivingState::Open && ends == Some(rule.ends_at)))
    }

    /// The work of the job that writes down what newly came under the rules of a project: it runs
    /// each open rule's saved search as the person who made the rule, and writes down each issue
    /// that is new to the rule, with the moment. An issue the agent may not read is not written
    /// down.
    ///
    /// # Errors
    /// `request.invalid` for a payload that names no project, an internal error when the store
    /// fails.
    pub(crate) async fn run_rule_matches(&self, job: &serde_json::Value) -> Result<(), AppError> {
        let project: bb_core::id::ProjectId = job
            .get(payload::PROJECT)
            .and_then(serde_json::Value::as_str)
            .and_then(|text| text.parse().ok())
            .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
        let rules = self
            .store()
            .open_rules_in(project)
            .await
            .map_err(|error| internal::<S>(&error))?;
        for rule in rules.into_iter().filter(|rule| rule.ends_at > self.now()) {
            self.match_rule(&rule).await?;
        }
        Ok(())
    }

    async fn match_rule(&self, rule: &RuleRecord) -> Result<(), AppError> {
        let failed = |error| internal::<S>(&error);
        let Some(person) = self.person_known(rule.person).await? else {
            return Ok(());
        };
        let agent_on = self.store().agent(rule.agent).await.map_err(failed)?;
        if !agent_on.is_some_and(|agent| agent.may_act() && agent.person == rule.person) {
            return Ok(());
        }
        let actor = Actor { person, key: None };
        let opened = self.open_search(&actor, rule.search).await;
        let Ok(opened) = opened else {
            return Ok(());
        };
        let text = opened.stored.search.text.clone();
        let mut prepared = self
            .prepare_search(&actor, &text, SearchScope::Within(rule.project))
            .await?;
        let mut fresh: Vec<IssueId> = Vec::new();
        prepared.request.limit = u64::from(MATCH_PAGE);
        loop {
            let hits = self
                .store()
                .search_page(&prepared.request)
                .await
                .map_err(crate::search::stopped)?;
            let last = hits.last().map(|hit| hit.issue.id);
            // Which of this page the rule knows already: read for the page, never for all.
            let page: Vec<IssueId> = hits.iter().map(|hit| hit.issue.id).collect();
            let known = self
                .store()
                .known_matches(rule.id, page.clone())
                .await
                .map_err(failed)?;
            fresh.extend(page.into_iter().filter(|issue| !known.contains(issue)));
            match last {
                Some(last) if hits.len() as u64 >= prepared.request.limit => {
                    prepared.request.after = Some(last);
                }
                _ => break,
            }
        }
        let now = self.now();
        if fresh.is_empty() {
            return Ok(());
        }
        let event = self.event(
            EventActor::System,
            entity(RecordKind::StandingRule, rule.id),
            vec![set_to(
                member::ISSUES,
                json!(fresh.iter().map(ToString::to_string).collect::<Vec<_>>()),
            )],
        )?;
        let rule_id = rule.id;
        self.store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    for issue in &fresh {
                        S::insert_match(
                            transaction,
                            &RuleMatch {
                                rule: rule_id,
                                issue: *issue,
                                at: now,
                            },
                        )
                        .await?;
                    }
                    Ok(())
                })
            })
            .await
            .map_err(failed)
    }
}
