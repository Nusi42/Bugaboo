//! Migration 15: workflows as data (WF-01, ISS-02, PRJ-02, ADR-0031).
//!
//! A workflow has a name, statuses and status changes; where it is used is a row of its own, for a
//! project (none for the whole organization) and an issue type (none for all types). Nothing is
//! written into the tables: until somebody changes a workflow, no row exists and the starting
//! workflow *Simple*, defined once in `bb-core`, counts. The statuses of stored issues are keys
//! (`open`, `in-progress`, `done`) that *Simple* keeps, so no stored issue changes.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{
    IssueRow, IssueTypeRow, ProjectRow, WorkflowChangeRow, WorkflowRow, WorkflowStatusRow,
    WorkflowUseRow,
};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = workflows();
    steps.extend(contents());
    steps.extend(uses());
    steps.extend(issues());
    steps
}

fn workflows() -> Vec<Step> {
    vec![Step::Table(
        Table::create()
            .table(WorkflowRow::Table)
            .col(
                ColumnDef::new(WorkflowRow::Id)
                    .blob()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(WorkflowRow::Name).text().not_null())
            .col(
                ColumnDef::new(WorkflowRow::Version)
                    .big_integer()
                    .not_null(),
            )
            .to_owned(),
    )]
}

fn contents() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(WorkflowStatusRow::Table)
                .col(id_column(WorkflowStatusRow::Workflow))
                .col(ColumnDef::new(WorkflowStatusRow::Key).text().not_null())
                .col(ColumnDef::new(WorkflowStatusRow::Name).text().not_null())
                .col(
                    ColumnDef::new(WorkflowStatusRow::StatusGroup)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(WorkflowStatusRow::Position)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(WorkflowStatusRow::Workflow)
                        .col(WorkflowStatusRow::Key),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(WorkflowStatusRow::Table, WorkflowStatusRow::Workflow)
                        .to(WorkflowRow::Table, WorkflowRow::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(WorkflowChangeRow::Table)
                .col(id_column(WorkflowChangeRow::Workflow))
                .col(
                    ColumnDef::new(WorkflowChangeRow::FromStatus)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(WorkflowChangeRow::ToStatus)
                        .text()
                        .not_null(),
                )
                .col(ColumnDef::new(WorkflowChangeRow::Button).text().not_null())
                // The rules as JSON text: roles, groups and fields by their IDs and words.
                .col(ColumnDef::new(WorkflowChangeRow::Rules).text().not_null())
                .col(
                    ColumnDef::new(WorkflowChangeRow::Position)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(WorkflowChangeRow::Workflow)
                        .col(WorkflowChangeRow::FromStatus)
                        .col(WorkflowChangeRow::ToStatus),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(WorkflowChangeRow::Table, WorkflowChangeRow::Workflow)
                        .to(WorkflowRow::Table, WorkflowRow::Id),
                )
                .to_owned(),
        ),
    ]
}

fn uses() -> Vec<Step> {
    vec![
        // A place is a project and an issue type that can each be none, so the one row for a place
        // is kept by the program, inside the change that writes it, not by a unique key.
        Step::Table(
            Table::create()
                .table(WorkflowUseRow::Table)
                .col(ColumnDef::new(WorkflowUseRow::Project).blob())
                .col(ColumnDef::new(WorkflowUseRow::IssueType).blob())
                .col(id_column(WorkflowUseRow::Workflow))
                .foreign_key(
                    ForeignKey::create()
                        .from(WorkflowUseRow::Table, WorkflowUseRow::Project)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(WorkflowUseRow::Table, WorkflowUseRow::IssueType)
                        .to(IssueTypeRow::Table, IssueTypeRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(WorkflowUseRow::Table, WorkflowUseRow::Workflow)
                        .to(WorkflowRow::Table, WorkflowRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("workflow_use_by_workflow")
                .table(WorkflowUseRow::Table)
                .col(WorkflowUseRow::Workflow)
                .to_owned(),
        ),
    ]
}

/// What a change to the workflows looks up: the live issues of some projects or issue types that
/// are in some statuses.
fn issues() -> Vec<Step> {
    vec![
        Step::Index(
            Index::create()
                .name("issue_by_project_and_status")
                .table(IssueRow::Table)
                .col(IssueRow::Project)
                .col(IssueRow::Status)
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("issue_by_type_and_status")
                .table(IssueRow::Table)
                .col(IssueRow::IssueType)
                .col(IssueRow::Status)
                .to_owned(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use bb_core::id::{IssueId, IssueTypeId, ProjectId, UserId};
    use bb_core::workflow::starter;
    use bb_testing::{Failure, TestResult, each_database};
    use jiff::Timestamp;
    use sea_query::{Expr, Query};

    use super::super::Database;
    use crate::db::StoredId as _;
    use crate::schema::{IssueRow, IssueTypeRow, ProjectRow, WorkflowUseRow};
    use crate::test_support::at;

    /// A database as M3 left it: migrations 1 to 14, a project, an issue type, and one issue in
    /// each of the three statuses, written the way the program of M3 wrote them.
    async fn m3_database(
        database: &Database,
    ) -> Result<(Vec<(IssueId, &'static str, bool)>, ProjectId), Failure> {
        database.migrate_until(14, at(0)).await?;
        let project = ProjectId::from_parts(Timestamp::UNIX_EPOCH, [1; 10])?;
        let issue_type = IssueTypeId::from_parts(Timestamp::UNIX_EPOCH, [2; 10])?;
        let mut insert = Query::insert();
        insert
            .into_table(ProjectRow::Table)
            .columns([
                ProjectRow::Id,
                ProjectRow::ShortName,
                ProjectRow::Name,
                ProjectRow::Version,
            ])
            .values([
                Expr::val(project.stored()),
                Expr::val("ACME"),
                Expr::val("Acme"),
                Expr::val(1_i64),
            ])?;
        database.execute(&insert).await?;
        let mut insert = Query::insert();
        insert
            .into_table(IssueTypeRow::Table)
            .columns([
                IssueTypeRow::Id,
                IssueTypeRow::Name,
                IssueTypeRow::Icon,
                IssueTypeRow::Position,
                IssueTypeRow::Version,
            ])
            .values([
                Expr::val(issue_type.stored()),
                Expr::val("Task"),
                Expr::val("square-check"),
                Expr::val(0_i64),
                Expr::val(1_i64),
            ])?;
        database.execute(&insert).await?;
        let mut issues = Vec::new();
        for (number, (status, closed)) in [("open", false), ("in-progress", false), ("done", true)]
            .into_iter()
            .enumerate()
        {
            let id = IssueId::from_parts(Timestamp::UNIX_EPOCH, [10 + u8::try_from(number)?; 10])?;
            let mut insert = Query::insert();
            insert
                .into_table(IssueRow::Table)
                .columns([
                    IssueRow::Id,
                    IssueRow::Project,
                    IssueRow::IssueType,
                    IssueRow::Code,
                    IssueRow::Title,
                    IssueRow::Description,
                    IssueRow::Status,
                    IssueRow::Priority,
                    IssueRow::ReportedBy,
                    IssueRow::CreatedAt,
                    IssueRow::UpdatedAt,
                    IssueRow::ClosedAt,
                    IssueRow::Version,
                ])
                .values([
                    Expr::val(id.stored()),
                    Expr::val(project.stored()),
                    Expr::val(issue_type.stored()),
                    Expr::val(format!("{}", number + 1)),
                    Expr::val("An issue"),
                    Expr::val(""),
                    Expr::val(status),
                    Expr::val("normal"),
                    Expr::val(UserId::from_random([3; 16]).stored()),
                    Expr::val(1_i64),
                    Expr::val(1_i64),
                    Expr::val(closed.then_some(5_i64)),
                    Expr::val(1_i64),
                ])?;
            database.execute(&insert).await?;
            issues.push((id, status, closed));
        }
        Ok((issues, project))
    }

    // The rows of the "M3 database" are written by hand, up to migration 14. That is the same as a
    // database made by M3: the schema of M3 is the schema these migrations build and nothing else
    // (a database is its migrations, checksums included), and the rows are the ones M3 writes for
    // an issue: the status as one of the words *open*, *in-progress* and *done*, and a closed date
    // only for *done*. The migration under test reads nothing but those columns.
    #[tokio::test]
    async fn wf_01_an_m3_database_keeps_every_status_after_the_upgrade() -> TestResult {
        each_database("wf_01_m3_upgrade", async |db| {
            let database = Database::connect(db.url()).await?;
            let (issues, _) = m3_database(&database).await?;
            assert_eq!(database.migrate_until(15, at(1)).await?, [15]);
            // Every issue is as it was: its key, its closed date, its version.
            for (id, status, closed) in &issues {
                let stored = database
                    .stored_issue(*id)
                    .await?
                    .ok_or("an issue is gone")?;
                assert_eq!(stored.status.as_str(), *status);
                assert_eq!(stored.closed_at.is_some(), *closed);
                assert_eq!(stored.version, 1);
            }
            // No workflow was written: the starting workflow counts, and it has every key.
            let mut uses = Query::select();
            uses.column(WorkflowUseRow::Workflow)
                .from(WorkflowUseRow::Table);
            assert!(database.fetch_all(&uses).await?.is_empty());
            let simple = starter::SIMPLE.content();
            for (_, status, _) in &issues {
                assert!(
                    simple
                        .statuses
                        .iter()
                        .any(|each| each.key.as_str() == *status),
                    "{status}"
                );
            }
            Ok(())
        })
        .await
    }
}
