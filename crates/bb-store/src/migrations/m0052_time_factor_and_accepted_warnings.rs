//! Migration 52: what the plan of the timeline keeps (PLAN-01, PLAN-10, PLAN-13, PLAN-14, ADR-0030
//! and its amendment M8, ADR-0034, ADR-0043, decision 2).
//!
//! - The time factor of an issue: one row while it is not ×1, with the person on the bar it is for
//!   and who set it. Every issue made before has none, so ×1.
//! - The plan warnings people accepted (*Keep and accept*): one row for each issue and kind, with
//!   the dates of the issue it was accepted for and who accepted it.
//! - The place of each assigned person, so the first one (the person on the bar) is the one a
//!   person put first. The people assigned before all get place 0 and keep the order they had (by
//!   when they were assigned, then by ID).
//! - The options of a timeline view on a saved search: the two date fields by their search words
//!   (none for *Start date* and *Due date*), *Plan by effort* (off) and the zoom (a week). Every
//!   search kept before starts with these.
//!
//! One `ALTER TABLE … ADD COLUMN` for each column, because SQLite adds one column at a time. No
//! worked-out day of a bar is kept anywhere.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{
    AcceptedWarningRow, AssignedPersonRow, IssueRow, SavedSearchRow, TimeFactorRow,
};

/// The zoom every saved search kept before starts with, as it is stored.
const WEEK: &str = "week";

pub(super) fn steps() -> Vec<Step> {
    let mut position = ColumnDef::new(AssignedPersonRow::Position);
    position.big_integer().not_null().default(0);
    let mut plan_by_effort = ColumnDef::new(SavedSearchRow::PlanByEffort);
    plan_by_effort.boolean().not_null().default(false);
    let mut zoom = ColumnDef::new(SavedSearchRow::Zoom);
    zoom.text().not_null().default(WEEK);
    let add = |table: SavedSearchRow, column: ColumnDef| {
        Step::Alter(Table::alter().table(table).add_column(column).to_owned())
    };
    vec![
        Step::Table(
            Table::create()
                .table(TimeFactorRow::Table)
                .col(
                    ColumnDef::new(TimeFactorRow::Issue)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(TimeFactorRow::Percent)
                        .big_integer()
                        .not_null(),
                )
                .col(id_column(TimeFactorRow::Person))
                .col(id_column(TimeFactorRow::SetBy))
                .foreign_key(
                    ForeignKey::create()
                        .from(TimeFactorRow::Table, TimeFactorRow::Issue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(AcceptedWarningRow::Table)
                .col(id_column(AcceptedWarningRow::Issue))
                .col(ColumnDef::new(AcceptedWarningRow::Kind).text().not_null())
                .col(ColumnDef::new(AcceptedWarningRow::StartDate).big_integer())
                .col(ColumnDef::new(AcceptedWarningRow::DueDate).big_integer())
                .col(id_column(AcceptedWarningRow::AcceptedBy))
                .col(
                    ColumnDef::new(AcceptedWarningRow::AcceptedAt)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(AcceptedWarningRow::Issue)
                        .col(AcceptedWarningRow::Kind),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(AcceptedWarningRow::Table, AcceptedWarningRow::Issue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(AssignedPersonRow::Table)
                .add_column(position)
                .to_owned(),
        ),
        add(
            SavedSearchRow::Table,
            ColumnDef::new(SavedSearchRow::TimelineStart)
                .text()
                .to_owned(),
        ),
        add(
            SavedSearchRow::Table,
            ColumnDef::new(SavedSearchRow::TimelineEnd)
                .text()
                .to_owned(),
        ),
        add(SavedSearchRow::Table, plan_by_effort),
        add(SavedSearchRow::Table, zoom),
    ]
}

#[cfg(test)]
mod tests {
    use bb_app::saved_searches::SavedSearchStore as _;
    use bb_core::id::{IssueId, IssueTypeId, ProjectId, SavedSearchId, UserId};
    use bb_core::saved_search::{TimelineOptions, Zoom};
    use bb_testing::{TestResult, each_database};
    use jiff::Timestamp;
    use sea_query::{Expr, ExprTrait as _, IntoIden as _, Order, Query};

    use super::super::Database;
    use super::super::old_rows::insert;
    use crate::auth::AuthStorage;
    use crate::db::StoredId as _;
    use crate::schema::{AssignedPersonRow, SavedSearchRow};
    use crate::test_support::{at, insert_old_issue, insert_old_issue_type, insert_old_project};

    #[tokio::test]
    async fn plan_10_migration_52_keeps_the_order_of_assigned_people_and_starts_every_timeline()
    -> TestResult {
        each_database("migration_52_the_plan", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(51, at(0)).await?;
            let issue_type = IssueTypeId::from_parts(Timestamp::UNIX_EPOCH, [2; 10])?;
            insert_old_issue_type(&database, issue_type, "Task").await?;
            let project = ProjectId::from_parts(Timestamp::UNIX_EPOCH, [3; 10])?;
            insert_old_project(&database, project.stored()).await?;
            let issue = IssueId::from_parts(Timestamp::UNIX_EPOCH, [4; 10])?;
            insert_old_issue(
                &database,
                (project.stored(), issue.stored()),
                issue_type,
                "1",
            )
            .await?;
            let (lea, ben) = (UserId::from_random([9; 16]), UserId::from_random([8; 16]));
            // Lea was assigned first, Ben later.
            for (person, added) in [(lea, 1_i64), (ben, 2)] {
                insert(
                    &database,
                    AssignedPersonRow::Table,
                    [
                        AssignedPersonRow::Issue.into_iden(),
                        AssignedPersonRow::Person.into_iden(),
                        AssignedPersonRow::AddedAt.into_iden(),
                    ],
                    [
                        Expr::val(issue.stored()),
                        Expr::val(person.stored()),
                        Expr::val(added),
                    ],
                )
                .await?;
            }
            let search = SavedSearchId::from_parts(Timestamp::UNIX_EPOCH, [5; 10])?;
            insert(
                &database,
                SavedSearchRow::Table,
                [
                    SavedSearchRow::Id.into_iden(),
                    SavedSearchRow::Name.into_iden(),
                    SavedSearchRow::Text.into_iden(),
                    SavedSearchRow::Maker.into_iden(),
                    SavedSearchRow::View.into_iden(),
                    SavedSearchRow::CreatedAt.into_iden(),
                    SavedSearchRow::UpdatedAt.into_iden(),
                    SavedSearchRow::Version.into_iden(),
                ],
                [
                    Expr::val(search.stored()),
                    Expr::val("Open bugs"),
                    Expr::val("status is open"),
                    Expr::val(lea.stored()),
                    Expr::val("list"),
                    Expr::val(1_i64),
                    Expr::val(1_i64),
                    Expr::val(1_i64),
                ],
            )
            .await?;
            assert_eq!(database.migrate_until(52, at(1)).await?, [52]);

            // Both people have place 0, so the order they had stays: Lea first.
            let mut read = Query::select();
            read.columns([AssignedPersonRow::Person, AssignedPersonRow::Position])
                .from(AssignedPersonRow::Table)
                .and_where(Expr::col(AssignedPersonRow::Issue).eq(issue.stored()))
                .order_by(AssignedPersonRow::Position, Order::Asc)
                .order_by(AssignedPersonRow::AddedAt, Order::Asc);
            let rows = database.fetch_all(&read).await?;
            let people: Vec<(UserId, i64)> = rows
                .iter()
                .map(|row| {
                    Ok((
                        row.id::<UserId>(AssignedPersonRow::Person)?,
                        row.get::<i64>(AssignedPersonRow::Position)?,
                    ))
                })
                .collect::<Result<_, crate::StoreError>>()?;
            assert_eq!(people, [(lea, 0), (ben, 0)]);
            // The search starts with the built-in dates, without plan by effort, a week wide.
            let mut read = database.begin_read().await?;
            let kept = AuthStorage::saved_search_in(&mut read, search)
                .await?
                .ok_or("the saved search is gone")?;
            assert_eq!(
                kept.timeline,
                TimelineOptions {
                    dates: None,
                    plan_by_effort: false,
                    zoom: Zoom::Week,
                }
            );
            Ok(())
        })
        .await
    }
}
