//! The port of time spent (ADR-0035, decision 4): how everything outside the module of time
//! entries reads the minutes booked on issues. The store implements it; the totals and the
//! comparison of the timesheet with working hours read it.
//!
//! The port decides nothing: it reads only the issues the caller passes or the compiled search
//! finds, and one person's own dates. Who may read what is decided by the use case first.

use std::future::Future;

use bb_core::id::{IssueId, UserId};
use bb_core::time::Minutes;
use jiff::civil::Date;

use crate::ports::ChangeStore;
use crate::search::{SearchFailure, SearchRequest};

/// The minutes booked on issues, for the readers outside the module of time entries.
pub trait TimeSpentStore: ChangeStore + 'static {
    /// The minutes booked on each issue of `issues`, one row for each, `Minutes(0)` for an
    /// issue without entries. Reads only the issues in the list.
    fn spent_on_issues(
        &self,
        issues: &[IssueId],
    ) -> impl Future<Output = Result<Vec<(IssueId, Minutes)>, Self::Error>> + Send;

    /// The sum of the minutes booked on the issues that `request` finds: the compiled search
    /// of the list, with the reader's sight and fields in it, in one statement. Like
    /// `search_count`, it ignores `after` and `limit`.
    fn spent_on_search(
        &self,
        request: &SearchRequest,
    ) -> impl Future<Output = Result<Minutes, SearchFailure>> + Send;

    /// The minutes `person` booked on each date from `first` to `last` (both included), by
    /// the date of each entry; a date without entries is left out.
    fn booked_per_date(
        &self,
        person: UserId,
        first: Date,
        last: Date,
    ) -> impl Future<Output = Result<Vec<(Date, Minutes)>, Self::Error>> + Send;
}
