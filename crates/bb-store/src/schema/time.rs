//! The tables of time on issues (ADR-0035): the timer of each person, the time entries, and the
//! marks of the starter fields of time entries.

use super::Iden;

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
