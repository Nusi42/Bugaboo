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
//! - `time_entry_value` holds the values of the own fields of time entries: the columns of
//!   `field_value`, apart, because every part of them is about a person's time (`employee_time`).
//! - `starter_field` names the own field of each starter mark (`activity`, `cost-center`), once.
//!
//! Nothing is written into the tables: the starter fields are made by the use case of the first
//! start, which runs at every start, so an upgraded server gets them too. No column refers to a
//! person (a removed person leaves a tombstone, PRIV-04).

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{
    FieldRow, IssueRow, StarterFieldRow, TimeEntryRow, TimeEntryValueRow, TimerRow,
};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = vec![timers()];
    steps.extend(entries());
    steps.extend(entry_values());
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

/// The values of the own fields of time entries: the columns of `field_value` (migration 9), with
/// the marks of time at work.
fn entry_values() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(TimeEntryValueRow::Table)
                .col(
                    ColumnDef::new(TimeEntryValueRow::EntityKind)
                        .text()
                        .not_null(),
                )
                .col(id_column(TimeEntryValueRow::EntityId))
                .col(id_column(TimeEntryValueRow::Field))
                .col(ColumnDef::new(TimeEntryValueRow::ValueText).text())
                .col(ColumnDef::new(TimeEntryValueRow::ValueNumber).big_integer())
                .col(ColumnDef::new(TimeEntryValueRow::ValueDate).big_integer())
                .col(ColumnDef::new(TimeEntryValueRow::ValueRef).blob())
                .col(
                    ColumnDef::new(TimeEntryValueRow::Position)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(TimeEntryValueRow::EntityKind)
                        .col(TimeEntryValueRow::EntityId)
                        .col(TimeEntryValueRow::Field)
                        .col(TimeEntryValueRow::Position),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(TimeEntryValueRow::Table, TimeEntryValueRow::Field)
                        .to(FieldRow::Table, FieldRow::Id),
                )
                .to_owned(),
        ),
        // The entries that hold a choice: combining two choices moves them.
        Step::Index(
            Index::create()
                .name("time_entry_value_by_ref")
                .table(TimeEntryValueRow::Table)
                .col(TimeEntryValueRow::Field)
                .col(TimeEntryValueRow::ValueRef)
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
