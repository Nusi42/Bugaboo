//! Migration 61: the requests for agents of programs (ADR-0029, amendment M9).
//!
//! One row for each request: the person asked, the access key or the grant of the app that asked
//! (one of the two), the name shown for it and the web address of an app, the reason, the agents
//! asked for as JSON (with the agents an approval made), the state, and when it was asked and
//! answered. Whether it ran out is the time, not a column. A person's requests are read newest first, the waiting ones of a key or grant are
//! counted, and the daily job deletes the old ones by when they were asked.

use sea_query::{ColumnDef, Index, Table};

use super::{Step, id_column};
use crate::schema::AgentRequestRow;

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(AgentRequestRow::Table)
                .col(
                    ColumnDef::new(AgentRequestRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(id_column(AgentRequestRow::Person))
                .col(ColumnDef::new(AgentRequestRow::AskedByKey).blob())
                .col(ColumnDef::new(AgentRequestRow::AskedByGrant).blob())
                .col(
                    ColumnDef::new(AgentRequestRow::AskedByName)
                        .text()
                        .not_null(),
                )
                .col(ColumnDef::new(AgentRequestRow::AskedFrom).text())
                .col(ColumnDef::new(AgentRequestRow::Reason).text().not_null())
                .col(ColumnDef::new(AgentRequestRow::Agents).text().not_null())
                .col(ColumnDef::new(AgentRequestRow::State).text().not_null())
                .col(
                    ColumnDef::new(AgentRequestRow::AskedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(AgentRequestRow::AnsweredAt).big_integer())
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("agent_request_by_person")
                .table(AgentRequestRow::Table)
                .col(AgentRequestRow::Person)
                .col(AgentRequestRow::AskedAt)
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("agent_request_by_asked_at")
                .table(AgentRequestRow::Table)
                .col(AgentRequestRow::AskedAt)
                .to_owned(),
        ),
    ]
}
