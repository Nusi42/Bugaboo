//! The stored records of work times (PLAN-11, PLAN-15, ADR-0030, decision 2; architecture §12,
//! *Personal data in M8*). Every field that says when or how much a person works is
//! `employee_time`; the legal basis is the contract, the work the organization plans with its
//! people. An away time has **no reason**: the record has no such field, and a review looks for one.

use jiff::civil::Date;

use crate::id::{AwayTimeId, ProjectId, UserId};
use crate::personal::Entity;

/// One away time of a person: a first and a last day, a half day at either end (*Afternoon only*
/// for the first, *Morning only* for the last), and a note that only the person reads.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct AwayTime {
    /// The ID of the away time.
    #[not_personal]
    pub id: AwayTimeId,
    /// The person who is away.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,
    /// The first day.
    #[personal(category = "employee_time", basis = "contract")]
    pub first: Date,
    /// The last day.
    #[personal(category = "employee_time", basis = "contract")]
    pub last: Date,
    /// Whether the person is away only in the afternoon of the first day.
    #[personal(category = "employee_time", basis = "contract")]
    pub first_afternoon_only: bool,
    /// Whether the person is away only in the morning of the last day.
    #[personal(category = "employee_time", basis = "contract")]
    pub last_morning_only: bool,
    /// The note; only the person reads and writes it, never somebody who enters work times for
    /// them, and it is never sent in a notification.
    #[personal(category = "content", basis = "contract")]
    pub note: Option<String>,
    /// Who entered it: the person, or somebody with *Enter work times for others*.
    #[personal(category = "identity", basis = "contract")]
    pub entered_by: UserId,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// One weekday of the usual work days of a person: whether they work on it and for how long.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct UsualWorkDay {
    /// The person.
    #[personal(category = "employee_time", basis = "contract")]
    pub person: UserId,
    /// The weekday, 1 for Monday to 7 for Sunday.
    #[personal(category = "employee_time", basis = "contract")]
    pub weekday: u8,
    /// Whether the person usually works on it.
    #[personal(category = "employee_time", basis = "contract")]
    pub works: bool,
    /// The work hours of the day, in minutes.
    #[personal(category = "employee_time", basis = "contract")]
    pub minutes: u16,
}

/// The share of a person in a project, in percent. A person with no row has 100 percent; the row
/// is kept while the person has no role in the project, where it counts as nothing.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct ProjectShare {
    /// The person.
    #[personal(category = "employee_time", basis = "contract")]
    pub person: UserId,
    /// The project.
    #[personal(category = "employee_time", basis = "contract")]
    pub project: ProjectId,
    /// The share in percent, 0 to 100.
    #[personal(category = "employee_time", basis = "contract")]
    pub percent: u8,
}
