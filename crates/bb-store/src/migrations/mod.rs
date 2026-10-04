//! Migrations: numbered, forward-only Rust code that builds the schema with the query builder, so
//! one definition runs on SQLite and PostgreSQL (ADR-0003, DATA-MIGRATIONS).
//!
//! Each applied migration is recorded in the `migration` table. A database that has a migration
//! this program does not know was changed by a newer Bugaboo, and the program refuses to start on
//! it instead of guessing what changed.

use bb_core::migration::MigrationRecord;
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{
    ColumnDef, Expr, IndexCreateStatement, InsertStatement, Order, Query, Table,
    TableCreateStatement,
};

use crate::db::{Database, Row, Transaction, from_micros, to_micros};
use crate::error::StoreError;
use crate::schema::Migration;

mod m0001_initial;

/// One step of a migration.
pub(crate) enum Step {
    /// Create a table.
    Table(TableCreateStatement),
    /// Create an index.
    Index(IndexCreateStatement),
    /// Insert a fixed row the program relies on.
    Seed(InsertStatement),
}

/// One migration.
struct Definition {
    version: i64,
    name: &'static str,
    steps: fn() -> Result<Vec<Step>, sea_query::error::Error>,
}

/// Every migration, in order. A new one is added at the end and never changed afterwards.
const ALL: &[Definition] = &[Definition {
    version: 1,
    name: "initial",
    steps: m0001_initial::steps,
}];

/// The number of the newest migration this program knows.
#[must_use]
pub fn latest_version() -> i64 {
    ALL.last().map_or(0, |migration| migration.version)
}

fn record_table() -> TableCreateStatement {
    Table::create()
        .table(Migration::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(Migration::Version)
                .big_integer()
                .not_null()
                .primary_key(),
        )
        .col(ColumnDef::new(Migration::Name).text().not_null())
        .col(
            ColumnDef::new(Migration::AppliedAt)
                .big_integer()
                .not_null(),
        )
        .to_owned()
}

impl Database {
    /// Brings the schema up to date (`now` is what gets recorded) and returns the numbers of the migrations it applied.
    ///
    /// # Errors
    /// Fails with [`StoreError::SchemaTooNew`] when the database was changed by a newer program,
    /// with [`StoreError::MigrationMismatch`] when the recorded migrations are not ours, and when
    /// the database fails.
    pub async fn migrate(&self, now: Timestamp) -> Result<Vec<i64>, StoreError> {
        let mut transaction = self.begin().await?;
        transaction.execute_schema(&record_table()).await?;
        transaction.commit().await?;

        let applied = self.applied_migrations().await?;
        let known = latest_version();
        if let Some(newest) = applied.iter().map(|record| record.version).max()
            && newest > known
        {
            return Err(StoreError::SchemaTooNew {
                found: newest,
                known,
            });
        }
        for (index, record) in applied.iter().enumerate() {
            let matches = ALL.get(index).is_some_and(|migration| {
                migration.version == record.version && migration.name == record.name.as_str()
            });
            if !matches {
                return Err(StoreError::MigrationMismatch {
                    version: record.version,
                });
            }
        }

        let mut done = Vec::new();
        for migration in ALL.iter().skip(applied.len()) {
            let mut transaction = self.begin().await?;
            for step in (migration.steps)().map_err(|_| StoreError::Corrupt("a migration"))? {
                match step {
                    Step::Table(statement) => transaction.execute_schema(&statement).await?,
                    Step::Index(statement) => transaction.execute_schema(&statement).await?,
                    Step::Seed(statement) => {
                        transaction.execute(&statement).await?;
                    }
                }
            }
            record(&mut transaction, migration, now).await?;
            transaction.commit().await?;
            done.push(migration.version);
        }
        Ok(done)
    }

    /// The migrations the database has recorded, oldest first.
    ///
    /// # Errors
    /// Fails when the database fails or a record is unreadable.
    pub async fn applied_migrations(&self) -> Result<Vec<MigrationRecord>, StoreError> {
        let mut select = Query::select();
        select
            .columns([Migration::Version, Migration::Name, Migration::AppliedAt])
            .from(Migration::Table)
            .order_by(Migration::Version, Order::Asc);
        self.fetch_all(&select)
            .await?
            .iter()
            .map(read_record)
            .collect()
    }
}

async fn record(
    transaction: &mut Transaction,
    migration: &Definition,
    now: Timestamp,
) -> Result<(), StoreError> {
    let mut insert = Query::insert();
    insert
        .into_table(Migration::Table)
        .columns([Migration::Version, Migration::Name, Migration::AppliedAt])
        .values([
            Expr::val(migration.version),
            Expr::val(migration.name),
            Expr::val(to_micros(now)),
        ])
        .map_err(|_| StoreError::Corrupt("a migration record"))?;
    transaction.execute(&insert).await?;
    Ok(())
}

fn read_record(row: &Row) -> Result<MigrationRecord, StoreError> {
    Ok(MigrationRecord {
        version: row.get(Migration::Version)?,
        name: TechnicalName::new(&row.get::<String>(Migration::Name)?)
            .map_err(|_| StoreError::Corrupt("a migration name"))?,
        applied_at: from_micros(row.get(Migration::AppliedAt)?)?,
    })
}

#[cfg(test)]
mod tests {
    use bb_testing::{TestResult, each_database};
    use sea_query::{Alias, Query};

    use super::*;
    use crate::test_support::at;

    #[tokio::test]
    async fn migrations() -> TestResult {
        each_database("migrations", async |db| {
            let database = Database::connect(db.url()).await?;
            assert_eq!(database.migrate(at(0)).await?, [1]);
            assert_eq!(database.migrate(at(1)).await?, Vec::<i64>::new());
            let applied = database.applied_migrations().await?;
            assert_eq!(applied.len(), 1);
            assert_eq!(applied[0].name.as_str(), "initial");
            assert_eq!(applied[0].applied_at, at(0));
            // Every table of the schema exists and belongs to an entity.
            for (name, marks) in crate::schema::tables() {
                let mut select = Query::select();
                select
                    .expr(Expr::col(sea_query::Asterisk))
                    .from(Alias::new(&name))
                    .limit(1);
                database.fetch_all(&select).await?;
                assert!(!marks.fields().is_empty());
            }
            database.close().await;
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn bad_example_a_newer_schema_is_refused() -> TestResult {
        each_database("bad_example_a_newer_schema_is_refused", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate(at(0)).await?;
            let mut insert = Query::insert();
            insert
                .into_table(Migration::Table)
                .columns([Migration::Version, Migration::Name, Migration::AppliedAt])
                .values([
                    Expr::val(99_i64),
                    Expr::val("from-the-future"),
                    Expr::val(0_i64),
                ])?;
            database.execute(&insert).await?;
            let error = database.migrate(at(1)).await.unwrap_err();
            assert!(
                matches!(
                    error,
                    StoreError::SchemaTooNew {
                        found: 99,
                        known: 1
                    }
                ),
                "{error}"
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn bad_example_a_renamed_migration_is_refused() -> TestResult {
        each_database("bad_example_a_renamed_migration_is_refused", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate(at(0)).await?;
            let mut update = Query::update();
            update
                .table(Migration::Table)
                .value(Migration::Name, "other");
            database.execute(&update).await?;
            let error = database.migrate(at(1)).await.unwrap_err();
            assert!(
                matches!(error, StoreError::MigrationMismatch { version: 1 }),
                "{error}"
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn sqlite_runs_in_wal_mode_with_foreign_keys() -> TestResult {
        each_database("sqlite_runs_in_wal_mode_with_foreign_keys", async |db| {
            if db.sqlite_pool().is_some() {
                let database = Database::connect(db.url()).await?;
                assert_eq!(database.dialect(), crate::Dialect::Sqlite);
                assert_eq!(database.sqlite_pragmas().await?, ("wal".to_owned(), 1));
            }
            Ok(())
        })
        .await
    }
}
