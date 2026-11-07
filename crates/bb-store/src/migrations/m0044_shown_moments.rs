//! Migration 44: the moment a row is shown at, and a hash that tells nothing (PRIV-09, ADR-0035,
//! decision 8).
//!
//! A reader who may not see the moment a time entry was saved is told its date, and the history and
//! the list of the entries of an issue are put in order and cut in pages for such a reader by what
//! they are shown, not by the order things were written in. So the database holds, for each row of
//! the history, the moment it is shown at to such a reader (the start of the date for the row of a
//! time entry, the moment itself for every other row) and, for each row of the history and each time
//! entry, a hash of its ID that ties rows of one moment or date and follows no order of making.
//!
//! The moments of the rows that exist are filled in here: the start of the day the row was written
//! on for a row of time (the date of the entry is in the changes of the row as text, which the two
//! databases read differently, and a row without its date is shown on the day it was written
//! anyway), the moment for the others. The hashes of the rows that exist are made at random once,
//! which is as good a tie as a hash of an ID; new rows get the hash of their IDs.

use sea_query::{ColumnDef, Expr, ExprTrait as _, Index, Query, Table};

use super::Step;
use crate::db::FixedSql;
use crate::schema::{HistoryEntryRow, TimeEntryRow};

/// Microseconds in a day, of 24 hours: the start of the day a moment is on, in UTC.
const DAY_MICROS: i64 = 86_400_000_000;

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Alter(
            Table::alter()
                .table(HistoryEntryRow::Table)
                .add_column(
                    ColumnDef::new(HistoryEntryRow::ShownAt)
                        .big_integer()
                        .not_null()
                        .default(0),
                )
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(HistoryEntryRow::Table)
                .add_column(
                    ColumnDef::new(HistoryEntryRow::ShownHash)
                        .text()
                        .not_null()
                        .default(""),
                )
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(TimeEntryRow::Table)
                .add_column(
                    ColumnDef::new(TimeEntryRow::ShownHash)
                        .text()
                        .not_null()
                        .default(""),
                )
                .to_owned(),
        ),
        Step::Update(
            Query::update()
                .table(HistoryEntryRow::Table)
                .value(HistoryEntryRow::ShownAt, Expr::col(HistoryEntryRow::At))
                .to_owned(),
        ),
        Step::Update(
            Query::update()
                .table(HistoryEntryRow::Table)
                .value(
                    HistoryEntryRow::ShownAt,
                    Expr::col(HistoryEntryRow::At)
                        .sub(Expr::col(HistoryEntryRow::At).modulo(DAY_MICROS)),
                )
                .and_where(Expr::col(HistoryEntryRow::RecordKind).eq("time_entry"))
                .to_owned(),
        ),
        Step::Sql(FixedSql {
            sqlite: vec![
                "UPDATE history_entry SET shown_hash = lower(hex(randomblob(16)))",
                "UPDATE time_entry SET shown_hash = lower(hex(randomblob(16)))",
            ],
            postgres: vec![
                "UPDATE history_entry SET shown_hash = md5(random()::text || clock_timestamp()::text)",
                "UPDATE time_entry SET shown_hash = md5(random()::text || clock_timestamp()::text)",
            ],
        }),
        Step::Index(
            Index::create()
                .name("history_entry_by_shown")
                .table(HistoryEntryRow::Table)
                .col(HistoryEntryRow::Issue)
                .col(HistoryEntryRow::ShownAt)
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("time_entry_by_issue_and_day")
                .table(TimeEntryRow::Table)
                .col(TimeEntryRow::Issue)
                .col(TimeEntryRow::Day)
                .to_owned(),
        ),
    ]
}
