//! Migration 38: *Never sent to AI* and the log of tool calls (AI-03, AI-05, ADR-0013).
//!
//! - `never_sent_to_ai` holds the mark of a built-in field (by its word) or of a project (with a
//!   reference to it), once each; a subject without a row is not marked, at version 0. An own
//!   field keeps its mark in its definition.
//! - `tool_call` is the log of tool calls: who, how, which tool, the sizes and the time, never the
//!   content. No column refers to a person (a removed person leaves a tombstone, PRIV-04). The
//!   index of the time serves the job that deletes what outlived its keep period.
//!
//! Nothing is written into the tables, so an upgraded database keeps nothing from AI that it did
//! not before: no field or project is marked until an administrator marks it.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{NeverSentRow, ProjectRow, ToolCallRow};

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(NeverSentRow::Table)
                .col(ColumnDef::new(NeverSentRow::Field).text())
                .col(ColumnDef::new(NeverSentRow::Project).blob())
                .col(ColumnDef::new(NeverSentRow::Marked).boolean().not_null())
                .col(
                    ColumnDef::new(NeverSentRow::Version)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(NeverSentRow::Table, NeverSentRow::Project)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .to_owned(),
        ),
        // An empty value never equals another, so a row with a project has no field and the other
        // way round, on both databases.
        Step::Index(
            Index::create()
                .name("never_sent_by_field")
                .table(NeverSentRow::Table)
                .col(NeverSentRow::Field)
                .unique()
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("never_sent_by_project")
                .table(NeverSentRow::Table)
                .col(NeverSentRow::Project)
                .unique()
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(ToolCallRow::Table)
                .col(id_column(ToolCallRow::Id).primary_key())
                .col(id_column(ToolCallRow::Person))
                .col(ColumnDef::new(ToolCallRow::Caller).text().not_null())
                .col(ColumnDef::new(ToolCallRow::App).text())
                .col(ColumnDef::new(ToolCallRow::Tool).text().not_null())
                .col(ColumnDef::new(ToolCallRow::Effect).text().not_null())
                .col(
                    ColumnDef::new(ToolCallRow::BytesIn)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ToolCallRow::BytesOut)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(ToolCallRow::At).big_integer().not_null())
                .col(ColumnDef::new(ToolCallRow::Outcome).text().not_null())
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("tool_call_by_time")
                .table(ToolCallRow::Table)
                .col(ToolCallRow::At)
                .to_owned(),
        ),
    ]
}
