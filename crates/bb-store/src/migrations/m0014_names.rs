//! Migration 14: the names of things are the words of the glossary (M4 WP-02, ADR-0008 amendment).
//!
//! Before the API stops changing, the stored values that carried an old word are written with the
//! new one, the way migration 13 did for a kind of job:
//!
//! - the permissions to create (`make-issues`, `make-projects`, `make-sub-projects`) in the rows of
//!   roles and of access keys.
//!
//! Every step is an update of a value that names something, so running the steps again changes
//! nothing.

use sea_query::{Expr, ExprTrait as _, Query};

use super::Step;
use crate::schema::{AccessKeyPermissionRow, RolePermissionRow};

/// The permissions whose word changed, old and new.
const PERMISSIONS: [(&str, &str); 3] = [
    ("make-issues", "create-issues"),
    ("make-projects", "create-projects"),
    ("make-sub-projects", "create-sub-projects"),
];

pub(super) fn steps() -> Vec<Step> {
    permission_words()
}

/// The permission words in the rows of roles and of access keys.
fn permission_words() -> Vec<Step> {
    let mut steps = Vec::new();
    for (old, new) in PERMISSIONS {
        steps.push(Step::Update(
            Query::update()
                .table(RolePermissionRow::Table)
                .value(RolePermissionRow::Permission, new)
                .and_where(Expr::col(RolePermissionRow::Permission).eq(old))
                .to_owned(),
        ));
        steps.push(Step::Update(
            Query::update()
                .table(AccessKeyPermissionRow::Table)
                .value(AccessKeyPermissionRow::Permission, new)
                .and_where(Expr::col(AccessKeyPermissionRow::Permission).eq(old))
                .to_owned(),
        ));
    }
    steps
}

#[cfg(test)]
mod tests {
    use bb_testing::{Failure, TestDatabase, TestResult, each_database};
    use sea_query::{Expr, Order, Query};

    use super::*;
    use crate::db::Database;
    use crate::schema::{AccessKeyRow, PersonRow, RoleRow};
    use crate::test_support::at;

    /// The database as M3 left it: migrations 1 to 13 applied, nothing newer.
    async fn m3_database(db: &TestDatabase) -> Result<Database, Failure> {
        let database = Database::connect(db.url()).await?;
        database.migrate_until(13, at(0)).await?;
        Ok(database)
    }

    /// Runs the update steps of the migration again.
    async fn run_updates_again(database: &Database) -> Result<(), Failure> {
        for step in steps() {
            if let Step::Update(update) = step {
                database.execute(&update).await?;
            }
        }
        Ok(())
    }

    async fn permissions_of(
        database: &Database,
        table: impl sea_query::Iden + Copy + 'static,
        column: impl sea_query::Iden + Copy + 'static,
    ) -> Result<Vec<String>, Failure> {
        let mut select = Query::select();
        select
            .column(column)
            .from(table)
            .order_by(column, Order::Asc);
        let rows = database.fetch_all(&select).await?;
        rows.iter()
            .map(|row| row.get::<String>(column).map_err(Into::into))
            .collect()
    }

    #[tokio::test]
    async fn adr_0008_the_permissions_to_create_are_written_with_the_new_words() -> TestResult {
        each_database("m0014_permission_words", async |db| {
            let database = m3_database(db).await?;
            // A role and an access key with the words M3 wrote.
            let mut role = Query::insert();
            role.into_table(RoleRow::Table)
                .columns([
                    RoleRow::Id,
                    RoleRow::Name,
                    RoleRow::Position,
                    RoleRow::TwoStep,
                    RoleRow::Version,
                ])
                .values([
                    Expr::val(vec![1_u8; 16]),
                    Expr::val("Old role"),
                    Expr::val(0_i64),
                    Expr::val(false),
                    Expr::val(1_i64),
                ])?;
            database.execute(&role).await?;
            let mut person = Query::insert();
            person
                .into_table(PersonRow::Table)
                .columns([
                    PersonRow::Id,
                    PersonRow::DisplayName,
                    PersonRow::Email,
                    PersonRow::CreatedAt,
                ])
                .values([
                    Expr::val(vec![2_u8; 16]),
                    Expr::val("Ada"),
                    Expr::val("ada@example.test"),
                    Expr::val(0_i64),
                ])?;
            database.execute(&person).await?;
            let mut key = Query::insert();
            key.into_table(AccessKeyRow::Table)
                .columns([
                    AccessKeyRow::Id,
                    AccessKeyRow::Owner,
                    AccessKeyRow::Name,
                    AccessKeyRow::Prefix,
                    AccessKeyRow::SecretHash,
                    AccessKeyRow::Choice,
                    AccessKeyRow::CreatedAt,
                    AccessKeyRow::ExpiresAt,
                ])
                .values([
                    Expr::val(vec![3_u8; 16]),
                    Expr::val(vec![2_u8; 16]),
                    Expr::val("Key"),
                    Expr::val("00000001"),
                    Expr::val(vec![4_u8; 32]),
                    Expr::val("permissions"),
                    Expr::val(0_i64),
                    Expr::val(1000_i64),
                ])?;
            database.execute(&key).await?;
            for old in [
                "make-issues",
                "make-projects",
                "make-sub-projects",
                "see-issues",
            ] {
                let mut role_row = Query::insert();
                role_row
                    .into_table(RolePermissionRow::Table)
                    .columns([RolePermissionRow::Role, RolePermissionRow::Permission])
                    .values([Expr::val(vec![1_u8; 16]), Expr::val(old)])?;
                database.execute(&role_row).await?;
                let mut key_row = Query::insert();
                key_row
                    .into_table(AccessKeyPermissionRow::Table)
                    .columns([
                        AccessKeyPermissionRow::Key,
                        AccessKeyPermissionRow::Permission,
                    ])
                    .values([Expr::val(vec![3_u8; 16]), Expr::val(old)])?;
                database.execute(&key_row).await?;
            }
            assert_eq!(database.migrate(at(1)).await?, [14]);
            let wanted = [
                "create-issues",
                "create-projects",
                "create-sub-projects",
                "see-issues",
            ];
            for _ in 0..2 {
                assert_eq!(
                    permissions_of(
                        &database,
                        RolePermissionRow::Table,
                        RolePermissionRow::Permission
                    )
                    .await?,
                    wanted
                );
                assert_eq!(
                    permissions_of(
                        &database,
                        AccessKeyPermissionRow::Table,
                        AccessKeyPermissionRow::Permission
                    )
                    .await?,
                    wanted
                );
                run_updates_again(&database).await?;
            }
            // Migrating again applies nothing.
            assert!(database.migrate(at(2)).await?.is_empty());
            Ok(())
        })
        .await
    }
}
