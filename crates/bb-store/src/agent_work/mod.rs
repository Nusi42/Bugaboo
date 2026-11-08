//! The adapter of the ports of agent work (`bb_app::agent_work`): hand-overs, standing rules, the
//! work on an issue and what belongs to it (AI-31 to AI-35, ADR-0029). `grants` holds hand-overs,
//! rules and proposals; `work` the work on an issue.

mod grants;
mod work;

use bb_app::agents::{AgentWork, AgentWorkStore};
use bb_core::agent_records::{
    Attempt, AttemptResult, GivingState, HandOver, IssueWork, Proposal, ProposalState, Question,
    ResultKind, Review, RuleMatch, StandingRule,
};
use bb_core::agent_work::{AgentWorkState, Allowance, CheckResult, HandOverPower, ReviewAnswer};
use bb_core::id::{
    AgentId, AgentNoteId, AgentWorkId, AttemptId, HandOverId, IssueId, ProjectId, SavedSearchId,
    StandingRuleId, UserId,
};
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
    let mut open = Query::select();
    open.column(HandOverRow::Id)
        .from(HandOverRow::Table)
        .and_where(Expr::col(HandOverRow::State).eq(GivingState::Open.code()));
    select
        .and_where(Expr::col((IssueWorkRow::Table, IssueWorkRow::Closed)).eq(false))
        .and_where(
            Expr::col((IssueWorkRow::Table, IssueWorkRow::State))
                .is_not_in(AgentWorkState::ENDED.map(AgentWorkState::code)),
        )
        .and_where(Expr::col((IssueWorkRow::Table, IssueWorkRow::HandOver)).in_subquery(open));
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
        .and_where(Expr::col((IssueWorkRow::Table, IssueWorkRow::Agent)).eq(agent.stored()))
        .and_where(Expr::col((HandOverRow::Table, HandOverRow::EndsAt)).gt(to_micros(now)));
    only_open_work(&mut select);
    select
}

/// Which standing rules of an agent work at `now`: the one definition, for the rules it reads and
/// for the issues they found.
fn working_rule(agent: AgentId, now: Timestamp) -> sea_query::Cond {
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
        asked_at: row.time(QuestionRow::AskedAt)?,
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
    /// The issues of the open work of the agent, and of the issues its open rules found, and the
    /// projects of both and of the rules.
    async fn agent_work(&self, agent: AgentId, now: Timestamp) -> Result<AgentWork, StoreError> {
        self.read_agent_work(agent, now).await
    }
}

impl AuthStorage {
    /// Several reads one after the other, each on its own: the work of an agent is no snapshot,
    /// and a change between them is seen on the next request.
    async fn read_agent_work(
        &self,
        agent: AgentId,
        now: Timestamp,
    ) -> Result<AgentWork, StoreError> {
        let mut work = AgentWork::default();
        let mut open = open_work_of_agent(agent, now);
        open.column((IssueRow::Table, IssueRow::Id))
            .column((IssueRow::Table, IssueRow::Project))
            .column((HandOverRow::Table, HandOverRow::Powers))
            .column((HandOverRow::Table, HandOverRow::Fields))
            .inner_join(
                IssueRow::Table,
                Expr::col((IssueRow::Table, IssueRow::Id))
                    .equals((IssueWorkRow::Table, IssueWorkRow::Issue)),
            );
        for row in self.database().fetch_all(&open).await? {
            let issue = row.id::<IssueId>(IssueRow::Id)?;
            work.issues.insert(issue);
            work.projects
                .insert(row.id::<ProjectId>(IssueRow::Project)?);
            let powers: Vec<HandOverPower> =
                serde_json::from_str(&row.get::<String>(HandOverRow::Powers)?)?;
            let fields: Vec<String> =
                serde_json::from_str(&row.get::<String>(HandOverRow::Fields)?)?;
            work.allowances
                .entry(issue)
                .or_default()
                .add(Allowance::of_hand_over(&powers, &fields));
        }
        // The rules that work and the issues they found, in two reads whatever their number.
        let rules = open_rules_of_agent(agent, now);
        let rules: Vec<StandingRule> = self
            .database()
            .fetch_all(&rules)
            .await?
            .iter()
            .map(read_rule)
            .collect::<Result<_, _>>()?;
        for rule in &rules {
            work.projects.insert(rule.project);
            work.creates_in.extend(rule.creates_in);
        }
        if rules.is_empty() {
            return Ok(work);
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
            );
        for row in self.database().fetch_all(&found).await? {
            let rule = row.id::<StandingRuleId>(RuleMatchRow::Rule)?;
            let issue = row.id::<IssueId>(RuleMatchRow::Issue)?;
            let Some(rule) = rules.iter().find(|held| held.id == rule) else {
                continue;
            };
            work.issues.insert(issue);
            work.allowances
                .entry(issue)
                .or_default()
                .add(Allowance::of_rule(&rule.fields));
        }
        Ok(work)
    }
}
