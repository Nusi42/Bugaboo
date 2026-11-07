//! Migration 40: runs of AI apps and agents, and the marks of texts written with AI (AI-38,
//! COMP-08, COMP-09, ADR-0029 decision 7, ADR-0013).
//!
//! - `change_run` gets the `kind` of the run (a row that was there is a change to many issues), the
//!   name of the `app` of a session of an AI app, and the `agent` of an attempt. A session and an
//!   attempt hold no action and no items.
//! - `event.run` names the run an event was made in, empty for every event before and for every
//!   change outside a run. The index serves the list of the changes of a run.
//! - `history_entry.run` and `history_entry.undid_run` name the run a row of the history was made
//!   in, and the run it is the undo of; they are empty for every row before and outside a run.
//! - `text_mark` holds, for each text that AI wrote (a part of a record), how it was written, with
//!   the app or the agent. A text without a row has no mark. A removed agent or person leaves the
//!   row as it is (the mark says how, not who).
//!
//! Nothing is written into the tables, so an upgraded database has no run and no mark before it.

use sea_query::{ColumnDef, Index, Table};

use super::{Step, id_column};
use crate::schema::{Event, HistoryEntryRow, ManyChangeRunRow, TextMarkRow};

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Alter(
            Table::alter()
                .table(ManyChangeRunRow::Table)
                .add_column(
                    ColumnDef::new(ManyChangeRunRow::Kind)
                        .text()
                        .not_null()
                        .default("many-changes"),
                )
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(ManyChangeRunRow::Table)
                .add_column(ColumnDef::new(ManyChangeRunRow::App).text())
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(ManyChangeRunRow::Table)
                .add_column(ColumnDef::new(ManyChangeRunRow::Agent).blob())
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(Event::Table)
                .add_column(ColumnDef::new(Event::Run).blob())
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(HistoryEntryRow::Table)
                .add_column(ColumnDef::new(HistoryEntryRow::Run).blob())
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(HistoryEntryRow::Table)
                .add_column(ColumnDef::new(HistoryEntryRow::UndidRun).blob())
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("event_by_run")
                .table(Event::Table)
                .col(Event::Run)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(TextMarkRow::Table)
                .col(ColumnDef::new(TextMarkRow::EntityKind).text().not_null())
                .col(id_column(TextMarkRow::EntityId))
                .col(ColumnDef::new(TextMarkRow::Part).text().not_null())
                .col(ColumnDef::new(TextMarkRow::WrittenBy).text().not_null())
                .col(ColumnDef::new(TextMarkRow::App).text())
                .col(ColumnDef::new(TextMarkRow::Agent).blob())
                .primary_key(
                    Index::create()
                        .col(TextMarkRow::EntityKind)
                        .col(TextMarkRow::EntityId)
                        .col(TextMarkRow::Part),
                )
                .to_owned(),
        ),
    ]
}
