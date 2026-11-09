//! Migration 47: the project of the issue on a piece of agent work (M7-F127).
//!
//! `issue_work.project` holds the project of the issue, so the projects an agent reaches are read
//! from the pieces of its work alone, not by looking up every issue of its work in the table of
//! the issues at every request. An issue stays in its project for good, so the column is written
//! once when the work is made. The rows that are there get the project of their issue.

use sea_query::{ColumnDef, Table};

use super::Step;
use crate::db::FixedSql;
use crate::schema::IssueWorkRow;

const FILL: &str =
    "UPDATE issue_work SET project = (SELECT project FROM issue WHERE issue.id = issue_work.issue)";

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Alter(
            Table::alter()
                .table(IssueWorkRow::Table)
                .add_column(ColumnDef::new(IssueWorkRow::Project).blob())
                .to_owned(),
        ),
        Step::Sql(FixedSql {
            sqlite: vec![FILL],
            postgres: vec![FILL],
        }),
    ]
}
