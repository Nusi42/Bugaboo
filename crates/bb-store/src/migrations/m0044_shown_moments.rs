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
//! The moments of the rows that exist are filled in here: for a row of time the start of the date of
//! its entry (the last `date` in the changes of the row, which the two databases read from the JSON
//! text each in its own way; the day the row was written on when it names none), the moment for the
//! others. The hashes of the rows that exist are made at random once,
//! which is as good a tie as a hash of an ID; new rows get the hash of their IDs.

use sea_query::{ColumnDef, Expr, Index, Query, Table};

use super::Step;
use crate::db::FixedSql;
use crate::schema::{HistoryEntryRow, TimeEntryRow};

/// The rows of time entries: shown at midnight UTC of the date of the entry (microseconds), else of the
/// day the row was written on (a day is 86 400 000 000 microseconds). A value that is not a date
/// (checked by its shape before the cast on PostgreSQL) counts as no date.
const SQLITE_TIME_ROWS: &str = "UPDATE history_entry SET shown_at = COALESCE((    SELECT CAST(strftime('%s', COALESCE(json_extract(change.value, '$.after'),                                         json_extract(change.value, '$.before'))) AS INTEGER) * 1000000     FROM json_each(history_entry.changes) AS change     WHERE json_extract(change.value, '$.field') = 'date'     ORDER BY change.key DESC LIMIT 1), at - at % 86400000000)     WHERE record_kind = 'time_entry'";

const POSTGRES_TIME_ROWS: &str = "UPDATE history_entry SET shown_at = COALESCE((    SELECT (extract(epoch FROM CASE WHEN COALESCE(change.value ->> 'after', change.value ->> 'before') ~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}$' THEN COALESCE(change.value ->> 'after', change.value ->> 'before')::date::timestamp END)             * 1000000)::bigint     FROM jsonb_array_elements(history_entry.changes::jsonb) WITH ORDINALITY AS change(value, place)     WHERE change.value ->> 'field' = 'date'     ORDER BY change.place DESC LIMIT 1), at - at % 86400000000)     WHERE record_kind = 'time_entry'";

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
        // The row of a time entry is shown at the start of the date of its entry: the date of the last
        // change of the date in its changes (a JSON list), or, for a row that names none, the day it
        // was written on. The two databases read JSON differently.
        Step::Sql(FixedSql {
            sqlite: vec![SQLITE_TIME_ROWS],
            postgres: vec![POSTGRES_TIME_ROWS],
        }),
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
        // The rows of other people: by the moment they are shown at.
        Step::Index(
            Index::create()
                .name("history_entry_by_shown")
                .table(HistoryEntryRow::Table)
                .col(HistoryEntryRow::Issue)
                .col(HistoryEntryRow::ShownAt)
                .col(HistoryEntryRow::ShownHash)
                .to_owned(),
        ),
        // The rows of the reader themself: by the moment they happened.
        Step::Index(
            Index::create()
                .name("history_entry_by_author")
                .table(HistoryEntryRow::Table)
                .col(HistoryEntryRow::Issue)
                .col(HistoryEntryRow::By)
                .col(HistoryEntryRow::At)
                .col(HistoryEntryRow::ShownHash)
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
