//! The start of a newer program on the database of an older one (OPS-06, ADR-0046 decision 5).
//! The store decides by the set of migrations applied whether some are due, makes the backup
//! `before-upgrade` before the first one and puts it back when one fails
//! (`Database::upgrade`); this module opens the keys the backup is sealed with, says what happened
//! in plain words, and stops with an exit code of its own, so a service manager that reads it
//! does not start the program again and again on a database it cannot change.

use std::fmt;
use std::sync::Arc;

use bb_core::message::{self, Message};
use bb_store::backup::{CreatedBackup, VERSION};
use bb_store::{Database, Due, KeyRing, StoreError, Upgrade, UpgradeError};
use jiff::Timestamp;

use crate::cli::backup::places;
use crate::data_folder::{DataFolder, Existing, write_private};
use crate::output;
use crate::wiring::database_failed;

/// The exit code of a start that stopped at an upgrade: a database a newer program changed, one
/// with a migration this program does not have or has in another form, a backup before an
/// upgrade that failed, or a migration that failed and was undone by putting the backup back.
/// Starting again would stop the same way, so a service must not restart on it.
pub(crate) const UPGRADE_STOPPED: u8 = 65;

/// A start that stopped at an upgrade, with the words that say what happened and what to do.
#[derive(Debug)]
pub(crate) struct UpgradeStopped(String);

impl fmt::Display for UpgradeStopped {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for UpgradeStopped {}

/// The exit code of a failed command that stopped at an upgrade, or `None` for any other failure:
/// the one place that tells, for the exit code of the command line and of the service.
pub(crate) fn stop_code(error: &anyhow::Error) -> Option<u8> {
    error.is::<UpgradeStopped>().then_some(UPGRADE_STOPPED)
}

/// The database brought up to date, with the keys of the installation.
pub(crate) struct UpToDate {
    /// The keys of the installation.
    pub(crate) keys: Arc<KeyRing>,
    /// The backup made before an upgrade, when there was one: its record is written once the
    /// database has the table for it.
    pub(crate) backup: Option<CreatedBackup>,
}

/// What `version.txt` of the data folder says: the version of the program that last started on
/// the data, for the words of a later stop, and while an upgrade has not finished, the backup made
/// before it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Written {
    /// The version of the program that last started on the data.
    pub(crate) version: Option<String>,
    /// The file name, in `backups/` of the data folder, of the backup made before an upgrade that
    /// has begun to migrate and not finished. It is written before the first migration and goes
    /// when a start ends well, so a start after one that was cut off part way knows that the
    /// database may be half changed.
    pub(crate) upgrading: Option<String>,
}

/// What starts the line of an upgrade that has not finished, before the name of its backup.
const UPGRADING: &str = "upgrading ";

impl Written {
    /// What the file says; nothing when it is not there.
    pub(crate) fn read(data: &DataFolder) -> Self {
        let text = std::fs::read_to_string(data.version_file()).unwrap_or_default();
        let mut written = Self::default();
        for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
            match line.strip_prefix(UPGRADING) {
                Some(backup) => written.upgrading = Some(backup.trim().to_owned()),
                None => written.version = Some(line.to_owned()),
            }
        }
        written
    }

    /// Writes the file anew.
    fn write(&self, data: &DataFolder) -> std::io::Result<()> {
        let lines: Vec<String> = self
            .version
            .iter()
            .cloned()
            .chain(
                self.upgrading
                    .iter()
                    .map(|backup| format!("{UPGRADING}{backup}")),
            )
            .collect();
        write_private(
            &data.version_file(),
            lines.join("\n").as_bytes(),
            Existing::Replace,
        )
    }

    /// This program's version and no upgrade that has not finished: a start that ended well.
    fn started() -> Self {
        Self {
            version: Some(VERSION.to_owned()),
            upgrading: None,
        }
    }
}

/// Brings the database up to date and opens the keys. A database that has migrations already and
/// lacks some is upgraded with a backup first, so its keys are opened before (the backup is
/// sealed with them); a new database gets its keys after its tables, as at the first start.
///
/// # Errors
/// An [`UpgradeStopped`] when the upgrade stopped, a plain explanation when the database or the
/// keys fail.
pub(crate) async fn bring_up_to_date(
    database: &Database,
    data: &DataFolder,
    now: Timestamp,
) -> anyhow::Result<UpToDate> {
    let written = Written::read(data);
    let due = database
        .migrations_due()
        .await
        .map_err(|error| refused(error, &written))?;
    let up_to_date = if due == Due::Some {
        let keys = open_keys(database, data).await?;
        let upgraded = database
            .upgrade(
                &places(data, data.backups(), Arc::clone(&keys)),
                now,
                |backup| begin_upgrade(&written, backup, data),
            )
            .await;
        let backup = match upgraded {
            Ok(Upgrade::Upgraded { backup, migrations }) => {
                tracing::info!(migrations = migrations.len(), "database upgraded");
                output::line(
                    &message::server_upgrade_backup(backup.path.display(), VERSION).render(),
                );
                Some(backup)
            }
            Ok(Upgrade::Current | Upgrade::New { .. }) => None,
            Err(error) => {
                // The database is as it was before this start, and so is what is written down.
                if matches!(
                    &error,
                    UpgradeError::MigrationFailed {
                        put_back: Ok(()),
                        ..
                    }
                ) && let Err(cause) = written.write(data)
                {
                    tracing::warn!(%cause, "the version of the program could not be written down");
                }
                return Err(stopped(error, &written, data));
            }
        };
        UpToDate { keys, backup }
    } else {
        let applied = database
            .migrate(now)
            .await
            .map_err(|error| database_failed(&error))?;
        tracing::info!(migrations = applied.len(), "database ready");
        UpToDate {
            keys: open_keys(database, data).await?,
            backup: None,
        }
    };
    if let Err(error) = Written::started().write(data) {
        tracing::warn!(cause = %error, "the version of the program could not be written down");
    }
    Ok(up_to_date)
}

/// Writes down, before the first migration, that an upgrade has begun with `backup`. After a start
/// that was cut off part way, the backup of that start stays named: it holds the database as the
/// program used before left it, and the backup of a later start does not.
fn begin_upgrade(
    written: &Written,
    backup: &CreatedBackup,
    data: &DataFolder,
) -> std::io::Result<()> {
    if written.upgrading.is_some() {
        return Ok(());
    }
    Written {
        version: written.version.clone(),
        upgrading: backup
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned()),
    }
    .write(data)
    .map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("{}: {error}", data.version_file().display()),
        )
    })
}

async fn open_keys(database: &Database, data: &DataFolder) -> anyhow::Result<Arc<KeyRing>> {
    Ok(Arc::new(
        database
            .open_keys(&data.keys())
            .await
            .map_err(|error| database_failed(&error))?,
    ))
}

/// The words for a database this program refuses before anything changes.
fn refused(error: StoreError, written: &Written) -> anyhow::Error {
    let words = match error {
        // The version written down is the newer program's only when it is another one.
        StoreError::SchemaTooNew { .. } => match written.version.as_deref() {
            Some(version) if version != VERSION => {
                message::server_database_newer_version(VERSION, version)
            }
            _ => message::server_database_newer(VERSION),
        },
        StoreError::MigrationMismatch { version } => {
            message::server_database_unknown_change(version, VERSION)
        }
        other => return database_failed(&other),
    };
    stop(words.render())
}

/// The words for an upgrade that stopped, and what to do next.
fn stopped(error: UpgradeError, written: &Written, data: &DataFolder) -> anyhow::Error {
    // After an upgrade that did not finish, the database may be half changed: the backup made
    // before it is named, while it is there (the rotation may have taken it away), and nobody is
    // told that the program used before works with the database.
    let interrupted = written
        .upgrading
        .as_ref()
        .map(|backup| data.backups().join(backup))
        .filter(|backup| backup.is_file());
    let way_back = || match &interrupted {
        Some(backup) => message::server_upgrade_interrupted(backup.display()).render(),
        None => written
            .version
            .as_deref()
            .map_or_else(
                || Message::from(message::SERVER_UPGRADE_PREVIOUS),
                message::server_upgrade_previous_version,
            )
            .render(),
    };
    let words = match error {
        UpgradeError::NotStarted(error) => return refused(error, written),
        UpgradeError::BackupFailed(reason) => format!(
            "{} {}",
            message::server_upgrade_backup_failed(data.backups().display(), &reason, VERSION)
                .render(),
            way_back()
        ),
        UpgradeError::MigrationFailed {
            error,
            backup,
            put_back: Ok(()),
        } => format!(
            "{} {}",
            message::server_upgrade_failed(backup.display(), &error, VERSION).render(),
            way_back()
        ),
        UpgradeError::MigrationFailed {
            error,
            backup,
            put_back: Err(problem),
        } => {
            let words = message::server_upgrade_put_back_failed(
                backup.display(),
                &problem,
                &error,
                VERSION,
            )
            .render();
            // Nobody is told to start the program used before on it; an older backup is named.
            match interrupted {
                Some(_) => format!("{words} {}", way_back()),
                None => words,
            }
        }
    };
    stop(words)
}

/// A start that stopped at an upgrade, logged with its exit code.
fn stop(words: String) -> anyhow::Error {
    tracing::error!(
        exit_code = UPGRADE_STOPPED,
        "the start stopped at the upgrade"
    );
    UpgradeStopped(words).into()
}

#[cfg(test)]
#[path = "tests/upgrade.rs"]
mod tests;
