//! Migration 46: when a question of an agent was answered (AI-34), and why work stopped (AI-33).
//!
//! `agent_question.answered_at` holds the moment the answer was written, in microseconds. The
//! answers written before have no moment to name, so the column stays empty for every row that is
//! there.
//!
//! `issue_work.stopped_for` holds the word of why work stopped without a review and without the
//! agent giving up (`agent-stopped`, `access-lost`, `handed-on`, `ran-out`). Work that stopped
//! before has no cause to name, so the column stays empty for every row that is there.

use sea_query::{ColumnDef, Table};

use super::Step;
use crate::schema::{IssueWorkRow, QuestionRow};

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Alter(
            Table::alter()
                .table(QuestionRow::Table)
                .add_column(ColumnDef::new(QuestionRow::AnsweredAt).big_integer())
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(IssueWorkRow::Table)
                .add_column(ColumnDef::new(IssueWorkRow::StoppedFor).text())
                .to_owned(),
        ),
    ]
}
