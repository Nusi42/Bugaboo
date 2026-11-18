//! The schedule of backups (`backups.daily`, ADR-0046 decision 3): every day at the quiet hour of
//! the organization a backup, while the setting is on; then the backups the rotation no longer
//! keeps are deleted (the newest of `backups.keep-daily` days and `backups.keep-weekly` weeks; a
//! backup made before a restore after the keep period of the security log; a name Bugaboo does
//! not read never).

use bb_core::backup::rotation::{Keep, to_delete};
use bb_core::backup::run::BackupMaker;
use bb_core::settings::{
    BACKUPS_DAILY, BACKUPS_KEEP_DAILY, BACKUPS_KEEP_WEEKLY, SECURITY_LOG_KEEP_DAYS,
};
use bb_core::wording::region::Region;

use crate::auth::{AppError, AuthStore, Ctx};

/// What a round of the schedule did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Round {
    /// Whether it created a backup (the setting may be off).
    pub(crate) created: bool,
    /// How many backups it deleted.
    pub(crate) deleted: usize,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Creates the backup of the day when the setting is on, and deletes what the rotation no
    /// longer keeps.
    pub(crate) async fn backup_round(&self) -> Result<Round, AppError> {
        let layers = self.setting_layers_of(None).await?;
        let mut round = Round::default();
        if layers.yes(&BACKUPS_DAILY) {
            let created = self.created_backup(BackupMaker::Schedule, None).await?;
            self.record_run(&created).await?;
            round.created = true;
        }
        // What the rotation no longer keeps goes through the one shape of deleting (ADR-0045).
        let step = self.keep_step(crate::jobs::JobKind::BackupsDaily).await?;
        round.deleted = usize::try_from(step.rows).unwrap_or(usize::MAX);
        Ok(round)
    }

    /// Deletes the backups the rotation no longer keeps, and says how many: the row *Backups* of
    /// the keep rules (ADR-0045), which the job of the schedule deletes through the one shape of
    /// deleting (`Ctx::keep_step`), as [`Self::backup_round`] does. Safe to run again.
    pub(crate) async fn prune_backups(&self) -> Result<usize, AppError> {
        let layers = self.setting_layers_of(None).await?;
        let mut deleted = 0;
        let keep = Keep {
            daily: usize::try_from(layers.number(&BACKUPS_KEEP_DAILY)).unwrap_or(1),
            weekly: usize::try_from(layers.number(&BACKUPS_KEEP_WEEKLY)).unwrap_or(0),
            before_restore: bb_core::time::days(layers.number(&SECURITY_LOG_KEEP_DAYS)),
        };
        let folder = self.backups_folder().await?;
        let files = self
            .services
            .backups
            .list(folder.clone())
            .await
            .map_err(|error| io(&error))?;
        let names: Vec<_> = files.into_iter().map(|file| file.name).collect();
        // The days and weeks of the organization's time zone.
        let zone = Region::of(&layers).zone().clone();
        for name in to_delete(&names, keep, &zone, self.now()) {
            self.services
                .backups
                .delete(folder.clone(), name.clone())
                .await
                .map_err(|error| io(&error))?;
            deleted += 1;
        }
        Ok(deleted)
    }
}

fn io(error: &std::io::Error) -> AppError {
    AppError::Internal(error.to_string())
}
