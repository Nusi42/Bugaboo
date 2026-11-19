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
//! Whether migrations are due is decided by the set of migrations applied, never by the newest
//! one alone: a database can lack a migration older than its newest (see below). A start that
//! finds migrations due on a database that has some already makes a backup first and puts it back
//! when a migration fails (`upgrade.rs`, ADR-0046 decision 5).
//!
//! Migrations that are given out to work packages built at the same time must not depend on each
//! other: a database can get them in either order (a database that has number 19 may get 18 later,
//! and the other way round), so each one builds only on the migrations before the numbers given out.

use bb_core::migration::MigrationRecord;
use bb_core::security_log::Digest;
use jiff::Timestamp;
use sea_query::{
    ColumnDef, DeleteStatement, IndexCreateStatement, IndexDropStatement, InsertStatement,
    PostgresQueryBuilder, SqliteQueryBuilder, TableAlterStatement, TableCreateStatement,
    TableRenameStatement, UpdateStatement,
};
use sha2::{Digest as _, Sha256};

use crate::db::{Database, FixedSql, Transaction};
use crate::error::StoreError;
use crate::locks::LockClass;
use list::ALL;

mod list;
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
mod m0026_name_key;
mod m0027_person_setting_version;
mod m0028_person_setting_counter;
mod m0029_notification_by_time;
mod m0030_saved_search_spacing;
mod m0031_time_entries;
mod m0032_working_hours;
mod m0033_issue_estimates;
mod m0034_layout_time_fields;
mod m0035_choice_numbers;
mod m0036_issue_totals_index;
mod m0037_ai_agreement;
mod m0038_ai_tools;
mod m0039_agents;
mod m0040_runs;
mod m0041_assistant_references;
mod m0042_agent_work;
mod m0043_oauth;
mod m0044_shown_moments;
mod m0045_agent_pause;
mod m0046_answer_time;
mod m0047_work_project;
mod m0048_rules_hand_over;
mod m0049_dates_and_date_effect;
mod m0050_work_times;
mod m0051_own_themes;
mod m0052_time_factor_and_accepted_warnings;
mod m0053_issue_dates_index;
mod m0054_backup_runs;
mod m0055_data_exports;
mod m0056_keeping;
mod m0057_person_standing;
mod m0058_e_mail_changes;
#[cfg(test)]
mod old_rows;
mod record;
mod upgrade;

pub(crate) use record::read_applied;
use record::{record, record_table};

pub use upgrade::{Due, Upgrade, UpgradeError};

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
#[derive(Clone, Copy)]
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

/// Every migration this program knows, with its name and checksum: what the migrations of a backup
/// are held against before it is restored (ADR-0046, decision 4).
///
/// # Errors
/// [`StoreError::Corrupt`] when a migration cannot be built (never, by the tests).
pub fn known_migrations() -> Result<Vec<(i64, &'static str, Digest)>, StoreError> {
    ALL.iter()
        .map(|definition| Ok((definition.version, definition.name, definition.checksum()?)))
        .collect()
}

/// The number of the newest migration this program knows.
#[must_use]
pub fn latest_version() -> i64 {
    newest_of(ALL)
}

/// The number of the newest migration of `list`.
fn newest_of(list: &[Definition]) -> i64 {
    list.iter()
        .map(|definition| definition.version)
        .max()
        .unwrap_or(0)
}

/// How the schema of a database stands to the program that opened it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaState {
    /// Every migration the program has is applied.
    Current,
    /// The program has migrations the database has not (newer ones, or one older than its newest):
    /// a start of the server applies them.
    Older,
    /// A newer program changed the database: this one must not work on it.
    Newer,
}

/// Whether what the database recorded is what the program with the migrations `list` has.
fn check(list: &[Definition], applied: &[MigrationRecord]) -> Result<(), StoreError> {
    let known = newest_of(list);
    if let Some(newest) = applied.iter().map(|record| record.version).max()
        && newest > known
    {
        return Err(StoreError::SchemaTooNew {
            found: newest,
            known,
        });
    }
    for record in applied {
        let definition = list
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
        state_of(ALL, &self.applied_migrations().await?)
    }

    /// Brings the schema up to date (`now` is what gets recorded) and returns the numbers of the
    /// migrations it applied. Afterwards it gives the people with an empty name key their key
    /// ([`Database::fill_name_keys`], migration 26).
    ///
    /// # Errors
    /// Fails with [`StoreError::SchemaTooNew`] when the database was changed by a newer program,
    /// with [`StoreError::MigrationMismatch`] when a recorded migration is not the one this
    /// program has, and when the database fails.
    pub async fn migrate(&self, now: Timestamp) -> Result<Vec<i64>, StoreError> {
        self.migrate_list(ALL, now).await
    }

    /// [`Database::migrate`] with the migrations `list`: the program's, or those of a test.
    async fn migrate_list(
        &self,
        list: &[Definition],
        now: Timestamp,
    ) -> Result<Vec<i64>, StoreError> {
        let done = self.migrate_through(list, i64::MAX, now).await?;
        // Data that the migrations only mark, made with the code that runs (migration 26).
        self.fill_name_keys().await?;
        Ok(done)
    }

    /// Brings the schema up to migration `last` and no further: the database of an older
    /// version, for the tests of an upgrade.
    ///
    /// # Errors
    /// The same as [`Database::migrate`].
    #[cfg(any(test, feature = "testing"))]
    pub async fn migrate_up_to(&self, last: i64, now: Timestamp) -> Result<Vec<i64>, StoreError> {
        self.migrate_until(last, now).await
    }

    #[cfg(any(test, feature = "testing"))]
    async fn migrate_until(&self, last: i64, now: Timestamp) -> Result<Vec<i64>, StoreError> {
        self.migrate_through(ALL, last, now).await
    }

    /// Applies the migrations of `list` up to number `last`, one transaction each.
    async fn migrate_through(
        &self,
        migrations: &[Definition],
        last: i64,
        now: Timestamp,
    ) -> Result<Vec<i64>, StoreError> {
        let mut done = Vec::new();
        loop {
            // One transaction for each migration, so a failure keeps the ones before it.
            let mut transaction = self.begin().await?;
            let applied = apply_next(
                &mut transaction,
                migrations,
                &|version| version <= last,
                now,
            )
            .await?;
            transaction.commit().await?;
            match applied {
                Some(version) => done.push(version),
                None => return Ok(done),
            }
        }
    }
}

/// Applies exactly the migrations `versions` and no other, inside `transaction`: the schema of
/// the database a backup was made from, which the rows of its copy are put back into before the
/// migrations it lacks run (ADR-0046, decision 4). Nothing of it stays when the transaction is
/// not committed.
pub(crate) async fn migrate_only_in(
    transaction: &mut Transaction,
    versions: &[i64],
    now: Timestamp,
) -> Result<Vec<i64>, StoreError> {
    let mut done = Vec::new();
    while let Some(version) = apply_next(
        transaction,
        ALL,
        &|version| versions.contains(&version),
        now,
    )
    .await?
    {
        done.push(version);
    }
    Ok(done)
}

/// How the migrations `applied` stand to the program with the migrations `list`.
fn state_of(list: &[Definition], applied: &[MigrationRecord]) -> Result<SchemaState, StoreError> {
    match check(list, applied) {
        Err(StoreError::SchemaTooNew { .. }) => Ok(SchemaState::Newer),
        Err(other) => Err(other),
        Ok(()) => Ok(if lacks_some(list, applied) {
            SchemaState::Older
        } else {
            SchemaState::Current
        }),
    }
}

/// Whether the migrations `applied` lack one of `list`: the one comparison of the two sets.
fn lacks_some(list: &[Definition], applied: &[MigrationRecord]) -> bool {
    list.iter()
        .any(|definition| !is_applied(applied, definition.version))
}

fn is_applied(applied: &[MigrationRecord], version: i64) -> bool {
    applied.iter().any(|record| record.version == version)
}

/// Applies the oldest migration of `list` that is `wanted` and not applied yet, inside
/// `transaction`, and returns its number; `None` when there is none.
async fn apply_next(
    transaction: &mut Transaction,
    list: &[Definition],
    wanted: &(dyn Fn(i64) -> bool + Sync),
    now: Timestamp,
) -> Result<Option<i64>, StoreError> {
    transaction.advisory_lock(LockClass::Migration, 0).await?;
    transaction.execute_schema(&record_table()).await?;
    let applied = read_applied(transaction).await?;
    check(list, &applied)?;
    let Some(next) = list
        .iter()
        .filter(|definition| {
            wanted(definition.version) && !is_applied(&applied, definition.version)
        })
        .min_by_key(|definition| definition.version)
    else {
        return Ok(None);
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
    record(transaction, next, now).await?;
    Ok(Some(next.version))
}

#[cfg(test)]
mod tests;
