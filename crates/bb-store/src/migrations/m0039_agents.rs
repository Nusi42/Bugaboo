//! Migration 39: agents (AI-30, ADR-0029).
//!
//! - `agent` holds the agents. The person responsible for an agent and the name are unique
//!   together, so one person has no two agents with one name. The state is `active`, `paused` or
//!   `stopped`; the three limits are whole numbers; `offered_to` is the person an agent is handed
//!   to who has not accepted it yet.
//! - `access_key.agent` names the agent of an agent key, and is empty for the key of a person.
//!
//! Nothing is written into the tables. Both people of an agent are references to `person`.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{AccessKeyRow, AgentRow, PersonRow};

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(AgentRow::Table)
                .col(id_column(AgentRow::Id).primary_key())
                .col(id_column(AgentRow::Person))
                .col(ColumnDef::new(AgentRow::Name).text().not_null())
                .col(ColumnDef::new(AgentRow::State).text().not_null())
                .col(
                    ColumnDef::new(AgentRow::ChangesAMinute)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(AgentRow::IssuesAtOnce)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(AgentRow::AttemptsOnOneIssue)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(AgentRow::OfferedTo).blob())
                .col(id_column(AgentRow::MadeBy))
                .col(ColumnDef::new(AgentRow::MadeAt).big_integer().not_null())
                .col(ColumnDef::new(AgentRow::Version).big_integer().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .from(AgentRow::Table, AgentRow::Person)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("agent_by_person_and_name")
                .table(AgentRow::Table)
                .col(AgentRow::Person)
                .col(AgentRow::Name)
                .unique()
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(AccessKeyRow::Table)
                .add_column(ColumnDef::new(AccessKeyRow::Agent).blob())
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("access_key_by_agent")
                .table(AccessKeyRow::Table)
                .col(AccessKeyRow::Agent)
                .to_owned(),
        ),
    ]
}
