//! The tables of working hours (TIME-08, ADR-0035, decision 5): a work day, its periods of work and
//! its breaks.

use super::Iden;

/// Work days: one for a person and a date. The person has no reference: a removed person's days
/// stay with the link (ADR-0035, decision 7).
#[derive(Clone, Copy, Iden)]
pub(crate) enum WorkDayRow {
    #[iden = "work_day"]
    Table,
    Id,
    Person,
    Date,
    Zone,
    Warnings,
    UpdatedAt,
    Version,
}

/// The periods of work of a day.
#[derive(Clone, Copy, Iden)]
pub(crate) enum WorkPeriodRow {
    #[iden = "work_period"]
    Table,
    Day,
    Start,
    End,
}

/// The breaks of a day.
#[derive(Clone, Copy, Iden)]
pub(crate) enum WorkBreakRow {
    #[iden = "work_break"]
    Table,
    Day,
    Start,
    End,
}
