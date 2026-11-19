//! A newer program started on an older database (OPS-06, ADR-0046 decision 5): when the set of
//! migrations applied lacks some of the program's, a backup `before-upgrade` is made before the
//! first migration, through the one routine of backups; when a migration fails, that backup is put
//! back and the start stops. A new database gets its migrations with no backup (there is nothing
//! to keep), and a database a newer program changed is refused before anything changes.

use std::path::PathBuf;

use bb_core::backup::name::Purpose;
use jiff::Timestamp;

use super::{ALL, Definition, check, lacks_some, record_table};
use crate::backup::{BackupPlaces, CreatedBackup, put_back_before_upgrade};
use crate::db::Database;
use crate::error::StoreError;
use crate::locks::LockClass;

/// Which migrations a start finds due.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Due {
    /// None: the database has every migration of the program.
    Nothing,
    /// All of them: the database is new. Nothing in it needs a backup.
    All,
    /// Some: the database was made by an older program. A backup comes first.
    Some,
}

/// What a start did to the schema.
#[derive(Debug)]
pub enum Upgrade {
    /// Nothing was due.
    Current,
    /// A new database got every migration.
    New {
        /// The migrations that ran.
        migrations: Vec<i64>,
    },
    /// An older database was backed up and got the migrations it lacked.
    Upgraded {
        /// The backup made before the first migration.
        backup: CreatedBackup,
        /// The migrations that ran.
        migrations: Vec<i64>,
    },
}

/// Why a start stopped before the server could work on the database.
#[derive(Debug)]
pub enum UpgradeError {
    /// No backup was needed or made: the database could not be read, a newer program changed it,
    /// a migration it records is not this program's (nothing was changed then), or a migration of
    /// a new database failed.
    NotStarted(StoreError),
    /// The backup before the first migration failed, so no migration ran.
    BackupFailed(StoreError),
    /// A migration failed. The backup made before was put back (`put_back` says whether that
    /// worked); the program that ran before works with it.
    MigrationFailed {
        /// What failed.
        error: StoreError,
        /// The backup made before the first migration.
        backup: PathBuf,
        /// Whether putting it back went well.
        put_back: Result<(), StoreError>,
    },
}

impl Database {
    /// Which migrations are due, by comparing the set applied with the program's (see the module).
    /// The record of migrations is made when the database has none yet.
    ///
    /// # Errors
    /// [`StoreError::SchemaTooNew`] for a database a newer program changed,
    /// [`StoreError::MigrationMismatch`] for a recorded migration this program does not have or
    /// has differently, and when the database fails.
    pub async fn migrations_due(&self) -> Result<Due, StoreError> {
        self.due_of(ALL).await
    }

    async fn due_of(&self, list: &[Definition]) -> Result<Due, StoreError> {
        let mut transaction = self.begin().await?;
        // Under the lock of the migrations, like every change of the schema: two programs that
        // start at once on PostgreSQL would otherwise both make the record table.
        transaction.advisory_lock(LockClass::Migration, 0).await?;
        transaction.execute_schema(&record_table()).await?;
        transaction.commit().await?;
        let applied = self.applied_migrations().await?;
        check(list, &applied)?;
        Ok(if !lacks_some(list, &applied) {
            Due::Nothing
        } else if applied.is_empty() {
            Due::All
        } else {
            Due::Some
        })
    }

    /// Brings the schema up to date as a start does (see the module): `places` says where the
    /// backup before an upgrade goes and holds the keys that seal it. `before_migrating` is called
    /// once the backup is made and before the first migration, so the caller can write down that
    /// an upgrade has begun: a start that is cut off part way (the computer stops) leaves a
    /// database that is half changed, and the next start must not tell anybody that the program
    /// used before works with it. When it fails, no migration runs.
    ///
    /// # Errors
    /// An [`UpgradeError`], which says what was changed and what was put back.
    pub async fn upgrade(
        &self,
        places: &BackupPlaces,
        now: Timestamp,
        before_migrating: impl FnOnce(&CreatedBackup) -> std::io::Result<()> + Send,
    ) -> Result<Upgrade, UpgradeError> {
        self.upgrade_with(ALL, places, now, before_migrating).await
    }

    /// [`Database::upgrade`] with the migrations `list`: the program's, or those of a test.
    pub(super) async fn upgrade_with(
        &self,
        list: &[Definition],
        places: &BackupPlaces,
        now: Timestamp,
        before_migrating: impl FnOnce(&CreatedBackup) -> std::io::Result<()> + Send,
    ) -> Result<Upgrade, UpgradeError> {
        match self.due_of(list).await.map_err(UpgradeError::NotStarted)? {
            Due::Nothing => Ok(Upgrade::Current),
            Due::All => self
                .migrate_list(list, now)
                .await
                .map(|migrations| Upgrade::New { migrations })
                .map_err(UpgradeError::NotStarted),
            Due::Some => {
                let backup = self
                    .create_backup(places, Purpose::BeforeUpgrade, now)
                    .await
                    .map_err(UpgradeError::BackupFailed)?;
                tracing::info!("backup created before the upgrade");
                before_migrating(&backup)
                    .map_err(|error| UpgradeError::BackupFailed(StoreError::Backup(error)))?;
                match self.migrate_list(list, now).await {
                    Ok(migrations) => Ok(Upgrade::Upgraded { backup, migrations }),
                    Err(error) => {
                        let put_back =
                            put_back_before_upgrade(&backup.path, self, places, now).await;
                        Err(UpgradeError::MigrationFailed {
                            error,
                            backup: backup.path,
                            put_back,
                        })
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/upgrade.rs"]
mod tests;
