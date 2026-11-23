//! The schedule of backups (`backups.daily`, ADR-0046 decision 3): every day at the quiet hour of
//! the organization a backup, while the setting is on; then the backups the rotation no longer
//! keeps are deleted (the newest of `backups.keep-daily` days and `backups.keep-weekly` weeks; a
//! backup made before a restore after the keep period of the security log; of the backups made
//! before an upgrade the newest `backups.keep-before-upgrade`; a name Bugaboo does not read never).

use bb_core::backup::name::Purpose;
use bb_core::backup::rotation::{Keep, to_delete};
use bb_core::backup::run::BackupMaker;
use bb_core::settings::{
    BACKUPS_DAILY, BACKUPS_KEEP_BEFORE_UPGRADE, BACKUPS_KEEP_DAILY, BACKUPS_KEEP_WEEKLY,
    SECURITY_LOG_KEEP_DAYS,
};
use bb_core::wording::region::Region;
use jiff::tz::TimeZone;

use super::KeepLease;
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
    /// longer keeps; with `keep`, the job of the round keeps its lease while the backup is made.
    pub(crate) async fn backup_round<'k>(
        &'k self,
        keep: Option<KeepLease<'k>>,
    ) -> Result<Round, AppError> {
        self.remove_backup_leftovers().await?;
        let layers = self.setting_layers_of(None).await?;
        let mut round = Round::default();
        if layers.yes(&BACKUPS_DAILY) {
            let id = self.new_backup_id()?;
            let created = self
                .created_backup(id, BackupMaker::Schedule, None, keep)
                .await?;
            self.record_run(&created, None).await?;
            round.created = true;
        }
        // What the rotation no longer keeps goes through the one shape of deleting (ADR-0045).
        let step = self.keep_step(crate::jobs::JobKind::BackupsDaily).await?;
        round.deleted = usize::try_from(step.rows).unwrap_or(usize::MAX);
        Ok(round)
    }

    /// Removes what backups that never finished left in the backups folder of the data folder
    /// and in the one the setting names. Only the backups of the server do it, the daily backup
    /// job and the backup asked for with the button, each before it starts; never the command
    /// line, and the work folder of a backup that still runs stays (ADR-0046).
    pub(super) async fn remove_backup_leftovers(&self) -> Result<(), AppError> {
        let named = self.backups_folder().await?;
        let backups = &self.services.backups;
        backups.remove_leftovers(None).await;
        if named.is_some() {
            backups.remove_leftovers(named).await;
        }
        Ok(())
    }

    /// Deletes the backups the rotation no longer keeps, and says how many: the row *Backups* of
    /// the keep rules (ADR-0045), which the job of the schedule deletes through the one shape of
    /// deleting (`Ctx::keep_step`), as [`Self::backup_round`] does. Safe to run again.
    pub(crate) async fn prune_backups(&self) -> Result<usize, AppError> {
        let layers = self.setting_layers_of(None).await?;
        let keep = Keep {
            daily: usize::try_from(layers.number(&BACKUPS_KEEP_DAILY)).unwrap_or(1),
            weekly: usize::try_from(layers.number(&BACKUPS_KEEP_WEEKLY)).unwrap_or(0),
            before_restore: bb_core::time::days(layers.number(&SECURITY_LOG_KEEP_DAYS)),
            before_upgrade: usize::try_from(layers.number(&BACKUPS_KEEP_BEFORE_UPGRADE))
                .unwrap_or(1),
        };
        // The days and weeks of the organization's time zone.
        let zone = Region::of(&layers).zone().clone();
        let folder = self.backups_folder().await?;
        let mut deleted = self.prune_in(folder.clone(), keep, &zone, None).await?;
        // A new program makes its backup before an upgrade in the backups folder of the data
        // folder, whatever folder the setting names (ADR-0046, decision 5): there only those go.
        if folder.is_some() {
            deleted += self
                .prune_in(None, keep, &zone, Some(Purpose::BeforeUpgrade))
                .await?;
        }
        Ok(deleted)
    }

    /// Deletes in `folder` what the rotation no longer keeps, of the backups made for `only` when
    /// it is given, and says how many.
    async fn prune_in(
        &self,
        folder: Option<String>,
        keep: Keep,
        zone: &TimeZone,
        only: Option<Purpose>,
    ) -> Result<usize, AppError> {
        let files = self
            .services
            .backups
            .list(folder.clone())
            .await
            .map_err(|error| io(&error))?;
        let names: Vec<_> = files
            .into_iter()
            .map(|file| file.name)
            .filter(|name| only.is_none_or(|purpose| name.purpose == purpose))
            .collect();
        let mut deleted = 0;
        for name in to_delete(&names, keep, zone, self.now()) {
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
