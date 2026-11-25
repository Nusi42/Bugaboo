//! Migration 65: the releases of a project (ISS-08, ADR-0049).
//!
//! One row for each release: the project it was made in, its name, its *Release date* and
//! *Released on* (calendar days kept as the days since 1970-01-01, like *Due date*), its state, who
//! made and last changed it (no reference to the person: a removed person leaves a tombstone,
//! PRIV-04) and when it went to the trash. An issue names the one release its fix is *Planned for*
//! in a column of its own, empty for every issue made before, and the releases it was *Found in* in
//! a table of its own, in the order given. The indexes: the releases of a project, the issues of a
//! release (to refuse the delete of a release that issues name, and to search by it), and each
//! column that names a person, which the walk of a person's data asks (ADR-0044).
//!
//! Last, a server whose starting roles were made already gets a row that *Manage releases* is due
//! (ADR-0025, amendment 2026-11-24): like a starting role that came later (migration 32), the
//! permission is given by the use case of the starting roles at the next start, with its events
//! and its line in the security log, never here. The key of the permission is written out here,
//! not read from the program, so the migration stays what it was.
//!
//! Then the layouts of the issue page saved before get the two fields where the starting layout
//! has them (`release_layouts.rs`), so an upgraded layout nobody changed is the starting layout.

use sea_query::{ColumnDef, Expr, ForeignKey, Index, Query, Table};

use super::{Step, id_column};
use crate::schema::{
    IssueFoundInRow, IssueRow, ReleaseRow, StartingPermissionDueRow, StartingRolesMadeRow,
};

/// The key of the permission *Manage releases*, as it is stored.
const MANAGE_RELEASES: &str = "manage-releases";

/// The table of the releases and its indexes, one of them for each column that names a person.
fn release_table() -> Vec<Step> {
    let day = |column: ReleaseRow| ColumnDef::new(column).big_integer().to_owned();
    let moment = |column: ReleaseRow| ColumnDef::new(column).big_integer().not_null().to_owned();
    let index = |name: &str, column: ReleaseRow| {
        Step::Index(
            Index::create()
                .name(name)
                .table(ReleaseRow::Table)
                .col(column)
                .to_owned(),
        )
    };
    vec![
        Step::Table(
            Table::create()
                .table(ReleaseRow::Table)
                .col(
                    ColumnDef::new(ReleaseRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(id_column(ReleaseRow::Project))
                .col(ColumnDef::new(ReleaseRow::Name).text().not_null())
                .col(day(ReleaseRow::ReleaseDate))
                .col(ColumnDef::new(ReleaseRow::State).text().not_null())
                .col(day(ReleaseRow::ReleasedOn))
                .col(id_column(ReleaseRow::MadeBy))
                .col(moment(ReleaseRow::MadeAt))
                .col(id_column(ReleaseRow::ChangedBy))
                .col(moment(ReleaseRow::ChangedAt))
                .col(ColumnDef::new(ReleaseRow::DeletedAt).big_integer())
                .col(ColumnDef::new(ReleaseRow::Version).big_integer().not_null())
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("release_by_project")
                .table(ReleaseRow::Table)
                .col(ReleaseRow::Project)
                .col(ReleaseRow::Id)
                .to_owned(),
        ),
        index("release_by_maker", ReleaseRow::MadeBy),
        index("release_by_changer", ReleaseRow::ChangedBy),
    ]
}

/// The column of *Planned for release* and the table of *Found in release*, with their indexes.
fn issue_releases() -> Vec<Step> {
    vec![
        Step::Alter(
            Table::alter()
                .table(IssueRow::Table)
                .add_column(ColumnDef::new(IssueRow::PlannedForRelease).blob())
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("issue_by_planned_release")
                .table(IssueRow::Table)
                .col(IssueRow::PlannedForRelease)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(IssueFoundInRow::Table)
                .col(id_column(IssueFoundInRow::Issue))
                .col(id_column(IssueFoundInRow::Release))
                .col(
                    ColumnDef::new(IssueFoundInRow::Position)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(IssueFoundInRow::Issue)
                        .col(IssueFoundInRow::Release),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(IssueFoundInRow::Table, IssueFoundInRow::Issue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(IssueFoundInRow::Table, IssueFoundInRow::Release)
                        .to(ReleaseRow::Table, ReleaseRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("issue_found_in_release_by_release")
                .table(IssueFoundInRow::Table)
                .col(IssueFoundInRow::Release)
                .col(IssueFoundInRow::Issue)
                .to_owned(),
        ),
    ]
}

pub(super) fn steps() -> Result<Vec<Step>, sea_query::error::Error> {
    let mut steps = release_table();
    steps.extend(issue_releases());
    steps.extend(due_permission()?);
    steps.extend(super::release_layouts::steps()?);
    Ok(steps)
}

/// The table of the permissions still to give to the starting roles, with *Manage releases* in
/// it when the starting roles were made already: the next start gives it (`access/starting.rs`).
fn due_permission() -> Result<Vec<Step>, sea_query::error::Error> {
    let mut made = Query::select();
    made.expr(Expr::val(MANAGE_RELEASES))
        .from(StartingRolesMadeRow::Table)
        .limit(1);
    let mut due = Query::insert();
    due.into_table(StartingPermissionDueRow::Table)
        .columns([StartingPermissionDueRow::Permission])
        .select_from(made)?;
    Ok(vec![
        Step::Table(
            Table::create()
                .table(StartingPermissionDueRow::Table)
                .col(
                    ColumnDef::new(StartingPermissionDueRow::Permission)
                        .text()
                        .not_null()
                        .primary_key(),
                )
                .to_owned(),
        ),
        Step::Seed(due),
    ])
}

#[cfg(test)]
mod tests {
    use bb_core::id::RoleId;
    use bb_testing::{TestResult, each_database};
    use jiff::Timestamp;
    use sea_query::{Expr, ExprTrait as _, IntoIden as _, Query};

    use super::super::old_rows::{insert, old_role};
    use super::super::{Database, latest_version};
    use super::MANAGE_RELEASES;
    use crate::schema::{RolePermissionRow, StartingPermissionDueRow, StartingRolesMadeRow};
    use crate::test_support::at;

    /// The keys of the permissions that are due, and how many roles hold *Manage releases*.
    async fn due_and_held(
        database: &Database,
    ) -> Result<(Vec<String>, usize), bb_testing::Failure> {
        let mut due = Query::select();
        due.column(StartingPermissionDueRow::Permission)
            .from(StartingPermissionDueRow::Table);
        let keys = database
            .fetch_all(&due)
            .await?
            .iter()
            .map(|row| row.get::<String>(StartingPermissionDueRow::Permission))
            .collect::<Result<Vec<_>, _>>()?;
        let mut held = Query::select();
        held.column(RolePermissionRow::Role)
            .from(RolePermissionRow::Table)
            .and_where(Expr::col(RolePermissionRow::Permission).eq(MANAGE_RELEASES));
        Ok((keys, database.fetch_all(&held).await?.len()))
    }

    #[tokio::test]
    async fn iss_08_migration_65_marks_manage_releases_due_where_the_starting_roles_were_made()
    -> TestResult {
        each_database("migration_65_releases", async |db| {
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
            let applied = database.migrate_until(65, at(1)).await?;
            assert_eq!(applied.last(), Some(&65));
            assert!(latest_version() >= 65);
            // The permission is due and given by the next start, never by the migration.
            assert_eq!(
                due_and_held(&database).await?,
                (vec![MANAGE_RELEASES.to_owned()], 0)
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn iss_08_migration_65_marks_nothing_due_on_a_new_server() -> TestResult {
        each_database("migration_65_new_server", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(65, at(0)).await?;
            assert_eq!(due_and_held(&database).await?, (Vec::new(), 0));
            Ok(())
        })
        .await
    }
}
