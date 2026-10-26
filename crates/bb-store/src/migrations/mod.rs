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
//!
//! Migrations that are given out to work packages built at the same time must not depend on each
//! other: a database can get them in either order (a database that has number 19 may get 18 later,
//! and the other way round), so each one builds only on the migrations before the numbers given out.

use bb_core::migration::MigrationRecord;
use bb_core::security_log::Digest;
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{
    ColumnDef, DeleteStatement, Expr, IndexCreateStatement, IndexDropStatement, InsertStatement,
    Order, PostgresQueryBuilder, Query, SqliteQueryBuilder, Table, TableAlterStatement,
    TableCreateStatement, TableRenameStatement, UpdateStatement,
};
use sha2::{Digest as _, Sha256};

use crate::db::{Database, FixedSql, Row, Transaction, from_micros, to_micros};
use crate::error::StoreError;
use crate::locks::LockClass;
use crate::schema::Migration;

mod m0001_initial;
mod m0002_sign_in;
mod m0003_policy;
mod m0004_settings;
mod m0005_projects;
mod m0006_access_keys;
mod m0007_issues;
mod m0008_editing;
mod m0009_fields;
mod m0010_relations;
mod m0011_tags;
mod m0012_layouts;
mod m0013_job_kinds;
mod m0014_names;
mod m0015_workflows;
mod m0016_files;
mod m0017_notifications;
mod m0018_offers;
mod m0019_layout_off_fields;
mod m0020_mention_read_at;
mod m0021_kebab_words;
mod m0022_text_index;
mod m0023_search_indexes;

/// A column that holds the ID of a record: 16 bytes, never empty.
pub(super) fn id_column(name: impl sea_query::Iden + 'static) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.blob().not_null();
    column
}

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
    /// Rewrite values the program names differently now.
    Update(UpdateStatement),
    /// Give a table the name the program calls it now.
    Rename(TableRenameStatement),
    /// Take an index away that a later rule no longer wants (a unique key that became a rule of
    /// the program).
    DropIndex(IndexDropStatement),
    /// Statements that only one database knows (a virtual table, a generated column).
    Sql(FixedSql),
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
                Step::Update(statement) => [
                    statement.to_string(SqliteQueryBuilder),
                    statement.to_string(PostgresQueryBuilder),
                ],
                Step::Rename(statement) => [
                    statement.build(SqliteQueryBuilder),
                    statement.build(PostgresQueryBuilder),
                ],
                Step::DropIndex(statement) => [
                    statement.build(SqliteQueryBuilder),
                    statement.build(PostgresQueryBuilder),
                ],
                Step::Sql(sql) => sql.texts(),
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
    Definition {
        version: 4,
        name: "settings",
        steps: m0004_settings::steps,
    },
    Definition {
        version: 5,
        name: "projects",
        steps: || Ok(m0005_projects::steps()),
    },
    Definition {
        version: 6,
        name: "access_keys",
        steps: || Ok(m0006_access_keys::steps()),
    },
    Definition {
        version: 7,
        name: "issues",
        steps: || Ok(m0007_issues::steps()),
    },
    Definition {
        version: 8,
        name: "editing",
        steps: || Ok(m0008_editing::steps()),
    },
    Definition {
        version: 9,
        name: "fields",
        steps: || Ok(m0009_fields::steps()),
    },
    Definition {
        version: 10,
        name: "relations",
        steps: || Ok(m0010_relations::steps()),
    },
    Definition {
        version: 11,
        name: "tags",
        steps: || Ok(m0011_tags::steps()),
    },
    Definition {
        version: 12,
        name: "layouts",
        steps: || Ok(m0012_layouts::steps()),
    },
    Definition {
        version: 13,
        name: "job_kinds",
        steps: || Ok(m0013_job_kinds::steps()),
    },
    Definition {
        version: 14,
        name: "names",
        steps: || Ok(m0014_names::steps()),
    },
    Definition {
        version: 15,
        name: "workflows",
        steps: || Ok(m0015_workflows::steps()),
    },
    Definition {
        version: 16,
        name: "files",
        steps: || Ok(m0016_files::steps()),
    },
    Definition {
        version: 17,
        name: "notifications",
        steps: || Ok(m0017_notifications::steps()),
    },
    Definition {
        version: 18,
        name: "offers",
        steps: || Ok(m0018_offers::steps()),
    },
    Definition {
        version: 19,
        name: "layout_off_fields",
        steps: || Ok(m0019_layout_off_fields::steps()),
    },
    Definition {
        version: 20,
        name: "mention_read_at",
        steps: || Ok(m0020_mention_read_at::steps()),
    },
    Definition {
        version: 21,
        name: "kebab_words",
        steps: || Ok(m0021_kebab_words::steps()),
    },
    Definition {
        version: 22,
        name: "text_index",
        steps: || Ok(m0022_text_index::steps()),
    },
    Definition {
        version: 23,
        name: "search_indexes",
        steps: || Ok(m0023_search_indexes::steps()),
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
            transaction.advisory_lock(LockClass::Migration, 0).await?;
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
                    Step::Update(statement) => {
                        transaction.execute(&statement).await?;
                    }
                    Step::Rename(statement) => transaction.execute_schema(&statement).await?,
                    Step::DropIndex(statement) => transaction.execute_schema(&statement).await?,
                    Step::Sql(sql) => transaction.execute_fixed(&sql).await?,
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

    /// By the number of the migration, not by place in the list: the numbers of other migrations that
    /// are added later fall between these.
    const APPLIED: [(i64, &str); 22] = [
        (
            1,
            "955f17699da59d0dbebd0aa84f15d41babff5bf3493ba3e72e422dfaf8d53de5",
        ),
        (
            2,
            "2f79f1fcf6ccadf282c92d37e1803b440c1759d098887112f96b5cbc24d2f30e",
        ),
        (
            3,
            "559268007329af29e9b173a8e836d1e5e5d924a70eb5a509147de6d2f1a54d2a",
        ),
        (
            4,
            "da7d77fae7b54c478965753e08448db6e9ff6bb7cc1910aa5acefeff4d15c990",
        ),
        (
            5,
            "197f6e67ca2b33794b484a19c5b8577eb97023cd759f34689ddd8fe3131fb9cd",
        ),
        (
            6,
            "888c571ed03837927111de0bc757b71c8608e08dff06c11631d94c6a292e8ed4",
        ),
        (
            7,
            "8fbd8b15ec9aed2d418484ea4ee883801c5aacc7e2cbc20e4c967852e6db8ab0",
        ),
        (
            8,
            "a994400ac493c6282be22dca5f4d6dd33dcd9e9556a80c648ac1322df0ff590e",
        ),
        (
            9,
            "c5dd6be187a9c42674b2bdfd05cf9d28eb88486bea90f96ec4d12b2a4f2fff07",
        ),
        (
            10,
            "d1579888290157b29550deda66ad28230d214174efffc6c0345a7d94edfdc10c",
        ),
        (
            11,
            "dcaa647639437b3e8751fee41009ecde93b5872c8064d39f2600bd992f7b559c",
        ),
        (
            12,
            "fffa0b40e2a8b74a36236e5d54896ec8d4a5078c82dcde3215f9ff610a4b6d2e",
        ),
        (
            13,
            "672ecd2ebb383bd7483263729b11f4f2f3e2695825b42b86bbaf22330b12a4bf",
        ),
        (
            14,
            "045061588034b6040764c6ba14cfd34631f165361100372b98f44a2812c6b790",
        ),
        (
            15,
            "16ed6f2372e8700c9a3158a53edbeb5e976ecf9737498802ee081c241ed286b2",
        ),
        (
            16,
            "b479b723a711237578a12f2cedcbce795c386d54a1a11430ffbb6ebb6193c9b7",
        ),
        (
            17,
            "d21f77afec1f2020ece56ef48d087d4f90e572a090a160a88488dac81164822a",
        ),
        (
            18,
            "c9688e8ee165a9f05d678b5d580e2ba23b46d619392a8342f9265fc31dfdefa7",
        ),
        (
            19,
            "f4b6ea4cb7a8bcd2247daefcc92413b74a89cff1eb45e5de62a697851f5eaedd",
        ),
        (
            20,
            "e1510d877cd996f9285d03a87bd8024feac32701a89fb0a9a8340785ac674ee6",
        ),
        (
            21,
            "d80ffbf5d3a0b8999ffadcf0dbf1a9fe63bd1d1628f7973ce02aabc7368f816d",
        ),
        (
            22,
            "b17ab3436cb24d81dcf609ca5ece9ade808ab32f0b8cbd03d00c3e4a2f650562",
        ),
    ];

    /// The checksum of every migration a database may have applied. If one fails after a rename in
    /// the code, a migration now builds other statements than the ones databases applied before:
    /// databases that applied it would refuse to start. Add a new migration instead of changing an
    /// old one, and name the old tables and columns it uses in the migration itself.
    #[test]
    fn the_checksum_of_an_applied_migration_never_changes() {
        for (version, wanted) in APPLIED {
            let definition = ALL
                .iter()
                .find(|definition| definition.version == version)
                .unwrap();
            let checksum = definition.checksum().unwrap();
            assert_eq!(
                bb_core::hex::encode(checksum.as_bytes()),
                wanted,
                "the checksum of migration {version} changed",
            );
        }
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
            assert_eq!(applied[3].name.as_str(), "settings");
            assert_eq!(applied[4].name.as_str(), "projects");
            assert_eq!(applied[5].name.as_str(), "access_keys");
            assert_eq!(applied[6].name.as_str(), "issues");
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

    #[tokio::test]
    async fn adr_0019_groups_and_roles_that_were_there_start_at_the_first_version() -> TestResult {
        each_database("migration_8_first_version", async |db| {
            use crate::schema::{GroupRow, RoleRow};
            use bb_core::version::FIRST_VERSION;
            let database = Database::connect(db.url()).await?;
            database.migrate_until(6, at(0)).await?;
            // A group and a role as the database held them before they had a version.
            let mut group = Query::insert();
            group
                .into_table(GroupRow::Table)
                .columns([GroupRow::Id, GroupRow::Name])
                .values([Expr::val(vec![1_u8; 16]), Expr::val("Old group")])?;
            database.execute(&group).await?;
            let mut role = Query::insert();
            role.into_table(RoleRow::Table)
                .columns([
                    RoleRow::Id,
                    RoleRow::Name,
                    RoleRow::Position,
                    RoleRow::TwoStep,
                ])
                .values([
                    Expr::val(vec![2_u8; 16]),
                    Expr::val("Old role"),
                    Expr::val(0_i64),
                    Expr::val(false),
                ])?;
            database.execute(&role).await?;
            database.migrate(at(1)).await?;
            let mut groups = Query::select();
            groups.column(GroupRow::Version).from(GroupRow::Table);
            let rows = database.fetch_all(&groups).await?;
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].get::<i64>(GroupRow::Version)?, FIRST_VERSION);
            let mut roles = Query::select();
            roles.column(RoleRow::Version).from(RoleRow::Table);
            let rows = database.fetch_all(&roles).await?;
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].get::<i64>(RoleRow::Version)?, FIRST_VERSION);
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
                .and_where(Expr::col(Migration::Version).eq(4_i64));
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
