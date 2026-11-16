//! Migration 53: the index for the totals of a project also holds the two dates of an issue
//! (PLAN-01, PERF-01, M8 exit).
//!
//! Every page of the timeline counts the issues of its search that have neither *Start date* nor
//! *Due date* (*Not scheduled yet*). Without the dates in an index that count read each of the
//! hundreds of thousands of issues of a big project whole from the table: 650 ms on SQLite, more
//! than twice the budget of the page (the benchmark `timeline-biggest-project`). The index of
//! migration 36 (the project, deleted at, the estimate and the time left, for the sums of the
//! totals) is made again with the dates after them, so both the count and the sums read the index
//! alone, and an issue keeps one index of this kind to write. The columns of migration 36 stay first
//! and in their order, so the sums read what they read before. Like that index it holds no ID: with
//! the ID in it SQLite chose the index for the text searches too, which then took seconds (measured
//! again at the exit of M8: all three text searches stopped at the limit of a search). The statistics
//! of the issues are made again at once, as migration 23 does: until they name the new index SQLite
//! chooses badly (half of the counts of a search stopped at the limit until the round of the
//! statistics five minutes after the start).

use sea_query::Index;

use super::Step;
use crate::db::FixedSql;
use crate::schema::IssueRow;

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::DropIndex(
            Index::drop()
                .name("issue_by_project_for_totals")
                .table(IssueRow::Table)
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("issue_by_project_for_totals_and_dates")
                .table(IssueRow::Table)
                .col(IssueRow::Project)
                .col(IssueRow::DeletedAt)
                .col(IssueRow::EstimatedTime)
                .col(IssueRow::TimeLeft)
                .col(IssueRow::StartDate)
                .col(IssueRow::DueDate)
                .to_owned(),
        ),
        Step::Sql(FixedSql {
            sqlite: vec!["ANALYZE issue"],
            postgres: vec!["ANALYZE issue"],
        }),
    ]
}
