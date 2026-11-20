//! Migration 63: the timers by their issue (PRIV-05, ADR-0045, decision 2). An issue of the trash
//! goes only when no timer runs on it, and the purge of the trash and the count of what a changed
//! period would delete both ask that of each issue. Without an index, SQLite reads every timer for
//! every issue of the trash; with it, one look per issue.

use sea_query::Index;

use super::Step;
use crate::schema::TimerRow;

pub(super) fn steps() -> Vec<Step> {
    vec![Step::Index(
        Index::create()
            .name("timer_by_issue")
            .table(TimerRow::Table)
            .col(TimerRow::Issue)
            .to_owned(),
    )]
}
