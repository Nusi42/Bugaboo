//! Migration 42: hand-overs, standing rules and agent work (AI-31 to AI-35, ADR-0029).
//!
//! - `hand_over` gives issues to an agent; `issue_work` is the work on one issue of it, with its
//!   `agent_attempt`s, their `agent_result`s (links and checks), the `agent_question`s and the
//!   `agent_review`s.
//! - `standing_rule` lets an agent work on a saved search; `agent_proposal` holds what it
//!   proposes and `rule_match` each issue that came under the rule, once.
//! - Lists inside a record (powers, fields) are JSON text. Moments are microseconds.
//!
//! Nothing is written into the tables.

use sea_query::{ColumnDef, Index, Table};

use super::{Step, id_column};
use crate::schema::{
    AttemptResultRow, AttemptRow, HandOverRow, IssueWorkRow, ProposalRow, QuestionRow, ReviewRow,
    RuleMatchRow, StandingRuleRow,
};

fn text(name: impl sea_query::Iden + 'static) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.text().not_null();
    column
}

fn number(name: impl sea_query::Iden + 'static) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.big_integer().not_null();
    column
}

fn flag(name: impl sea_query::Iden + 'static) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.boolean().not_null();
    column
}

fn maybe_text(name: impl sea_query::Iden + 'static) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.text();
    column
}

fn maybe_number(name: impl sea_query::Iden + 'static) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.big_integer();
    column
}

fn maybe_id(name: impl sea_query::Iden + 'static) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.blob();
    column
}

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(HandOverRow::Table)
                .col(id_column(HandOverRow::Id).primary_key())
                .col(id_column(HandOverRow::Agent))
                .col(id_column(HandOverRow::Person))
                .col(text(HandOverRow::Note))
                .col(text(HandOverRow::Powers))
                .col(text(HandOverRow::Fields))
                .col(number(HandOverRow::BudgetAttempts))
                .col(maybe_number(HandOverRow::BudgetMinutes))
                .col(maybe_number(HandOverRow::BudgetCost))
                .col(flag(HandOverRow::PlanNeedsApproval))
                .col(flag(HandOverRow::AcceptByItself))
                .col(text(HandOverRow::State))
                .col(number(HandOverRow::EndsAt))
                .col(number(HandOverRow::MadeAt))
                .col(number(HandOverRow::Version))
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("hand_over_by_agent")
                .table(HandOverRow::Table)
                .col(HandOverRow::Agent)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(StandingRuleRow::Table)
                .col(id_column(StandingRuleRow::Id).primary_key())
                .col(id_column(StandingRuleRow::Agent))
                .col(id_column(StandingRuleRow::Person))
                .col(id_column(StandingRuleRow::Search))
                .col(id_column(StandingRuleRow::Project))
                .col(text(StandingRuleRow::Fields))
                .col(maybe_id(StandingRuleRow::CreatesIn))
                .col(text(StandingRuleRow::State))
                .col(number(StandingRuleRow::EndsAt))
                .col(number(StandingRuleRow::MadeAt))
                .col(number(StandingRuleRow::Version))
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("standing_rule_by_project")
                .table(StandingRuleRow::Table)
                .col(StandingRuleRow::Project)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(IssueWorkRow::Table)
                .col(id_column(IssueWorkRow::Id).primary_key())
                .col(id_column(IssueWorkRow::Issue))
                .col(id_column(IssueWorkRow::HandOver))
                .col(id_column(IssueWorkRow::Agent))
                .col(id_column(IssueWorkRow::Person))
                .col(text(IssueWorkRow::State))
                .col(maybe_text(IssueWorkRow::GaveUp))
                .col(flag(IssueWorkRow::Closed))
                .col(flag(IssueWorkRow::AcceptedByItself))
                .col(number(IssueWorkRow::MadeAt))
                .col(number(IssueWorkRow::Version))
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("issue_work_by_issue")
                .table(IssueWorkRow::Table)
                .col(IssueWorkRow::Issue)
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("issue_work_by_agent")
                .table(IssueWorkRow::Table)
                .col(IssueWorkRow::Agent)
                .col(IssueWorkRow::Id)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(AttemptRow::Table)
                .col(id_column(AttemptRow::Id).primary_key())
                .col(id_column(AttemptRow::Work))
                .col(number(AttemptRow::Number))
                .col(maybe_text(AttemptRow::Plan))
                .col(flag(AttemptRow::PlanApproved))
                .col(number(AttemptRow::ReportedMinutes))
                .col(number(AttemptRow::ReportedCost))
                .col(number(AttemptRow::StartedAt))
                .col(maybe_number(AttemptRow::EndedAt))
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("agent_attempt_by_work")
                .table(AttemptRow::Table)
                .col(AttemptRow::Work)
                .col(AttemptRow::Number)
                .unique()
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(AttemptResultRow::Table)
                .col(id_column(AttemptResultRow::Id).primary_key())
                .col(id_column(AttemptResultRow::Attempt))
                .col(text(AttemptResultRow::Kind))
                .col(text(AttemptResultRow::Label))
                .col(maybe_text(AttemptResultRow::Link))
                .col(maybe_text(AttemptResultRow::Outcome))
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("agent_result_by_attempt")
                .table(AttemptResultRow::Table)
                .col(AttemptResultRow::Attempt)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(QuestionRow::Table)
                .col(id_column(QuestionRow::Id).primary_key())
                .col(id_column(QuestionRow::Work))
                .col(text(QuestionRow::Text))
                .col(maybe_text(QuestionRow::Answer))
                .col(maybe_id(QuestionRow::AnsweredBy))
                .col(number(QuestionRow::AskedAt))
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("agent_question_by_work")
                .table(QuestionRow::Table)
                .col(QuestionRow::Work)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(ReviewRow::Table)
                .col(id_column(ReviewRow::Id).primary_key())
                .col(id_column(ReviewRow::Work))
                .col(text(ReviewRow::Answer))
                .col(maybe_text(ReviewRow::Note))
                .col(maybe_id(ReviewRow::By))
                .col(number(ReviewRow::At))
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("agent_review_by_work")
                .table(ReviewRow::Table)
                .col(ReviewRow::Work)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(ProposalRow::Table)
                .col(id_column(ProposalRow::Id).primary_key())
                .col(id_column(ProposalRow::Rule))
                .col(id_column(ProposalRow::Agent))
                .col(id_column(ProposalRow::Issue))
                .col(text(ProposalRow::Field))
                .col(text(ProposalRow::Before))
                .col(text(ProposalRow::After))
                .col(text(ProposalRow::State))
                .col(maybe_id(ProposalRow::DecidedBy))
                .col(number(ProposalRow::MadeAt))
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("agent_proposal_by_issue")
                .table(ProposalRow::Table)
                .col(ProposalRow::Issue)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(RuleMatchRow::Table)
                .col(id_column(RuleMatchRow::Rule))
                .col(id_column(RuleMatchRow::Issue))
                .col(number(RuleMatchRow::At))
                .primary_key(
                    Index::create()
                        .col(RuleMatchRow::Rule)
                        .col(RuleMatchRow::Issue),
                )
                .to_owned(),
        ),
    ]
}
