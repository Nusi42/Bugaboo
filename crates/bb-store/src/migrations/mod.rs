//! Migrations: numbered, forward-only Rust code that builds the schema with the query builder, so
//! one definition runs on SQLite and PostgreSQL (ADR-0003, DATA-MIGRATIONS).
//!
//! Each applied migration is recorded in the `migration` table with its name and a checksum of
//! its steps. The program refuses to start when the database has a migration it does not know (it
//! was changed by a newer Bugaboo), or one whose name or checksum differs from the migration of the
//! same number in this program (somebody changed an applied migration). Applying runs inside a
//! transaction that also takes the write lock (SQLite) or an advisory lock (PostgreSQL) and reads
//! the record of applied migrations again inside it, so two processes that start at the same
//! moment cannot both apply the same migration.

use bb_core::migration::MigrationRecord;
use bb_core::security_log::Digest;
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{
    ColumnDef, DeleteStatement, Expr, IndexCreateStatement, InsertStatement, Order,
    PostgresQueryBuilder, Query, SqliteQueryBuilder, Table, TableAlterStatement,
    TableCreateStatement,
};
use sha2::{Digest as _, Sha256};

use crate::db::{Database, Row, Transaction, from_micros, to_micros};
use crate::error::StoreError;
use crate::schema::Migration;

mod m0001_initial;
mod m0002_sign_in;
mod m0003_policy;
mod m0005_projects;

/// The advisory lock PostgreSQL processes take while they migrate.
const MIGRATION_LOCK: i64 = 0x4242_4D49_4752_0001;

/// One step of a migration.
pub(crate) enum Step {
    /// Create a table.
    Table(TableCreateStatement),
    /// Create an index.
    Index(IndexCreateStatement),
    /// Insert rows the program relies on, or rows copied from another table.
    Seed(InsertStatement),
    /// Change a table (take a column away).
    Alter(TableAlterStatement),
    /// Delete rows that no longer have a meaning.
    Delete(DeleteStatement),
}

/// One migration.
struct Definition {
    version: i64,
    name: &'static str,
    steps: fn() -> Result<Vec<Step>, sea_query::error::Error>,
}

impl Definition {
    fn steps(&self) -> Result<Vec<Step>, StoreError> {
        (self.steps)().map_err(|_| StoreError::Corrupt("a migration"))
    }

    /// A hash of the number, the name and the statements of every step, as both databases
    /// get them.
    fn checksum(&self) -> Result<Digest, StoreError> {
        let mut hasher = Sha256::new();
        hasher.update(self.version.to_be_bytes());
        hasher.update(self.name.as_bytes());
        for step in self.steps()? {
            let texts = match &step {
                Step::Table(statement) => [
                    statement.build(SqliteQueryBuilder),
                    statement.build(PostgresQueryBuilder),
                ],
                Step::Index(statement) => [
                    statement.build(SqliteQueryBuilder),
                    statement.build(PostgresQueryBuilder),
                ],
                Step::Seed(statement) => [
                    statement.to_string(SqliteQueryBuilder),
                    statement.to_string(PostgresQueryBuilder),
                ],
                Step::Alter(statement) => [
                    statement.build(SqliteQueryBuilder),
                    statement.build(PostgresQueryBuilder),
                ],
                Step::Delete(statement) => [
                    statement.to_string(SqliteQueryBuilder),
                    statement.to_string(PostgresQueryBuilder),
                ],
            };
            for text in texts {
                hasher.update(u64::try_from(text.len()).unwrap_or(u64::MAX).to_be_bytes());
                hasher.update(text.as_bytes());
            }
        }
        Ok(Digest::new(hasher.finalize().into()))
    }
}

/// Every migration, in order. A new one is added at the end and never changed afterwards.
const ALL: &[Definition] = &[
    Definition {
        version: 1,
        name: "initial",
        steps: m0001_initial::steps,
    },
    Definition {
        version: 2,
        name: "sign_in",
        steps: || Ok(m0002_sign_in::steps()),
    },
    Definition {
        version: 3,
        name: "policy",
        steps: m0003_policy::steps,
    },
    // 4 is the number of the settings in layers (WP-07).
    Definition {
        version: 5,
        name: "projects",
        steps: || Ok(m0005_projects::steps()),
    },
];

/// The number of the newest migration this program knows.
#[must_use]
pub fn latest_version() -> i64 {
    ALL.iter()
        .map(|definition| definition.version)
        .max()
        .unwrap_or(0)
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
        .col(ColumnDef::new(Migration::Checksum).blob().not_null())
        .col(
            ColumnDef::new(Migration::AppliedAt)
                .big_integer()
                .not_null(),
        )
        .to_owned()
}

/// Whether what the database recorded is what this program has.
fn check(applied: &[MigrationRecord]) -> Result<(), StoreError> {
    let known = latest_version();
    if let Some(newest) = applied.iter().map(|record| record.version).max()
        && newest > known
    {
        return Err(StoreError::SchemaTooNew {
            found: newest,
            known,
        });
    }
    for record in applied {
        let definition = ALL
            .iter()
            .find(|definition| definition.version == record.version)
            .ok_or(StoreError::MigrationMismatch {
                version: record.version,
            })?;
        if definition.name != record.name.as_str() || definition.checksum()? != record.checksum {
            return Err(StoreError::MigrationMismatch {
                version: record.version,
            });
        }
    }
    Ok(())
}

impl Database {
    /// Brings the schema up to date (`now` is what gets recorded) and returns the numbers of the
    /// migrations it applied.
    ///
    /// # Errors
    /// Fails with [`StoreError::SchemaTooNew`] when the database was changed by a newer program,
    /// with [`StoreError::MigrationMismatch`] when a recorded migration is not the one this
    /// program has, and when the database fails.
    pub async fn migrate(&self, now: Timestamp) -> Result<Vec<i64>, StoreError> {
        self.migrate_until(i64::MAX, now).await
    }

    /// Brings the schema up to migration `last` and no further: the database of an older
    /// version, for the tests of an upgrade.
    ///
    /// # Errors
    /// The same as [`Database::migrate`].
    #[cfg(feature = "testing")]
    pub async fn migrate_up_to(&self, last: i64, now: Timestamp) -> Result<Vec<i64>, StoreError> {
        self.migrate_until(last, now).await
    }

    async fn migrate_until(&self, last: i64, now: Timestamp) -> Result<Vec<i64>, StoreError> {
        let mut done = Vec::new();
        loop {
            let mut transaction = self.begin().await?;
            transaction.advisory_lock(MIGRATION_LOCK).await?;
            transaction.execute_schema(&record_table()).await?;
            let applied = read_applied(&mut transaction).await?;
            check(&applied)?;
            let Some(next) = ALL
                .iter()
                .filter(|definition| {
                    definition.version <= last
                        && !applied
                            .iter()
                            .any(|record| record.version == definition.version)
                })
                .min_by_key(|definition| definition.version)
            else {
                transaction.commit().await?;
                return Ok(done);
            };
            for step in next.steps()? {
                match step {
                    Step::Table(statement) => transaction.execute_schema(&statement).await?,
                    Step::Index(statement) => transaction.execute_schema(&statement).await?,
                    Step::Alter(statement) => transaction.execute_schema(&statement).await?,
                    Step::Seed(statement) => {
                        transaction.execute(&statement).await?;
                    }
                    Step::Delete(statement) => {
                        transaction.execute(&statement).await?;
                    }
                }
            }
            record(&mut transaction, next, now).await?;
            transaction.commit().await?;
            done.push(next.version);
        }
    }

    /// The migrations the database has recorded, oldest first.
    ///
    /// # Errors
    /// Fails when the database fails or a record is unreadable.
    pub async fn applied_migrations(&self) -> Result<Vec<MigrationRecord>, StoreError> {
        self.fetch_all(&applied_query())
            .await?
            .iter()
            .map(read_record)
            .collect()
    }
}

fn applied_query() -> sea_query::SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            Migration::Version,
            Migration::Name,
            Migration::Checksum,
            Migration::AppliedAt,
        ])
        .from(Migration::Table)
        .order_by(Migration::Version, Order::Asc);
    select
}

async fn read_applied(transaction: &mut Transaction) -> Result<Vec<MigrationRecord>, StoreError> {
    transaction
        .fetch_all(&applied_query())
        .await?
        .iter()
        .map(read_record)
        .collect()
}

async fn record(
    transaction: &mut Transaction,
    definition: &Definition,
    now: Timestamp,
) -> Result<(), StoreError> {
    let mut insert = Query::insert();
    insert
        .into_table(Migration::Table)
        .columns([
            Migration::Version,
            Migration::Name,
            Migration::Checksum,
            Migration::AppliedAt,
        ])
        .values([
            Expr::val(definition.version),
            Expr::val(definition.name),
            Expr::val(definition.checksum()?.as_bytes().to_vec()),
            Expr::val(to_micros(now)),
        ])
        .map_err(|_| StoreError::Corrupt("a migration record"))?;
    transaction.execute(&insert).await?;
    Ok(())
}

fn read_record(row: &Row) -> Result<MigrationRecord, StoreError> {
    let checksum: Vec<u8> = row.get(Migration::Checksum)?;
    Ok(MigrationRecord {
        version: row.get(Migration::Version)?,
        name: TechnicalName::new(&row.get::<String>(Migration::Name)?)
            .map_err(|_| StoreError::Corrupt("a migration name"))?,
        checksum: Digest::from_slice(&checksum).ok_or(StoreError::Corrupt("a checksum"))?,
        applied_at: from_micros(row.get(Migration::AppliedAt)?)?,
    })
}

#[cfg(test)]
mod tests {
    use bb_testing::{TestResult, each_database};
    use sea_query::{Alias, ExprTrait as _, Query};

    use super::*;
    use crate::test_support::at;

    #[test]
    fn the_checksum_of_an_applied_migration_never_changes() {
        // If this fails after an upgrade of sea-query or an edit of migration 1, databases that
        // applied it would refuse to start. Add a new migration instead of changing an old one.
        let checksum = ALL[0].checksum().unwrap();
        let mut hex = String::new();
        for byte in checksum.as_bytes() {
            use std::fmt::Write as _;
            write!(hex, "{byte:02x}").unwrap();
        }
        assert_eq!(
            hex, "955f17699da59d0dbebd0aa84f15d41babff5bf3493ba3e72e422dfaf8d53de5",
            "the checksum of migration 1 changed"
        );
    }

    #[test]
    fn the_checksum_of_migration_2_never_changes() {
        // Migration 2 names the column that migration 3 takes away through `Retired`; its text
        // is the one databases applied before.
        let checksum = ALL[1].checksum().unwrap();
        let mut hex = String::new();
        for byte in checksum.as_bytes() {
            use std::fmt::Write as _;
            write!(hex, "{byte:02x}").unwrap();
        }
        assert_eq!(
            hex, "2f79f1fcf6ccadf282c92d37e1803b440c1759d098887112f96b5cbc24d2f30e",
            "the checksum of migration 2 changed"
        );
    }

    #[tokio::test]
    async fn migrations() -> TestResult {
        each_database("migrations", async |db| {
            let database = Database::connect(db.url()).await?;
            let versions: Vec<i64> = ALL.iter().map(|definition| definition.version).collect();
            assert_eq!(database.migrate(at(0)).await?, versions);
            assert_eq!(database.migrate(at(1)).await?, Vec::<i64>::new());
            let applied = database.applied_migrations().await?;
            assert_eq!(applied.len(), ALL.len());
            assert_eq!(applied[0].name.as_str(), "initial");
            assert_eq!(applied[1].name.as_str(), "sign_in");
            assert_eq!(applied[2].name.as_str(), "policy");
            for (record, definition) in applied.iter().zip(ALL) {
                assert_eq!(record.name.as_str(), definition.name);
                assert_eq!(record.applied_at, at(0));
                assert_eq!(record.checksum, definition.checksum()?);
            }
            // Every table of the schema exists and belongs to an entity.
            for table in crate::schema::tables() {
                let mut select = Query::select();
                select
                    .expr(Expr::col(sea_query::Asterisk))
                    .from(Alias::new(&table.name))
                    .limit(1);
                database.fetch_all(&select).await?;
                assert!(!table.marks.fields().is_empty());
            }
            database.close().await;
            Ok(())
        })
        .await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn two_processes_migrating_at_once_apply_each_migration_once() -> TestResult {
        each_database("migrations_race", async |db| {
            // Each process has its own pool. They connect one after the other (switching a new
            // SQLite file to WAL mode is not something two processes can do at once) and then
            // migrate at the same moment.
            let mut databases = Vec::new();
            for _ in 0..4 {
                databases.push(Database::connect(db.url()).await?);
            }
            let mut tasks = Vec::new();
            for (number, database) in databases.into_iter().enumerate() {
                let number = i64::try_from(number).unwrap_or(0);
                tasks.push(tokio::spawn(async move {
                    let done = database.migrate(at(number)).await?;
                    database.close().await;
                    Ok::<_, StoreError>(done)
                }));
            }
            let mut applied = 0;
            for task in tasks {
                applied += task.await??.len();
            }
            assert_eq!(
                applied,
                ALL.len(),
                "each migration was applied by exactly one process"
            );
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
                .columns([
                    Migration::Version,
                    Migration::Name,
                    Migration::Checksum,
                    Migration::AppliedAt,
                ])
                .values([
                    Expr::val(99_i64),
                    Expr::val("from-the-future"),
                    Expr::val(vec![0_u8; 32]),
                    Expr::val(0_i64),
                ])?;
            database.execute(&insert).await?;
            let error = database.migrate(at(1)).await.unwrap_err();
            assert!(
                matches!(
                    error,
                    StoreError::SchemaTooNew {
                        found: 99,
                        known
                    } if known == latest_version()
                ),
                "{error}"
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn bad_example_a_renamed_or_changed_migration_is_refused() -> TestResult {
        each_database("bad_example_a_renamed_migration_is_refused", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate(at(0)).await?;
            for (column, value) in [
                (Migration::Name, sea_query::Value::from("other")),
                (Migration::Checksum, sea_query::Value::from(vec![1_u8; 32])),
            ] {
                let mut update = Query::update();
                update
                    .table(Migration::Table)
                    .value(column, value)
                    .and_where(Expr::col(Migration::Version).eq(1_i64));
                database.execute(&update).await?;
                let error = database.migrate(at(1)).await.unwrap_err();
                assert!(
                    matches!(error, StoreError::MigrationMismatch { version: 1 }),
                    "{error}"
                );
                // Put the record right for the next round.
                let mut restore = Query::update();
                restore
                    .table(Migration::Table)
                    .value(Migration::Name, "initial")
                    .value(Migration::Checksum, ALL[0].checksum()?.as_bytes().to_vec())
                    .and_where(Expr::col(Migration::Version).eq(1_i64));
                database.execute(&restore).await?;
            }
            assert!(database.migrate(at(2)).await?.is_empty());
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn a_number_the_program_does_not_have_is_not_taken_for_granted() -> TestResult {
        each_database("migrations_unknown_number", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate(at(0)).await?;
            let mut update = Query::update();
            update
                .table(Migration::Table)
                .value(Migration::Version, 0_i64)
                .and_where(Expr::col(Migration::Version).eq(3_i64));
            database.execute(&update).await?;
            let error = database.migrate(at(1)).await.unwrap_err();
            assert!(
                matches!(error, StoreError::MigrationMismatch { version: 0 }),
                "{error}"
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn sqlite_runs_in_wal_mode_with_foreign_keys_and_full_sync() -> TestResult {
        each_database("sqlite_runs_in_wal_mode_with_foreign_keys", async |db| {
            if db.sqlite_pool().is_some() {
                let database = Database::connect(db.url()).await?;
                assert_eq!(database.dialect(), crate::Dialect::Sqlite);
                assert_eq!(database.sqlite_pragmas().await?, ("wal".to_owned(), 1, 2));
            }
            Ok(())
        })
        .await
    }
}
