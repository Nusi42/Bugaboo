//! Migration 36: an index for the totals of the time of a project (TIME-03, PERF-01).
//!
//! The totals of a project sum *Estimated time* and *Time left* over its issues. Without this index
//! each of the hundreds of thousands of issues of a big project is read whole from the table, which
//! took seconds on a million issues (the benchmark `project-time-totals`, M6 WP-12); with it the
//! sums are read from the index alone. It holds the columns the sums and the filter of a project
//! read, and not the ID: with the ID in it SQLite chose the index for the text searches too, which
//! then took seconds (the benchmark `search-text-*`, M6 WP-12).

use sea_query::Index;

use super::Step;
use crate::schema::IssueRow;

pub(super) fn steps() -> Vec<Step> {
    vec![Step::Index(
        Index::create()
            .name("issue_by_project_for_totals")
            .table(IssueRow::Table)
            .col(IssueRow::Project)
            .col(IssueRow::DeletedAt)
            .col(IssueRow::EstimatedTime)
            .col(IssueRow::TimeLeft)
            .to_owned(),
    )]
}
