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
mod m0024_saved_searches;
mod m0025_text_in_nfc;

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
    Definition {
        version: 24,
        name: "saved_searches",
        steps: || Ok(m0024_saved_searches::steps()),
    },
    Definition {
        version: bb_core::text_index::RULE_MIGRATION,
        name: "text_in_nfc",
        steps: || Ok(m0025_text_in_nfc::steps()),
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

/// How the schema of a database stands to the program that opened it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaState {
    /// Every migration the program has is applied.
    Current,
    /// The program has migrations the database has not: a start of the server applies them.
    Older,
    /// A newer program changed the database: this one must not work on it.
    Newer,
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
    /// How the schema stands to this program, without changing anything: the one place that
    /// decides (commands that look after an installation and migrate nothing ask it).
    ///
    /// # Errors
    /// [`StoreError::MigrationMismatch`] when a recorded migration is not the one this program has,
    /// and when the database fails.
    pub async fn schema_state(&self) -> Result<SchemaState, StoreError> {
        let applied = self.applied_migrations().await?;
        match check(&applied) {
            Err(StoreError::SchemaTooNew { .. }) => Ok(SchemaState::Newer),
            Err(other) => Err(other),
            Ok(()) => {
                let newest = applied
                    .iter()
                    .map(|record| record.version)
                    .max()
                    .unwrap_or(0);
                Ok(if newest < latest_version() {
                    SchemaState::Older
                } else {
                    SchemaState::Current
                })
            }
        }
    }

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

    /// Records a migration this program does not have, as a newer program would have: the database
    /// then stands [`SchemaState::Newer`], for the tests of what refuses to work on it.
    ///
    /// # Errors
    /// When the database fails.
    #[cfg(any(test, feature = "testing"))]
    pub async fn record_newer_migration(&self, now: Timestamp) -> Result<(), StoreError> {
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
                Expr::val(latest_version() + 1),
                Expr::val("from-the-future"),
                Expr::val(vec![0_u8; 32]),
                Expr::val(to_micros(now)),
            ])
            .map_err(|_| StoreError::Corrupt("a migration"))?;
        self.execute(&insert).await?;
        Ok(())
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
mod tests;
