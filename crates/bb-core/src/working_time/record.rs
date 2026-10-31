//! The stored records of working hours (TIME-08, ADR-0035, decision 5): a work day, its periods of
//! work and its breaks. All moments are in UTC; the day keeps the zone it started in, so a later
//! move to another zone never moves a past day. Every field that says something about the person's
//! time is `employee_time`, kept because the law asks for it (ADR-0015; architecture §12, *Personal
//! data in M6*). The reason of a change is kept in the event of the change, with the old values
//! (ADR-0035, decision 7).

use jiff::Timestamp;
use jiff::civil::Date;

use super::{Span, Times, Warning};
use crate::id::{UserId, WorkDayId};
use crate::personal::Entity;

/// One work day of a person: at most one for a person and a date.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct WorkDay {
    /// The ID of the day.
    #[not_personal]
    pub id: WorkDayId,
    /// The person whose day it is. A removed person's days keep the link and the name (ADR-0035,
    /// decision 7).
    #[personal(category = "identity", basis = "legal_obligation")]
    pub person: UserId,
    /// The person's day of its first start.
    #[personal(category = "employee_time", basis = "legal_obligation")]
    pub date: Date,
    /// The time zone the day started in, by its name in the zone data (`Europe/Berlin`).
    #[personal(category = "employee_time", basis = "legal_obligation")]
    pub zone: String,
    /// The warnings of the working-time rules, as they were checked at the last change of the day
    /// or of the day before it.
    #[personal(category = "employee_time", basis = "legal_obligation")]
    pub warnings: Vec<Warning>,
    /// When the day was last changed.
    #[personal(category = "employee_time", basis = "legal_obligation")]
    pub updated_at: Timestamp,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// One period of work of a day, from a start to an end (none while it runs).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct WorkPeriod {
    /// The day.
    #[not_personal]
    pub day: WorkDayId,
    /// When it started.
    #[personal(category = "employee_time", basis = "legal_obligation")]
    pub start: Timestamp,
    /// When it ended.
    #[personal(category = "employee_time", basis = "legal_obligation")]
    pub end: Option<Timestamp>,
}

/// One break of a day, inside one of its periods.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct WorkBreak {
    /// The day.
    #[not_personal]
    pub day: WorkDayId,
    /// When it started.
    #[personal(category = "employee_time", basis = "legal_obligation")]
    pub start: Timestamp,
    /// When it ended.
    #[personal(category = "employee_time", basis = "legal_obligation")]
    pub end: Option<Timestamp>,
}

/// A work day with its periods and breaks, as it is read and written whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredDay {
    /// The day.
    pub day: WorkDay,
    /// Its moments.
    pub times: Times,
}

impl StoredDay {
    /// The rows of its periods.
    #[must_use]
    pub fn period_rows(&self) -> Vec<WorkPeriod> {
        rows(self.day.id, &self.times.periods)
            .map(|(day, span)| WorkPeriod {
                day,
                start: span.start,
                end: span.end,
            })
            .collect()
    }

    /// The rows of its breaks.
    #[must_use]
    pub fn break_rows(&self) -> Vec<WorkBreak> {
        rows(self.day.id, &self.times.breaks)
            .map(|(day, span)| WorkBreak {
                day,
                start: span.start,
                end: span.end,
            })
            .collect()
    }
}

fn rows(day: WorkDayId, spans: &[Span]) -> impl Iterator<Item = (WorkDayId, Span)> + '_ {
    spans.iter().map(move |span| (day, *span))
}
