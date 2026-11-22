//! The backup files of an installation, as the use cases see them ([`BackupFiles`]): created into
//! a folder of the installation while people work, listed, deleted, and cleared of what a backup
//! that never finished left behind. The server and the developer tool that makes the sample
//! organization (`bb-sample`) both use this one adapter; only the choice of the folder is theirs.

use std::path::PathBuf;
use std::sync::Arc;

use bb_app::backups::{BackupFile, BackupFiles};
use bb_app::ports::BoxFuture;
use bb_core::backup::name::{BackupName, Purpose};
use bb_core::backup::run::CreatedFile;
use jiff::Timestamp;

use super::{BackupPlaces, backups_in, remove_leftovers};
use crate::db::Database;

/// The folder a backup goes into: the folder the setting `backups.folder` names (`Some`), or the
/// backups folder of the data folder (`None`). The caller decides whether a named folder may be
/// used (the server refuses one that others can open).
pub type FolderOf = Arc<dyn Fn(Option<&str>) -> std::io::Result<PathBuf> + Send + Sync>;

/// Whether the system says that a folder lies on a share of another computer (a mapped network
/// drive, a mounted share); the caller knows its system, so it answers.
pub type Elsewhere = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// The backup files of one installation.
pub struct FolderBackups {
    database: Database,
    /// Where everything but the folder of the backups is; the folder comes from `folder_of`.
    places: BackupPlaces,
    folder_of: FolderOf,
    /// `None`: the adapter cannot tell, so no folder is taken for one on another computer.
    elsewhere: Option<Elsewhere>,
}

impl FolderBackups {
    /// The backups of the installation whose database and data folder `places` name.
    #[must_use]
    pub fn new(database: Database, places: BackupPlaces, folder_of: FolderOf) -> Self {
        Self {
            database,
            places,
            folder_of,
            elsewhere: None,
        }
    }

    /// The same backups, with the judgment of whether a folder lies on another computer.
    #[must_use]
    pub fn judging_elsewhere(self, elsewhere: Elsewhere) -> Self {
        Self {
            elsewhere: Some(elsewhere),
            ..self
        }
    }

    fn folder(&self, named: Option<&str>) -> std::io::Result<PathBuf> {
        (self.folder_of)(named)
    }
}

impl BackupFiles for FolderBackups {
    fn on_another_computer(&self, folder: &str) -> bool {
        self.elsewhere
            .as_ref()
            .is_some_and(|elsewhere| elsewhere(folder))
    }

    fn create(
        &self,
        folder: Option<String>,
        purpose: Purpose,
        now: Timestamp,
    ) -> BoxFuture<'_, std::io::Result<CreatedFile>> {
        Box::pin(async move {
            let places = BackupPlaces {
                folder: self.folder(folder.as_deref())?,
                ..self.places.clone()
            };
            self.database
                .create_backup(&places, purpose, now)
                .await
                .map(|created| created.file)
                .map_err(|error| std::io::Error::other(error.to_string()))
        })
    }

    fn list(&self, folder: Option<String>) -> BoxFuture<'_, std::io::Result<Vec<BackupFile>>> {
        Box::pin(async move {
            let found = backups_in(&self.folder(folder.as_deref())?)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            Ok(found
                .into_iter()
                .map(|(name, _, size)| BackupFile { name, size })
                .collect())
        })
    }

    fn delete(
        &self,
        folder: Option<String>,
        name: BackupName,
    ) -> BoxFuture<'_, std::io::Result<()>> {
        Box::pin(async move {
            std::fs::remove_file(self.folder(folder.as_deref())?.join(name.to_string()))
        })
    }

    fn remove_leftovers(&self, folder: Option<String>) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            match self.folder(folder.as_deref()) {
                Ok(folder) => remove_leftovers(&folder),
                Err(error) => tracing::warn!(cause = %error, "the backups folder is not there"),
            }
        })
    }
}
