//! The tables of time on issues (ADR-0035): the timer of each person, the time entries, the values
//! of their own fields, and the marks of the starter fields of time entries.

use super::{FieldValueRow, Iden, StoredTable};

/// The timer of each person who ever started one (TIME-01): the person is the key, so one person
/// has one timer on every device.
#[derive(Clone, Copy, Iden)]
pub(crate) enum TimerRow {
    #[iden = "timer"]
    Table,
    Person,
    Issue,
    StartedAt,
    WarnedAt,
    ChangedAt,
    Version,
}

/// Time entries (TIME-01, TIME-02). The values of their own fields are in `field_value`.
#[derive(Clone, Copy, Iden)]
pub(crate) enum TimeEntryRow {
    #[iden = "time_entry"]
    Table,
    Id,
    Person,
    Issue,
    Day,
    Minutes,
    Note,
    Billable,
    StartedAt,
    CreatedAt,
    Version,
}

/// Which own field each starter field of time entries is (TIME-13): found by the mark, never by
/// the name.
#[derive(Clone, Copy, Iden)]
pub(crate) enum StarterFieldRow {
    #[iden = "starter_field"]
    Table,
    Starter,
    Field,
}

/// The values of the own fields of time entries (TIME-13), in a table of their own because every
/// part of them is about a person's time. Its columns are those of `field_value`, named once by
/// [`FieldValueRow`](super::FieldValueRow).
#[derive(Clone, Copy, Iden)]
pub(crate) enum TimeEntryValueRow {
    #[iden = "time_entry_value"]
    Table,
}

stored_tables! {
    TimerRow => bb_core::time::entry::Timer {
        Person => person,
        Issue => running,
        StartedAt => running,
        WarnedAt => running,
        ChangedAt => changed_at,
        Version => version,
    },
    TimeEntryRow => bb_core::time::entry::TimeEntry {
        Id => id,
        Person => person,
        Issue => issue,
        Day => date,
        Minutes => minutes,
        Note => note,
        Billable => billable,
        StartedAt => started_at,
        CreatedAt => created_at,
        Version => version,
    },
    TimeEntryValueRow(FieldValueRow) => bb_core::time::entry::TimeEntryValue {
        EntityKind => entity_kind,
        EntityId => entity_id,
        Field => field,
        ValueText => value_text,
        ValueNumber => value_number,
        ValueDate => value_date,
        ValueRef => value_ref,
        Position => position,
    },
    StarterFieldRow => bb_core::time::entry::StarterFieldMark {
        Starter => starter,
        Field => field,
    },
}
