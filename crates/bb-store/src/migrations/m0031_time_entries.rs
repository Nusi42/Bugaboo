//! Migration 31: time on issues (TIME-01, TIME-02, TIME-13, ADR-0035): the timer of each person, the
//! time entries, and the marks of the starter fields of time entries.
//!
//! - `timer` has one row for each person who ever started a timer; the person is the primary key,
//!   so two devices that start at the same moment make one timer and one refusal. It runs while it
//!   names an issue.
//! - `time_entry` holds the entries, with the day as the number of days since 1970 (the way a day
//!   of an own field is stored). The index of the issue serves the entries of an issue and the
//!   minutes spent on it; the index of the person and the day serves a person's week and the count
//!   of a day.
//! - `starter_field` names the own field of each starter mark (`activity`, `cost-center`), once.
//!
//! Nothing is written into the tables: the starter fields are made by the use case of the first
//! start, which runs at every start, so an upgraded server gets them too. No column refers to a
//! person (a removed person leaves a tombstone, PRIV-04).

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{FieldRow, IssueRow, StarterFieldRow, TimeEntryRow, TimerRow};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = vec![timers()];
    steps.extend(entries());
    steps.push(starter_fields());
    steps
}

fn timers() -> Step {
    Step::Table(
        Table::create()
            .table(TimerRow::Table)
            .col(id_column(TimerRow::Person).primary_key())
            .col(ColumnDef::new(TimerRow::Issue).blob())
            .col(ColumnDef::new(TimerRow::StartedAt).big_integer())
            .col(ColumnDef::new(TimerRow::WarnedAt).big_integer())
            .col(ColumnDef::new(TimerRow::ChangedAt).big_integer().not_null())
            .col(ColumnDef::new(TimerRow::Version).big_integer().not_null())
            .foreign_key(
                ForeignKey::create()
                    .from(TimerRow::Table, TimerRow::Issue)
                    .to(IssueRow::Table, IssueRow::Id),
            )
            .to_owned(),
    )
}

fn entries() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(TimeEntryRow::Table)
                .col(id_column(TimeEntryRow::Id).primary_key())
                .col(id_column(TimeEntryRow::Person))
                .col(id_column(TimeEntryRow::Issue))
                .col(ColumnDef::new(TimeEntryRow::Day).big_integer().not_null())
                .col(
                    ColumnDef::new(TimeEntryRow::Minutes)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(TimeEntryRow::Note).text().not_null())
                .col(ColumnDef::new(TimeEntryRow::Billable).boolean().not_null())
                .col(ColumnDef::new(TimeEntryRow::StartedAt).big_integer())
                .col(
                    ColumnDef::new(TimeEntryRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(TimeEntryRow::Version)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(TimeEntryRow::Table, TimeEntryRow::Issue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .to_owned(),
        ),
        // The entries of an issue, and the minutes spent on it.
        Step::Index(
            Index::create()
                .name("time_entry_by_issue")
                .table(TimeEntryRow::Table)
                .col(TimeEntryRow::Issue)
                .col(TimeEntryRow::Minutes)
                .to_owned(),
        ),
        // A person's entries by day: their week, and how many they booked on a day.
        Step::Index(
            Index::create()
                .name("time_entry_by_person")
                .table(TimeEntryRow::Table)
                .col(TimeEntryRow::Person)
                .col(TimeEntryRow::Day)
                .col(TimeEntryRow::Id)
                .to_owned(),
        ),
    ]
}

fn starter_fields() -> Step {
    Step::Table(
        Table::create()
            .table(StarterFieldRow::Table)
            .col(
                ColumnDef::new(StarterFieldRow::Starter)
                    .text()
                    .not_null()
                    .primary_key(),
            )
            .col(id_column(StarterFieldRow::Field).unique_key())
            .foreign_key(
                ForeignKey::create()
                    .from(StarterFieldRow::Table, StarterFieldRow::Field)
                    .to(FieldRow::Table, FieldRow::Id),
            )
            .to_owned(),
    )
}
