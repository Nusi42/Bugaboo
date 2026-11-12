//! The tables of the plan of the timeline (PLAN-13, PLAN-14, ADR-0030 and its amendment M8): the
//! time factor of an issue and the plan warnings people accepted. Worked-out days have no table.

use super::{Iden, StoredTable};

/// The time factor of an issue: one row while it is not ×1. The issue is referenced, so the row
/// goes with the issue.
#[derive(Clone, Copy, Iden)]
pub(crate) enum TimeFactorRow {
    #[iden = "issue_time_factor"]
    Table,
    Issue,
    Percent,
    Person,
    SetBy,
}

/// The plan warnings people accepted: one row for each issue and kind.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AcceptedWarningRow {
    #[iden = "accepted_warning"]
    Table,
    Issue,
    Kind,
    StartDate,
    DueDate,
    AcceptedBy,
    AcceptedAt,
}

stored_tables! {
    TimeFactorRow => bb_core::plan::record::IssueTimeFactor {
        Issue => issue,
        Percent => factor,
        Person => person,
        SetBy => set_by,
    },
    AcceptedWarningRow => bb_core::plan::record::AcceptedWarning {
        Issue => issue,
        Kind => kind,
        StartDate => start_date,
        DueDate => due_date,
        AcceptedBy => accepted_by,
        AcceptedAt => accepted_at,
    },
}
