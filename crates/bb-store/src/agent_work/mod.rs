//! The adapter of the ports of agent work (`bb_app::agent_work`): hand-overs, standing rules, the
//! work on an issue and what belongs to it (AI-31 to AI-35, ADR-0029). `grants` holds hand-overs,
//! rules and proposals; `work` the work on an issue.

mod grants;
#[cfg(any(test, feature = "testing"))]
mod stored;
mod work;

use bb_app::agents::{AgentReach, AgentWorkStore, Hold};
use bb_core::agent_records::{
    Attempt, AttemptResult, GivingState, HandOver, IssueWork, Proposal, ProposalState, Question,
    ResultKind, Review, RuleMatch, StandingRule,
};
use bb_core::agent_work::{
    AgentWorkState, Allowance, CheckResult, HandOverPower, ReviewAnswer, StopCause,
};
use bb_core::id::{
    AgentId, AgentNoteId, AgentWorkId, AttemptId, HandOverId, IssueId, ProjectId, SavedSearchId,
    StandingRuleId, UserId,
};
use std::collections::BTreeMap;

use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::locks::{LockClass, lock_key};
use crate::schema::{
    AttemptResultRow, AttemptRow, Computed, HandOverRow, IssueRow, IssueWorkRow, ProposalRow,
    QuestionRow, ReviewRow, RuleMatchRow, StandingRuleRow,
};

/// Keeps the work that is open: its state is not an end, no review closed it, and its hand-over
/// still gives work (an accepted hand-over ended, and its work with it).
pub(super) fn only_open_work(select: &mut SelectStatement) {
    select
        .and_where(Expr::col((IssueWorkRow::Table, IssueWorkRow::Closed)).eq(false))
        .and_where(
            Expr::col((IssueWorkRow::Table, IssueWorkRow::State))
                .is_not_in(AgentWorkState::ENDED.map(AgentWorkState::code)),
        )
        .and_where(
            Expr::col((IssueWorkRow::Table, IssueWorkRow::HandOver)).in_subquery(open_hand_overs()),
        );
}

/// The hand-overs that are open: the one definition of a hand-over that still gives work, before its
/// end is looked at.
fn open_hand_overs() -> SelectStatement {
    let mut open = Query::select();
    open.column(HandOverRow::Id)
        .from(HandOverRow::Table)
        .and_where(Expr::col(HandOverRow::State).eq(GivingState::Open.code()));
    open
}

/// Keeps the work that is open at `now` ([`only_open_work`]) and whose hand-over has not run out: a
/// hand-over past its end gives no work, though its job did not end it yet. The one definition of
/// *open at `now`*, for the work of an agent and for the work on issues.
pub(super) fn only_open_work_at(select: &mut SelectStatement, now: Timestamp) {
    only_open_work(select);
    select.and_where(
        Expr::col((IssueWorkRow::Table, IssueWorkRow::HandOver))
            .in_subquery(hand_overs_giving_work_at(now)),
    );
}

/// The hand-overs that still give work at `now`: open, and not run out.
pub(super) fn hand_overs_giving_work_at(now: Timestamp) -> SelectStatement {
    let mut giving = open_hand_overs();
    giving.and_where(Expr::col(HandOverRow::EndsAt).gt(to_micros(now)));
    giving
}

/// The open work of an agent at `now`, joined to its hand-over: the one definition of the work an
/// agent reaches, for what it may read and change and for what its searches find.
pub(super) fn open_work_of_agent(agent: AgentId, now: Timestamp) -> SelectStatement {
    let mut select = Query::select();
    select
        .from(IssueWorkRow::Table)
        .inner_join(
            HandOverRow::Table,
            Expr::col((HandOverRow::Table, HandOverRow::Id))
                .equals((IssueWorkRow::Table, IssueWorkRow::HandOver)),
        )
        .and_where(Expr::col((IssueWorkRow::Table, IssueWorkRow::Agent)).eq(agent.stored()));
    only_open_work_at(&mut select, now);
    select
}

/// Which standing rules of an agent work at `now`: the one definition, for the rules it reads and
/// for the issues they found.
pub(super) fn working_rule(agent: AgentId, now: Timestamp) -> sea_query::Cond {
    let column = |column: StandingRuleRow| Expr::col((StandingRuleRow::Table, column));
    sea_query::Cond::all()
        .add(column(StandingRuleRow::Agent).eq(agent.stored()))
        .add(column(StandingRuleRow::State).eq(GivingState::Open.code()))
        .add(column(StandingRuleRow::EndsAt).gt(to_micros(now)))
}

/// The standing rules of an agent that work at `now`.
pub(super) fn open_rules_of_agent(agent: AgentId, now: Timestamp) -> SelectStatement {
    let mut rules = rule_select();
    rules.cond_where(working_rule(agent, now));
    rules
}

/// The issues an agent reaches at `now`: those of its open work and those its open rules found.
pub(crate) fn issues_of_agent(agent: AgentId, now: Timestamp) -> sea_query::Cond {
    let mut work = open_work_of_agent(agent, now);
    work.column((IssueWorkRow::Table, IssueWorkRow::Issue));
    let mut found = Query::select();
    found
        .column((RuleMatchRow::Table, RuleMatchRow::Issue))
        .from(RuleMatchRow::Table)
        .inner_join(
            StandingRuleRow::Table,
            Expr::col((StandingRuleRow::Table, StandingRuleRow::Id))
                .equals((RuleMatchRow::Table, RuleMatchRow::Rule)),
        )
        .cond_where(working_rule(agent, now));
    sea_query::Cond::any()
        .add(Expr::col((IssueRow::Table, IssueRow::Id)).in_subquery(work))
        .add(Expr::col((IssueRow::Table, IssueRow::Id)).in_subquery(found))
}

pub(super) fn word<T: bb_core::words::Word>(
    row: &Row,
    column: impl sea_query::Iden + Copy,
) -> Result<T, StoreError> {
    T::from_code(&row.get::<String>(column)?).ok_or(StoreError::Corrupt("a word of agent work"))
}

pub(super) fn hand_over_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            HandOverRow::Id,
            HandOverRow::Agent,
            HandOverRow::Person,
            HandOverRow::Note,
            HandOverRow::Powers,
            HandOverRow::Fields,
            HandOverRow::BudgetAttempts,
            HandOverRow::BudgetMinutes,
            HandOverRow::BudgetCost,
            HandOverRow::PlanNeedsApproval,
            HandOverRow::AcceptByItself,
            HandOverRow::State,
            HandOverRow::EndsAt,
            HandOverRow::MadeAt,
            HandOverRow::Version,
        ])
        .from(HandOverRow::Table);
    select
}

pub(super) fn read_hand_over(row: &Row) -> Result<HandOver, StoreError> {
    Ok(HandOver {
        id: row.id(HandOverRow::Id)?,
        agent: row.id(HandOverRow::Agent)?,
        person: row.id(HandOverRow::Person)?,
        note: row.get(HandOverRow::Note)?,
        powers: serde_json::from_str(&row.get::<String>(HandOverRow::Powers)?)?,
        fields: serde_json::from_str(&row.get::<String>(HandOverRow::Fields)?)?,
        budget_attempts: row.get(HandOverRow::BudgetAttempts)?,
        budget_minutes: row.get(HandOverRow::BudgetMinutes)?,
        budget_cost: row.get(HandOverRow::BudgetCost)?,
        plan_needs_approval: row.get(HandOverRow::PlanNeedsApproval)?,
        accept_by_itself: row.get(HandOverRow::AcceptByItself)?,
        state: word::<GivingState>(row, HandOverRow::State)?,
        ends_at: row.time(HandOverRow::EndsAt)?,
        made_at: row.time(HandOverRow::MadeAt)?,
        version: row.get(HandOverRow::Version)?,
    })
}

pub(super) fn rule_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            StandingRuleRow::Id,
            StandingRuleRow::Agent,
            StandingRuleRow::Person,
            StandingRuleRow::Search,
            StandingRuleRow::Project,
            StandingRuleRow::Fields,
            StandingRuleRow::CreatesIn,
            StandingRuleRow::State,
            StandingRuleRow::EndsAt,
            StandingRuleRow::MadeAt,
            StandingRuleRow::Version,
        ])
        .from(StandingRuleRow::Table);
    select
}

pub(super) fn read_rule(row: &Row) -> Result<StandingRule, StoreError> {
    Ok(StandingRule {
        id: row.id(StandingRuleRow::Id)?,
        agent: row.id(StandingRuleRow::Agent)?,
        person: row.id(StandingRuleRow::Person)?,
        search: row.id::<SavedSearchId>(StandingRuleRow::Search)?,
        project: row.id(StandingRuleRow::Project)?,
        fields: serde_json::from_str(&row.get::<String>(StandingRuleRow::Fields)?)?,
        creates_in: row.optional_id(StandingRuleRow::CreatesIn)?,
        state: word(row, StandingRuleRow::State)?,
        ends_at: row.time(StandingRuleRow::EndsAt)?,
        made_at: row.time(StandingRuleRow::MadeAt)?,
        version: row.get(StandingRuleRow::Version)?,
    })
}

pub(super) fn work_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            IssueWorkRow::Id,
            IssueWorkRow::Issue,
            IssueWorkRow::HandOver,
            IssueWorkRow::Agent,
            IssueWorkRow::Person,
            IssueWorkRow::State,
            IssueWorkRow::GaveUp,
            IssueWorkRow::StoppedFor,
            IssueWorkRow::Closed,
            IssueWorkRow::AcceptedByItself,
            IssueWorkRow::StatusPending,
            IssueWorkRow::MadeAt,
            IssueWorkRow::Version,
        ])
        .from(IssueWorkRow::Table);
    select
}

pub(super) fn read_work(row: &Row) -> Result<IssueWork, StoreError> {
    Ok(IssueWork {
        id: row.id(IssueWorkRow::Id)?,
        issue: row.id(IssueWorkRow::Issue)?,
        hand_over: row.id(IssueWorkRow::HandOver)?,
        agent: row.id(IssueWorkRow::Agent)?,
        person: row.id(IssueWorkRow::Person)?,
        state: word(row, IssueWorkRow::State)?,
        gave_up: row.get(IssueWorkRow::GaveUp)?,
        stopped_for: row
            .get::<Option<String>>(IssueWorkRow::StoppedFor)?
            .map(|code| StopCause::from_code(&code).ok_or(StoreError::Corrupt("why work stopped")))
            .transpose()?,
        closed: row.get(IssueWorkRow::Closed)?,
        accepted_by_itself: row.get(IssueWorkRow::AcceptedByItself)?,
        status_pending: row.get(IssueWorkRow::StatusPending)?,
        made_at: row.time(IssueWorkRow::MadeAt)?,
        version: row.get(IssueWorkRow::Version)?,
    })
}

pub(super) fn attempt_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            AttemptRow::Id,
            AttemptRow::Work,
            AttemptRow::Number,
            AttemptRow::Plan,
            AttemptRow::PlanApproved,
            AttemptRow::ReportedMinutes,
            AttemptRow::ReportedCost,
            AttemptRow::StartedAt,
            AttemptRow::EndedAt,
            AttemptRow::Run,
        ])
        .from(AttemptRow::Table);
    select
}

pub(super) fn read_attempt(row: &Row) -> Result<Attempt, StoreError> {
    Ok(Attempt {
        id: row.id(AttemptRow::Id)?,
        work: row.id(AttemptRow::Work)?,
        number: row.get(AttemptRow::Number)?,
        plan: row.get(AttemptRow::Plan)?,
        plan_approved: row.get(AttemptRow::PlanApproved)?,
        reported_minutes: row.get(AttemptRow::ReportedMinutes)?,
        reported_cost: row.get(AttemptRow::ReportedCost)?,
        started_at: row.time(AttemptRow::StartedAt)?,
        ended_at: row.optional_time(AttemptRow::EndedAt)?,
        run: row.optional_id(AttemptRow::Run)?,
    })
}

pub(super) fn question_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            QuestionRow::Id,
            QuestionRow::Work,
            QuestionRow::Text,
            QuestionRow::Answer,
            QuestionRow::AnsweredBy,
            QuestionRow::AnsweredAt,
            QuestionRow::AskedAt,
        ])
        .from(QuestionRow::Table);
    select
}

pub(super) fn read_question(row: &Row) -> Result<Question, StoreError> {
    Ok(Question {
        id: row.id(QuestionRow::Id)?,
        work: row.id(QuestionRow::Work)?,
        text: row.get(QuestionRow::Text)?,
        answer: row.get(QuestionRow::Answer)?,
        answered_by: row.optional_id(QuestionRow::AnsweredBy)?,
        answered_at: row.optional_time(QuestionRow::AnsweredAt)?,
        asked_at: row.time(QuestionRow::AskedAt)?,
    })
}

/// The columns of the results of attempts.
pub(super) fn results_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            AttemptResultRow::Id,
            AttemptResultRow::Attempt,
            AttemptResultRow::Kind,
            AttemptResultRow::Label,
            AttemptResultRow::Link,
            AttemptResultRow::Outcome,
        ])
        .from(AttemptResultRow::Table);
    select
}

/// Reads one result of an attempt.
pub(super) fn read_result(row: &Row) -> Result<AttemptResult, StoreError> {
    let outcome = row
        .get::<Option<String>>(AttemptResultRow::Outcome)?
        .map(|code| CheckResult::from_code(&code).ok_or(StoreError::Corrupt("a check result")))
        .transpose()?;
    Ok(AttemptResult {
        id: row.id(AttemptResultRow::Id)?,
        attempt: row.id(AttemptResultRow::Attempt)?,
        kind: word::<ResultKind>(row, AttemptResultRow::Kind)?,
        label: row.get(AttemptResultRow::Label)?,
        link: row.get(AttemptResultRow::Link)?,
        outcome,
    })
}

/// The columns of the reviews of work.
pub(super) fn reviews_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            ReviewRow::Id,
            ReviewRow::Work,
            ReviewRow::Answer,
            ReviewRow::Note,
            ReviewRow::By,
            ReviewRow::At,
        ])
        .from(ReviewRow::Table);
    select
}

/// Reads one review.
pub(super) fn read_review(row: &Row) -> Result<Review, StoreError> {
    Ok(Review {
        id: row.id(ReviewRow::Id)?,
        work: row.id(ReviewRow::Work)?,
        answer: word::<ReviewAnswer>(row, ReviewRow::Answer)?,
        note: row.get(ReviewRow::Note)?,
        by: row.optional_id(ReviewRow::By)?,
        at: row.time(ReviewRow::At)?,
    })
}

pub(super) fn proposal_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            ProposalRow::Id,
            ProposalRow::Rule,
            ProposalRow::Agent,
            ProposalRow::Issue,
            ProposalRow::Field,
            ProposalRow::Before,
            ProposalRow::After,
            ProposalRow::State,
            ProposalRow::DecidedBy,
            ProposalRow::MadeAt,
        ])
        .from(ProposalRow::Table);
    select
}

pub(super) fn read_proposal(row: &Row) -> Result<Proposal, StoreError> {
    Ok(Proposal {
        id: row.id(ProposalRow::Id)?,
        rule: row.id(ProposalRow::Rule)?,
        agent: row.id(ProposalRow::Agent)?,
        issue: row.id(ProposalRow::Issue)?,
        field: word(row, ProposalRow::Field)?,
        before: serde_json::from_str(&row.get::<String>(ProposalRow::Before)?)?,
        after: serde_json::from_str(&row.get::<String>(ProposalRow::After)?)?,
        state: word(row, ProposalRow::State)?,
        decided_by: row.optional_id(ProposalRow::DecidedBy)?,
        made_at: row.time(ProposalRow::MadeAt)?,
    })
}

pub(super) async fn all<T>(
    transaction: &mut Transaction,
    select: &SelectStatement,
    read: fn(&Row) -> Result<T, StoreError>,
) -> Result<Vec<T>, StoreError> {
    transaction
        .fetch_all(select)
        .await?
        .iter()
        .map(read)
        .collect()
}

pub(super) async fn first<T>(
    transaction: &mut Transaction,
    select: &SelectStatement,
    read: fn(&Row) -> Result<T, StoreError>,
) -> Result<Option<T>, StoreError> {
    transaction
        .fetch_optional(select)
        .await?
        .as_ref()
        .map(read)
        .transpose()
}

/// Counts the version of a row up if it is at `version`; whether it was.
pub(super) async fn update_at(
    transaction: &mut Transaction,
    table: impl sea_query::Iden + 'static,
    id_column: impl sea_query::Iden + Copy + 'static,
    id: Vec<u8>,
    version_column: impl sea_query::Iden + Copy + 'static,
    version: i64,
    set: impl FnOnce(&mut sea_query::UpdateStatement),
) -> Result<bool, StoreError> {
    let mut update = Query::update();
    update
        .table(table)
        .value(version_column, version + 1)
        .and_where(Expr::col(id_column).eq(id))
        .and_where(Expr::col(version_column).eq(version));
    set(&mut update);
    Ok(transaction.execute(&update).await? > 0)
}

pub(super) fn after<I: StoredId>(
    mut select: SelectStatement,
    column: impl sea_query::Iden + Copy + 'static,
    after: Option<I>,
    limit: u64,
) -> SelectStatement {
    if let Some(after) = after {
        select.and_where(Expr::col(column).gt(after.stored()));
    }
    select.order_by(column, Order::Asc).limit(limit);
    select
}

impl AgentWorkStore for AuthStorage {
    /// The projects of the open work of the agent and of its open rules, and the projects its
    /// rules create issues in: no issue is listed, however many the agent holds.
    async fn agent_reach(&self, agent: AgentId, now: Timestamp) -> Result<AgentReach, StoreError> {
        let mut reach = AgentReach::default();
        let mut open = open_work_of_agent(agent, now);
        open.distinct()
            .column((IssueRow::Table, IssueRow::Project))
            .inner_join(
                IssueRow::Table,
                Expr::col((IssueRow::Table, IssueRow::Id))
                    .equals((IssueWorkRow::Table, IssueWorkRow::Issue)),
            );
        for row in self.database().fetch_all(&open).await? {
            reach
                .projects
                .insert(row.id::<ProjectId>(IssueRow::Project)?);
        }
        for row in self
            .database()
            .fetch_all(&open_rules_of_agent(agent, now))
            .await?
        {
            let rule = read_rule(&row)?;
            reach.projects.insert(rule.project);
            reach.creates_in.extend(rule.creates_in);
        }
        Ok(reach)
    }

    /// What the agent holds on each of these issues: its open work on it (handed, with the ticks of
    /// the hand-over) and the open rules that found it (their fields), in two reads whatever the
    /// number of issues, rules and matches.
    async fn agent_holds(
        &self,
        agent: AgentId,
        now: Timestamp,
        issues: Vec<IssueId>,
    ) -> Result<BTreeMap<IssueId, Hold>, StoreError> {
        let mut holds: BTreeMap<IssueId, Hold> = BTreeMap::new();
        if issues.is_empty() {
            return Ok(holds);
        }
        let named: Vec<Vec<u8>> = issues.iter().map(|issue| issue.stored()).collect();
        let mut open = open_work_of_agent(agent, now);
        open.column((IssueWorkRow::Table, IssueWorkRow::Issue))
            .column((HandOverRow::Table, HandOverRow::Powers))
            .column((HandOverRow::Table, HandOverRow::Fields))
            .and_where(Expr::col((IssueWorkRow::Table, IssueWorkRow::Issue)).is_in(named.clone()));
        for row in self.database().fetch_all(&open).await? {
            let powers: Vec<HandOverPower> =
                serde_json::from_str(&row.get::<String>(HandOverRow::Powers)?)?;
            let fields: Vec<String> =
                serde_json::from_str(&row.get::<String>(HandOverRow::Fields)?)?;
            let hold = holds
                .entry(row.id::<IssueId>(IssueWorkRow::Issue)?)
                .or_default();
            hold.handed = true;
            hold.allowance
                .add(Allowance::of_hand_over(&powers, &fields));
        }
        let rules: Vec<StandingRule> = self
            .database()
            .fetch_all(&open_rules_of_agent(agent, now))
            .await?
            .iter()
            .map(read_rule)
            .collect::<Result<_, _>>()?;
        if rules.is_empty() {
            return Ok(holds);
        }
        let mut found = Query::select();
        found
            .column(RuleMatchRow::Rule)
            .column(RuleMatchRow::Issue)
            .from(RuleMatchRow::Table)
            .and_where(
                Expr::col(RuleMatchRow::Rule).is_in(
                    rules
                        .iter()
                        .map(|rule| rule.id.stored())
                        .collect::<Vec<_>>(),
                ),
            )
            .and_where(Expr::col(RuleMatchRow::Issue).is_in(named));
        for row in self.database().fetch_all(&found).await? {
            let rule = row.id::<StandingRuleId>(RuleMatchRow::Rule)?;
            let Some(rule) = rules.iter().find(|held| held.id == rule) else {
                continue;
            };
            holds
                .entry(row.id::<IssueId>(RuleMatchRow::Issue)?)
                .or_default()
                .allowance
                .add(Allowance::of_rule(&rule.fields));
        }
        Ok(holds)
    }
}
