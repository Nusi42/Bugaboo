//! A person's time on issues as it is stored (ADR-0035, decisions 1 and 3): the time entries, the
//! timer of each person, and the marks that find the starter fields *Activity* and *Cost center*.
//! The rules that decide them are in `bb-app::time`.
//!
//! Every column says something about a person's time, so it is `employee_time` (ADR-0015), apart
//! from the person, who is who they are, and the ID and the version, which say nothing.

use jiff::Timestamp;
use jiff::civil::Date;

use super::Minutes;
use crate::id::{FieldId, IssueId, TimeEntryId, UserId};
use crate::message::{self, MessageKey};
use crate::personal::Entity;

/// One time entry: what a person spent time on, for accounting (TIME-01, TIME-02).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct TimeEntry {
    /// The ID of the entry.
    #[not_personal]
    pub id: TimeEntryId,
    /// Whose time it is.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,
    /// The issue the time was spent on.
    #[personal(category = "employee_time", basis = "contract")]
    pub issue: IssueId,
    /// The person's day of its start, or the date typed with *Add time*.
    #[personal(category = "employee_time", basis = "contract")]
    pub date: Date,
    /// How long, in whole minutes.
    #[personal(category = "employee_time", basis = "contract")]
    pub minutes: Minutes,
    /// A note in the person's words: plain text, never HTML.
    #[personal(category = "employee_time", basis = "contract")]
    pub note: String,
    /// Whether the time can be charged to a customer.
    #[personal(category = "employee_time", basis = "contract")]
    pub billable: bool,
    /// The moment the timer started, for an entry saved from a timer; none for *Add time*.
    #[personal(category = "employee_time", basis = "contract")]
    pub started_at: Option<Timestamp>,
    /// When the entry was made.
    #[personal(category = "employee_time", basis = "contract")]
    pub created_at: Timestamp,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// The timer of a person (TIME-01): one row for each person who ever started one, running while it
/// names an issue. One row per person is what makes one running timer for every device.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Timer {
    /// Whose timer it is.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,
    /// What it runs on, and since when; none while it stands still.
    #[personal(category = "employee_time", basis = "contract")]
    pub running: Option<Running>,
    /// The last moment it was started or switched: what the limit of changes a second counts from.
    #[personal(category = "employee_time", basis = "contract")]
    pub changed_at: Timestamp,
    /// Counts up with every change (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// A timer that runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Running {
    /// The issue it runs on.
    pub issue: IssueId,
    /// When it started (*Change start time* moves it).
    pub started_at: Timestamp,
    /// When the person was told that it runs long, if they were.
    pub warned_at: Option<Timestamp>,
}

impl Timer {
    /// The run, if the timer runs.
    #[must_use]
    pub const fn running(&self) -> Option<&Running> {
        self.running.as_ref()
    }
}

words! {
    /// The stable mark of a starter field of time entries (TIME-13, ADR-0035 decision 1): Bugaboo
    /// finds the field by it, never by its name, so a renamed field is still found.
    pub enum StarterField {
        /// *Activity*: what kind of work the time was.
        Activity => "activity",
        /// *Cost center*: where the time is charged.
        CostCenter => "cost-center",
    }
    messages name = "starter-field."
}

/// The choices *Activity* is made with, in their order.
const ACTIVITY_CHOICES: [MessageKey; 5] = [
    message::STARTER_FIELD_ACTIVITY_DEVELOPMENT,
    message::STARTER_FIELD_ACTIVITY_TESTING,
    message::STARTER_FIELD_ACTIVITY_DOCUMENTATION,
    message::STARTER_FIELD_ACTIVITY_MEETINGS,
    message::STARTER_FIELD_ACTIVITY_SUPPORT,
];

impl StarterField {
    /// The names of the choices the field is made with: the five kinds of work of *Activity*;
    /// *Cost center* starts empty, its choices are the organization's own.
    #[must_use]
    pub const fn starting_choices(self) -> &'static [MessageKey] {
        match self {
            Self::Activity => &ACTIVITY_CHOICES,
            Self::CostCenter => &[],
        }
    }
}

/// Which own field a starter field is: written once, when the field is made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Entity)]
pub struct StarterFieldMark {
    /// The mark.
    #[not_personal]
    pub starter: StarterField,
    /// The own field it marks.
    #[not_personal]
    pub field: FieldId,
}
