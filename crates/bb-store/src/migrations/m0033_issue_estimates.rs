//! Migration 33: an issue gets *Estimated time* and *Time left* (TIME-03, ADR-0035, decision 4).
//!
//! Both are whole minutes and empty until somebody gives them; an empty *Time left* is worked out
//! from the estimate and the time spent (`bb_core::time::left`). Every issue made before keeps
//! what it had and has neither. Two `ALTER TABLE … ADD COLUMN`, one for each column, because
//! SQLite adds one column at a time. No index yet: the benchmark decides whether sorting by them
//! needs one, the way migration 23 was found.

use sea_query::{ColumnDef, Table};

use super::Step;
use crate::schema::IssueRow;

pub(super) fn steps() -> Vec<Step> {
    [IssueRow::EstimatedTime, IssueRow::TimeLeft]
        .into_iter()
        .map(|column| {
            Step::Alter(
                Table::alter()
                    .table(IssueRow::Table)
                    .add_column(ColumnDef::new(column).big_integer())
                    .to_owned(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bb_core::id::{IssueId, IssueTypeId, ProjectId};
    use bb_testing::{TestResult, each_database};
    use jiff::Timestamp;
    use sea_query::{Expr, ExprTrait as _};

    use super::super::{Database, latest_version};
    use crate::db::StoredId as _;
    use crate::issues::{issue_select, read_issue};
    use crate::schema::IssueRow;
    use crate::test_support::{at, insert_old_issue, insert_old_issue_type, insert_old_project};

    #[tokio::test]
    async fn time_03_migration_33_gives_every_issue_an_empty_estimate_and_time_left() -> TestResult
    {
        each_database("migration_33_issue_estimates", async |db| {
            let database = Database::connect(db.url()).await?;
            // Every migration before this one, whatever their numbers are.
            database.migrate_until(32, at(0)).await?;
            let issue_type = IssueTypeId::from_parts(Timestamp::UNIX_EPOCH, [2; 10])?;
            insert_old_issue_type(&database, issue_type, "Task").await?;
            let issue = IssueId::from_parts(Timestamp::UNIX_EPOCH, [4; 10])?;
            let project = ProjectId::from_parts(Timestamp::UNIX_EPOCH, [3; 10])?;
            insert_old_project(&database, project.stored()).await?;
            insert_old_issue(
                &database,
                (project.stored(), issue.stored()),
                issue_type,
                "1",
            )
            .await?;
            let applied = database.migrate_until(33, at(1)).await?;
            assert_eq!(applied.last(), Some(&33));
            assert!(latest_version() >= 33);
            // The issue is read with the columns of the program as it is now, so the migrations
            // after this one are applied before it is read.
            database.migrate(at(2)).await?;

            // The issue reads as it was, with no estimate and no typed time left.
            let mut read = issue_select();
            read.and_where(Expr::col(IssueRow::Id).eq(issue.stored()));
            let row = database.fetch_optional(&read).await?;
            let kept = read_issue(&row.ok_or("the issue is gone")?)?;
            assert_eq!(kept.title, "An issue");
            assert_eq!((kept.estimated_time, kept.time_left), (None, None));
            Ok(())
        })
        .await
    }
}
