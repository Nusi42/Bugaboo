//! Migration 45: the limit that paused an agent (AI-37, ADR-0029, decision 9).
//!
//! `agent.paused_for` holds the word of the limit an agent reached when it was paused by it (empty
//! for an agent that is not paused or that its person paused). The people who paused an agent
//! before have no limit to name, so the column stays empty for every row that is there.

use sea_query::{ColumnDef, Table};

use super::Step;
use crate::schema::AgentRow;

pub(super) fn steps() -> Vec<Step> {
    vec![Step::Alter(
        Table::alter()
            .table(AgentRow::Table)
            .add_column(ColumnDef::new(AgentRow::PausedFor).text())
            .to_owned(),
    )]
}
