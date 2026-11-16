//! The port of time spent (ADR-0035, decision 4): how everything outside the module of time
//! entries reads the minutes booked on issues. The store implements it; the issue's own time and
//! the comparison of the timesheet with working hours read it. The totals over a search read the
//! time spent with the other lengths, in the one statement of the search (`search_lengths`,
//! ADR-0035, amendment M8).
//!
//! The port decides nothing: it reads only the issues the caller passes and one person's own
//! dates. Who may read what is decided by the use case first.

use std::future::Future;

use bb_core::id::{IssueId, UserId};
use bb_core::time::Minutes;
use jiff::civil::Date;

use crate::ports::ChangeStore;

/// The minutes booked on issues, for the readers outside the module of time entries.
pub trait TimeSpentStore: ChangeStore + 'static {
    /// The minutes booked on each issue of `issues`, one row for each, `Minutes(0)` for an
    /// issue without entries. Reads only the issues in the list.
    fn spent_on_issues(
        &self,
        issues: &[IssueId],
    ) -> impl Future<Output = Result<Vec<(IssueId, Minutes)>, Self::Error>> + Send;

    /// The minutes `person` booked on each date from `first` to `last` (both included), by
    /// the date of each entry; a date without entries is left out.
    fn booked_per_date(
        &self,
        person: UserId,
        first: Date,
        last: Date,
    ) -> impl Future<Output = Result<Vec<(Date, Minutes)>, Self::Error>> + Send;
}
