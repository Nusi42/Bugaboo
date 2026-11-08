//! Migration 46: when a question of an agent was answered (AI-34).
//!
//! `agent_question.answered_at` holds the moment the answer was written, in microseconds. The
//! answers written before have no moment to name, so the column stays empty for every row that is
//! there.

use sea_query::{ColumnDef, Table};

use super::Step;
use crate::schema::QuestionRow;

pub(super) fn steps() -> Vec<Step> {
    vec![Step::Alter(
        Table::alter()
            .table(QuestionRow::Table)
            .add_column(ColumnDef::new(QuestionRow::AnsweredAt).big_integer())
            .to_owned(),
    )]
}
