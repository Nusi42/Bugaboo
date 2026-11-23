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

use super::work::blocking;
use super::{BackupPlaces, backups_in, io, remove_leftovers};
use crate::db::Database;
use crate::error::StoreError;

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

    /// The folder of the backups; the system is asked on a blocking thread (whether a named folder
    /// lies on another computer can take a while).
    async fn folder(&self, named: Option<String>) -> std::io::Result<PathBuf> {
        let folder_of = Arc::clone(&self.folder_of);
        on_disk(move || folder_of(named.as_deref())).await
    }
}

/// Runs file work on a blocking thread of the runtime ([`blocking`]), with the errors of the port.
async fn on_disk<T: Send + 'static>(
    work: impl FnOnce() -> std::io::Result<T> + Send + 'static,
) -> std::io::Result<T> {
    blocking(move || work().map_err(io))
        .await
        .map_err(|error| plain(&error))
}

fn plain(error: &StoreError) -> std::io::Error {
    std::io::Error::other(error.to_string())
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
                folder: self.folder(folder).await?,
                ..self.places.clone()
            };
            self.database
                .create_backup(&places, purpose, now)
                .await
                .map(|created| created.file)
                .map_err(|error| plain(&error))
        })
    }

    fn list(&self, folder: Option<String>) -> BoxFuture<'_, std::io::Result<Vec<BackupFile>>> {
        Box::pin(async move {
            let folder = self.folder(folder).await?;
            let found = on_disk(move || backups_in(&folder).map_err(|error| plain(&error))).await?;
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
            let file = self.folder(folder).await?.join(name.to_string());
            on_disk(move || std::fs::remove_file(file)).await
        })
    }

    fn remove_leftovers(&self, folder: Option<String>) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            match self.folder(folder).await {
                Ok(folder) => {
                    let removed = on_disk(move || {
                        remove_leftovers(&folder);
                        Ok(())
                    })
                    .await;
                    if let Err(error) = removed {
                        tracing::warn!(cause = %error, "what a backup left behind stays");
                    }
                }
                Err(error) => tracing::warn!(cause = %error, "the backups folder is not there"),
            }
        })
    }
}
