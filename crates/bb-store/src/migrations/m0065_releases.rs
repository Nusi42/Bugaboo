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
//! Last, the starting roles *Administrator* and *Project lead* of an existing server get the new
//! permission *Manage releases* (ADR-0025, amendment 2026-11-24). The roles are found by the key
//! of their `starting` mark, which a rename keeps; the keys and the permission are written out here,
//! not read from the program, so the migration stays what it was.

use sea_query::{ColumnDef, Expr, ExprTrait as _, ForeignKey, Index, Query, Table};

use super::{Step, id_column};
use crate::schema::{IssueFoundInRow, IssueRow, ReleaseRow, RolePermissionRow, RoleRow};

/// The key of the permission *Manage releases*, as it is stored.
const MANAGE_RELEASES: &str = "manage-releases";

/// The keys of the starting roles that hold it at the start.
const HOLDERS: [&str; 2] = ["administrator", "project-lead"];

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
                .col(moment(ReleaseRow::Version))
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
    let grants = Query::insert()
        .into_table(RolePermissionRow::Table)
        .columns([RolePermissionRow::Role, RolePermissionRow::Permission])
        .select_from(
            Query::select()
                .column(RoleRow::Id)
                .expr(Expr::val(MANAGE_RELEASES))
                .from(RoleRow::Table)
                .and_where(Expr::col(RoleRow::Starting).is_in(HOLDERS))
                .to_owned(),
        )?
        .to_owned();
    let mut steps = release_table();
    steps.extend(issue_releases());
    steps.push(Step::Seed(grants));
    Ok(steps)
}

#[cfg(test)]
mod tests {
    use bb_core::id::RoleId;
    use bb_testing::{TestResult, each_database};
    use jiff::Timestamp;
    use sea_query::{Expr, ExprTrait as _, IntoIden as _, Order, Query};

    use super::super::old_rows::{insert, old_role};
    use super::super::{Database, latest_version};
    use super::MANAGE_RELEASES;
    use crate::db::StoredId as _;
    use crate::schema::{RolePermissionRow, RoleRow};
    use crate::test_support::at;

    #[tokio::test]
    async fn iss_08_migration_65_gives_manage_releases_to_the_starting_administrator_and_project_lead()
    -> TestResult {
        each_database("migration_65_releases", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(64, at(0)).await?;
            let roles: Vec<RoleId> = (1..=4_u8)
                .map(|byte| RoleId::from_parts(Timestamp::UNIX_EPOCH, [byte; 10]))
                .collect::<Result<_, _>>()?;
            // Three starting roles, one renamed, and a role somebody made.
            for (place, (role, starting)) in roles
                .iter()
                .zip([
                    Some("administrator"),
                    Some("project-lead"),
                    Some("team-member"),
                    None,
                ])
                .enumerate()
            {
                old_role(&database, *role, i64::try_from(place)?).await?;
                let mut mark = Query::update();
                mark.table(RoleRow::Table)
                    .value(RoleRow::Starting, starting)
                    .and_where(Expr::col(RoleRow::Id).eq(role.stored()));
                database.execute(&mark).await?;
            }
            insert(
                &database,
                RolePermissionRow::Table,
                [
                    RolePermissionRow::Role.into_iden(),
                    RolePermissionRow::Permission.into_iden(),
                ],
                [Expr::val(roles[0].stored()), Expr::val("manage-labels")],
            )
            .await?;
            let applied = database.migrate_until(65, at(1)).await?;
            assert_eq!(applied.last(), Some(&65));
            assert!(latest_version() >= 65);
            let mut read = Query::select();
            read.column(RolePermissionRow::Role)
                .from(RolePermissionRow::Table)
                .and_where(Expr::col(RolePermissionRow::Permission).eq(MANAGE_RELEASES))
                .order_by(RolePermissionRow::Role, Order::Asc);
            let holders: Vec<RoleId> = database
                .fetch_all(&read)
                .await?
                .iter()
                .map(|row| row.id::<RoleId>(RolePermissionRow::Role))
                .collect::<Result<_, _>>()?;
            assert_eq!(holders, roles[..2]);
            Ok(())
        })
        .await
    }
}
