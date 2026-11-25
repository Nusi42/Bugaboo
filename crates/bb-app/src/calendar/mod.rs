//! The calendar (PLAN-09, PRJ-06, ADR-0034, amendment of 2026-11-24): a saved search or a search
//! text shown on the days of one date field (*Dates from*: *Due date* at the start, M10-Q5), with
//! the due dates of goals as marks. A drag on the calendar is the change of that date field, with
//! its version check, through the change of an issue that exists.

mod read;
mod types;

pub use read::calendar;
pub use types::*;
