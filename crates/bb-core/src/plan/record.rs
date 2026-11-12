//! What the plan keeps (migration 52): the time factor of an issue with who set it, and the plan
//! warnings people accepted (*Keep and accept*). Worked-out days are never kept.

use jiff::Timestamp;
use jiff::civil::Date;

use super::TimeFactor;
use super::warning::WarningKind;
use crate::id::{IssueId, UserId};
use crate::personal::Entity;

/// The time factor of an issue, kept only while it is not ×1 (ADR-0030, decision 4 and amendment
/// M8). It is the estimate of the person on the bar it was set for, and goes away in the change
/// that gives the issue another first assigned person.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct IssueTimeFactor {
    /// The issue.
    #[not_personal]
    pub issue: IssueId,
    /// The factor.
    #[personal(category = "employee_time", basis = "contract")]
    pub factor: TimeFactor,
    /// The person on the bar it was set for: the first assigned person when it was set.
    #[personal(category = "employee_time", basis = "contract")]
    pub person: UserId,
    /// Who set it: the person on the bar (*Lea's own estimate*) or somebody else (*set by Ada
    /// Brandt*).
    #[personal(category = "employee_time", basis = "contract")]
    pub set_by: UserId,
}

/// A plan warning somebody accepted (*Keep and accept*, ADR-0030, amendment M8): it shows faded to
/// everybody who sees the issue while the issue keeps the dates it was accepted for, and comes back
/// when they change.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct AcceptedWarning {
    /// The issue.
    #[not_personal]
    pub issue: IssueId,
    /// The kind of the warning.
    #[not_personal]
    pub kind: WarningKind,
    /// The *Start date* of the issue when it was accepted.
    #[not_personal]
    pub start_date: Option<Date>,
    /// The *Due date* of the issue when it was accepted.
    #[not_personal]
    pub due_date: Option<Date>,
    /// Who accepted it.
    #[personal(category = "identity", basis = "contract")]
    pub accepted_by: UserId,
    /// When.
    #[not_personal]
    pub accepted_at: Timestamp,
}

impl AcceptedWarning {
    /// Whether it still counts for an issue with these dates: only while they are the dates it was
    /// accepted for.
    #[must_use]
    pub fn holds_for(&self, start_date: Option<Date>, due_date: Option<Date>) -> bool {
        self.start_date == start_date && self.due_date == due_date
    }
}
