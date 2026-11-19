//! Migration 62: what a person lets their agent do beyond its work (ADR-0029, amendment M9).
//!
//! Two switches of an agent, *May hand over to my other agents* and *May undo its own runs*, both
//! off for every agent there is and every agent made later. Beside them, on a hand-over, the agent
//! that made it (none for a person or a standing rule) and its place in a chain of hand-overs (0 for
//! every hand-over there is, which a person or a rule made).

use sea_query::{ColumnDef, Table};

use super::Step;
use crate::schema::{AgentRow, HandOverRow};

fn off(name: impl sea_query::Iden + 'static) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.boolean().not_null().default(false);
    column
}

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Alter(
            Table::alter()
                .table(AgentRow::Table)
                .add_column(off(AgentRow::MayHandOver))
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(AgentRow::Table)
                .add_column(off(AgentRow::MayUndo))
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(HandOverRow::Table)
                .add_column(ColumnDef::new(HandOverRow::ByAgent).blob())
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(HandOverRow::Table)
                .add_column(
                    ColumnDef::new(HandOverRow::Depth)
                        .big_integer()
                        .not_null()
                        .default(0),
                )
                .to_owned(),
        ),
    ]
}
