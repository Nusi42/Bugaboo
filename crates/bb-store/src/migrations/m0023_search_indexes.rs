//! Migration 23 (provisional number: the lead renumbers it at the merge): the indexes that the
//! search of M5 WP-04 needs on the large dataset of PERF-01 (1 M issues, 500 projects), found by
//! the benchmark and not by guessing.
//!
//! - The newest issues first, across the projects the person reads: a search that is cut after its
//!   first rows reads the issues in the order of their creation and stops when it has them.
//! - The newest issues of one project, which the list of a project is: no sort of a big project.
//! - The columns the filters and the sight of a count or a list of groups read, so they are
//!   answered from the index alone (the project, the status, the priority, whether the issue is in
//!   the trash and who reported it), without reading 1 M issues. The ID is not in it on purpose:
//!   with it SQLite prefers this index to the two of the creation and sorts a whole project.

use sea_query::Index;

use super::Step;
use crate::db::FixedSql;
use crate::schema::IssueRow;

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Index(
            Index::create()
                .name("issue_by_created")
                .table(IssueRow::Table)
                .col(IssueRow::CreatedAt)
                .col(IssueRow::Id)
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("issue_by_project_and_created")
                .table(IssueRow::Table)
                .col(IssueRow::Project)
                .col(IssueRow::CreatedAt)
                .col(IssueRow::Id)
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("issue_by_filters")
                .table(IssueRow::Table)
                .col(IssueRow::Project)
                .col(IssueRow::Status)
                .col(IssueRow::Priority)
                .col(IssueRow::DeletedAt)
                .col(IssueRow::ReportedBy)
                .to_owned(),
        ),
        // SQLite chooses between these indexes by what it knows of the table: without it, it reads
        // a whole project through the index of the project and sorts it, where the index of the
        // creation gives the newest page at once (2.9 s against 2 ms on 1 M issues). PostgreSQL
        // keeps its statistics itself.
        // SQLite only: the index of the project and the ID goes. Every read that has the project
        // finds its rows by the other indexes that lead with the project (the code, the creation,
        // the filters), so none loses its index; and with it SQLite looked every issue found by
        // its words up in each of the 500 projects of a search instead of by its ID (17 s against
        // 0.5 s on 1 M issues). It changes what is fast and never what is found, so the results
        // stay the same on both databases (ARCH-DB-PARITY is about results). PostgreSQL plans from
        // its own statistics and keeps the index.
        Step::Sql(FixedSql {
            sqlite: vec!["DROP INDEX IF EXISTS issue_by_project", "ANALYZE issue"],
            postgres: vec!["ANALYZE issue"],
        }),
    ]
}
