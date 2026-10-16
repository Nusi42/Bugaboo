//! Migration 14: the names of things are the words of the glossary (M4 WP-02, ADR-0008 amendment).
//!
//! Before the API stops changing, the stored values that carried an old word are written with the
//! new one, the way migration 13 did for a kind of job:
//!
//! - the permissions to create (`make-issues`, `make-projects`, `make-sub-projects`) in the rows of
//!   roles and of access keys;
//! - the kind of record of a run of a change to many issues in the events (`bulk_run` is
//!   `change_run`), which the outbox points to, so it needs no rewrite of its own;
//! - the kind of job that goes on with such a run (`bulk.step` is `many-changes.step`).
//!
//! Every step is an update of a value that names something, so running the steps again changes
//! nothing.

use sea_query::{Expr, ExprTrait as _, Query};

use super::Step;
use crate::schema::{AccessKeyPermissionRow, Event, Job, RolePermissionRow};

/// The permissions whose word changed, old and new.
const PERMISSIONS: [(&str, &str); 3] = [
    ("make-issues", "create-issues"),
    ("make-projects", "create-projects"),
    ("make-sub-projects", "create-sub-projects"),
];

pub(super) fn steps() -> Vec<Step> {
    let mut steps = permission_words();
    steps.push(rename_value(
        Event::Table,
        Event::EntityKind,
        "bulk_run",
        "change_run",
    ));
    steps.push(rename_value(
        Job::Table,
        Job::Kind,
        "bulk.step",
        "many-changes.step",
    ));
    steps
}

/// Writes `new` where `column` of `table` holds `old`.
fn rename_value(
    table: impl sea_query::Iden + 'static,
    column: impl sea_query::Iden + Copy + 'static,
    old: &str,
    new: &str,
) -> Step {
    Step::Update(
        Query::update()
            .table(table)
            .value(column, new)
            .and_where(Expr::col(column).eq(old))
            .to_owned(),
    )
}

/// The permission words in the rows of roles and of access keys.
fn permission_words() -> Vec<Step> {
    let mut steps = Vec::new();
    for (old, new) in PERMISSIONS {
        steps.push(rename_value(
            RolePermissionRow::Table,
            RolePermissionRow::Permission,
            old,
            new,
        ));
        steps.push(rename_value(
            AccessKeyPermissionRow::Table,
            AccessKeyPermissionRow::Permission,
            old,
            new,
        ));
    }
    steps
}

#[cfg(test)]
mod tests {
    use bb_core::event::EntityKind;
    use bb_core::id::JobId;
    use bb_core::technical::TechnicalName;
    use bb_testing::{Failure, TestDatabase, TestResult, each_database};
    use sea_query::{Expr, Order, Query};
    use serde_json::Value;

    use super::*;
    use crate::db::Database;
    use crate::jobs::NewJob;
    use crate::schema::{AccessKeyRow, PersonRow, RoleRow};
    use crate::test_support::{at, event, ids};

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
    async fn adr_0008_an_event_of_a_run_of_a_change_to_many_issues_has_the_new_kind() -> TestResult
    {
        each_database("m0014_change_run_kind", async |db| {
            let database = m3_database(db).await?;
            let mut source = ids();
            for (seconds, kind) in [(1, "bulk_run"), (2, "issue")] {
                let mut event = event(&mut source, seconds);
                event.entity.kind = EntityKind::new(kind)?;
                database
                    .record_change(&event, async |_transaction| Ok(()))
                    .await?;
            }
            assert_eq!(database.migrate(at(1)).await?, [14]);
            for _ in 0..2 {
                let mut select = Query::select();
                select
                    .column(Event::EntityKind)
                    .from(Event::Table)
                    .order_by(Event::EntityKind, Order::Asc);
                let kinds: Vec<String> = database
                    .fetch_all(&select)
                    .await?
                    .iter()
                    .map(|row| row.get::<String>(Event::EntityKind))
                    .collect::<Result<_, _>>()?;
                assert_eq!(kinds, ["change_run", "issue"]);
                run_updates_again(&database).await?;
            }
            assert!(database.migrate(at(2)).await?.is_empty());
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn adr_0017_a_job_that_goes_on_with_a_run_waits_under_the_new_kind() -> TestResult {
        each_database("m0014_many_changes_job_kind", async |db| {
            let database = m3_database(db).await?;
            let mut source = ids();
            let job = NewJob {
                kind: TechnicalName::new("bulk.step")?,
                payload: Value::Null,
                run_at: at(0),
                max_attempts: 5,
            };
            database
                .enqueue_job(JobId::generate(&mut source)?, &job, at(0))
                .await?;
            assert_eq!(database.migrate(at(1)).await?, [14]);
            run_updates_again(&database).await?;
            let worker = TechnicalName::new("worker.one")?;
            let taken = database
                .lease_jobs(&worker, at(2), jiff::SignedDuration::from_secs(60), 5)
                .await?;
            assert_eq!(taken.len(), 1);
            assert_eq!(taken[0].kind.as_str(), "many-changes.step");
            assert!(database.migrate(at(3)).await?.is_empty());
            Ok(())
        })
        .await
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
