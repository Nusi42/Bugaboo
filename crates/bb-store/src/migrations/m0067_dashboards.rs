//! Migration 67: dashboards (CFG-03, ADR-0050).
//!
//! A dashboard, its boxes in order and the groups and projects it is shared with. The maker has no
//! reference to the person (a removed person leaves a tombstone, PRIV-04); the index of the maker
//! is the one the walk of a person's data and the limit of dashboards ask. A box keeps its kind,
//! its place on the grid, its title, what it reads (a search text or the ID of a saved search,
//! with no reference: a box whose saved search is gone is left out) and the options of its kind.
//! The rows of `dashboard_share` have no reference for their target, which is the ID of a group or
//! of a project, told apart by `kind`, like the shares of saved searches.
//!
//! Last, a server whose starting roles were made already gets a row that *Share dashboards* is due
//! (ADR-0025, amendment 2026-11-24): the next start gives it to the starting roles that hold it,
//! with its events and its line in the security log, as migration 65 does for *Manage releases*.
//! The key of the permission is written out here, not read from the program, so the migration
//! stays what it was.

use sea_query::{ColumnDef, Expr, ForeignKey, Index, Query, Table};

use super::{Step, id_column};
use crate::schema::{
    DashboardBoxRow, DashboardRow, DashboardShareRow, StartingPermissionDueRow,
    StartingRolesMadeRow,
};

/// The key of the permission *Share dashboards*, as it is stored.
const SHARE_DASHBOARDS: &str = "share-dashboards";

fn dashboards() -> Vec<Step> {
    let moment = |column: DashboardRow| ColumnDef::new(column).big_integer().not_null().to_owned();
    vec![
        Step::Table(
            Table::create()
                .table(DashboardRow::Table)
                .col(
                    ColumnDef::new(DashboardRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(DashboardRow::Name).text().not_null())
                .col(id_column(DashboardRow::Maker))
                .col(ColumnDef::new(DashboardRow::Home).boolean().not_null())
                .col(moment(DashboardRow::CreatedAt))
                .col(moment(DashboardRow::UpdatedAt))
                .col(ColumnDef::new(DashboardRow::DeletedAt).big_integer())
                .col(moment(DashboardRow::Version))
                .to_owned(),
        ),
        // The dashboards of one person, oldest first.
        Step::Index(
            Index::create()
                .name("dashboard_by_maker")
                .table(DashboardRow::Table)
                .col(DashboardRow::Maker)
                .col(DashboardRow::Id)
                .to_owned(),
        ),
    ]
}

fn boxes() -> Vec<Step> {
    let number =
        |column: DashboardBoxRow| ColumnDef::new(column).big_integer().not_null().to_owned();
    let text = |column: DashboardBoxRow| ColumnDef::new(column).text().to_owned();
    vec![Step::Table(
        Table::create()
            .table(DashboardBoxRow::Table)
            .col(id_column(DashboardBoxRow::Dashboard))
            .col(id_column(DashboardBoxRow::Id))
            .col(number(DashboardBoxRow::Position))
            .col(ColumnDef::new(DashboardBoxRow::Kind).text().not_null())
            .col(number(DashboardBoxRow::GridColumn))
            .col(number(DashboardBoxRow::GridRow))
            .col(number(DashboardBoxRow::Width))
            .col(number(DashboardBoxRow::Height))
            .col(ColumnDef::new(DashboardBoxRow::Title).text().not_null())
            .col(text(DashboardBoxRow::SearchText))
            .col(ColumnDef::new(DashboardBoxRow::SavedSearch).blob())
            .col(text(DashboardBoxRow::ChartBy))
            .col(text(DashboardBoxRow::ChartShape))
            .col(text(DashboardBoxRow::TimeSpan))
            .primary_key(
                Index::create()
                    .col(DashboardBoxRow::Dashboard)
                    .col(DashboardBoxRow::Position),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(DashboardBoxRow::Table, DashboardBoxRow::Dashboard)
                    .to(DashboardRow::Table, DashboardRow::Id),
            )
            .to_owned(),
    )]
}

fn shares() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(DashboardShareRow::Table)
                .col(id_column(DashboardShareRow::Dashboard))
                .col(ColumnDef::new(DashboardShareRow::Kind).text().not_null())
                .col(id_column(DashboardShareRow::Target))
                .col(
                    ColumnDef::new(DashboardShareRow::SharedAt)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(DashboardShareRow::Dashboard)
                        .col(DashboardShareRow::Kind)
                        .col(DashboardShareRow::Target),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(DashboardShareRow::Table, DashboardShareRow::Dashboard)
                        .to(DashboardRow::Table, DashboardRow::Id),
                )
                .to_owned(),
        ),
        // The dashboards shared with one group or project.
        Step::Index(
            Index::create()
                .name("dashboard_share_by_target")
                .table(DashboardShareRow::Table)
                .col(DashboardShareRow::Kind)
                .col(DashboardShareRow::Target)
                .col(DashboardShareRow::Dashboard)
                .to_owned(),
        ),
    ]
}

/// The row that *Share dashboards* is due, when the starting roles were made already.
fn due_permission() -> Result<Step, sea_query::error::Error> {
    let mut made = Query::select();
    made.expr(Expr::val(SHARE_DASHBOARDS))
        .from(StartingRolesMadeRow::Table)
        .limit(1);
    let mut due = Query::insert();
    due.into_table(StartingPermissionDueRow::Table)
        .columns([StartingPermissionDueRow::Permission])
        .select_from(made)?;
    Ok(Step::Seed(due))
}

pub(super) fn steps() -> Result<Vec<Step>, sea_query::error::Error> {
    let mut steps = dashboards();
    steps.extend(boxes());
    steps.extend(shares());
    steps.push(due_permission()?);
    Ok(steps)
}

#[cfg(test)]
mod tests {
    use bb_core::id::RoleId;
    use bb_testing::{TestResult, each_database};
    use jiff::Timestamp;
    use sea_query::{Expr, IntoIden as _, Query};

    use super::super::Database;
    use super::super::old_rows::{insert, old_role};
    use crate::schema::{DashboardBoxRow, StartingPermissionDueRow, StartingRolesMadeRow};
    use crate::test_support::at;

    /// The keys of the permissions that are due.
    async fn due(database: &Database) -> Result<Vec<String>, bb_testing::Failure> {
        let mut due = Query::select();
        due.column(StartingPermissionDueRow::Permission)
            .from(StartingPermissionDueRow::Table);
        Ok(database
            .fetch_all(&due)
            .await?
            .iter()
            .map(|row| row.get::<String>(StartingPermissionDueRow::Permission))
            .collect::<Result<Vec<_>, _>>()?)
    }

    // The tables come with migration 67, refuse a box of a dashboard that is not there, and mark
    // *Share dashboards* due where the starting roles were made, beside *Manage releases*.
    #[tokio::test]
    async fn cfg_03_migration_67_makes_the_tables_and_marks_share_dashboards_due() -> TestResult {
        each_database("migration_67_dashboards", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(64, at(0)).await?;
            old_role(
                &database,
                RoleId::from_parts(Timestamp::UNIX_EPOCH, [1; 10])?,
                0,
            )
            .await?;
            insert(
                &database,
                StartingRolesMadeRow::Table,
                [StartingRolesMadeRow::MadeAt.into_iden()],
                [Expr::val(0_i64)],
            )
            .await?;
            assert_eq!(database.migrate_until(67, at(1)).await?, [65, 66, 67]);
            assert_eq!(
                due(&database).await?,
                ["manage-releases", "share-dashboards"]
            );
            let mut orphan = Query::insert();
            orphan
                .into_table(DashboardBoxRow::Table)
                .columns([
                    DashboardBoxRow::Dashboard,
                    DashboardBoxRow::Id,
                    DashboardBoxRow::Position,
                    DashboardBoxRow::Kind,
                    DashboardBoxRow::GridColumn,
                    DashboardBoxRow::GridRow,
                    DashboardBoxRow::Width,
                    DashboardBoxRow::Height,
                    DashboardBoxRow::Title,
                ])
                .values_panic([
                    Expr::val(vec![7_u8; 16]),
                    Expr::val(vec![8_u8; 16]),
                    Expr::val(0_i64),
                    Expr::val("my-timer"),
                    Expr::val(0_i64),
                    Expr::val(0_i64),
                    Expr::val(3_i64),
                    Expr::val(2_i64),
                    Expr::val("Timer"),
                ]);
            assert!(database.execute(&orphan).await.is_err());
            Ok(())
        })
        .await
    }
}
