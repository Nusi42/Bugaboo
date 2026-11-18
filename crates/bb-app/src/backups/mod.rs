//! Backups as the use cases see them (OPS-07, ADR-0046 decisions 3 and 6): the list of the
//! backups, *Create a backup now*, and the schedule that creates one every day at the quiet hour
//! and keeps the newest of seven days and four weeks. Restoring is a command of the server, with
//! the server stopped; no use case restores.
//!
//! The bytes are the business of the port [`BackupFiles`] (the server copies the database and the
//! files); the records go through [`BackupStore`] with their event, and every backup writes its
//! line to the security log.

mod ports;
mod schedule;

use bb_core::backup::name::{BackupName, Purpose};
use bb_core::backup::run::{BackupEvent, BackupMaker, BackupRun};
use bb_core::event::{Actor as EventActor, DomainEvent};
use bb_core::id::{BackupId, UserId};
use bb_core::settings::BACKUPS_FOLDER;
use bb_policy::{Authorized, Masked, Permission, Place};

pub use ports::{BackupFile, BackupFiles, BackupStore, CreatedFile, NoBackupFiles};

use crate::auth::{AppError, AuthStore, Ctx, Stops, by, change_failed, internal};

/// How many records of backups the list shows.
const LISTED_RUNS: u64 = 50;

/// Who asks for the list of backups.
#[derive(Clone, Debug)]
pub struct ListBackupsQuery {
    /// The session of the administrator. An access key or an agent is turned away.
    pub session_token: String,
}

/// Who asks for a backup now.
#[derive(Clone, Debug)]
pub struct CreateBackup {
    /// The session of the administrator. An access key or an agent is turned away.
    pub session_token: String,
}

/// A backup file in the folder of the backups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupItem {
    /// Its name, with its moment and the version that created it.
    pub name: BackupName,
    /// Its size in bytes.
    pub size: u64,
}

/// The backups and their records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Backups {
    /// The backup files, newest first.
    pub files: Vec<BackupItem>,
    /// The newest records of backups created and restored, newest first.
    pub runs: Vec<BackupRun>,
}

impl Masked for Backups {}
impl Masked for BackupItem {}

/// The list of backups, for *Organization settings › System › Backups*.
///
/// # Errors
/// `auth.access-key-not-allowed` for an access key or an agent, `access.forbidden` without
/// *Change organization settings*.
pub async fn list_backups<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ListBackupsQuery,
) -> Result<Authorized<Backups>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let folder = ctx.backups_folder().await?;
    let files = ctx
        .services
        .backups
        .list(folder)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
        .into_iter()
        .map(|file| BackupItem {
            name: file.name,
            size: file.size,
        })
        .collect();
    let runs = ctx
        .store()
        .backup_runs(LISTED_RUNS)
        .await
        .map_err(|error| internal::<S>(&error))?;
    Ok(access.authorize(Backups { files, runs }))
}

/// *Create a backup now*: a backup while people go on working.
///
/// # Errors
/// `auth.access-key-not-allowed` for an access key or an agent, `access.forbidden` without
/// *Change organization settings*, an internal error when the backup could not be created.
pub async fn create_backup<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateBackup,
) -> Result<Authorized<BackupItem>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let created = ctx
        .created_backup(BackupMaker::Person, Some(actor.person.id))
        .await?;
    let stored = created.run.clone();
    ctx.store()
        .record_change(&created.event, move |transaction| {
            Box::pin(async move { S::insert_backup_run(transaction, &stored).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(
        created.run.security_action(),
        Some(actor.person.id),
        Some(created.event.entity),
    )
    .await;
    Ok(access.authorize(created.item))
}

/// A backup that was created, with its record and the event that records it.
pub(crate) struct Created {
    pub(crate) item: BackupItem,
    pub(crate) run: BackupRun,
    pub(crate) event: DomainEvent,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The folder the setting names, or `None` for the backups folder of the data folder.
    pub(crate) async fn backups_folder(&self) -> Result<Option<String>, AppError> {
        let layers = self.setting_layers_of(None).await?;
        let folder = layers.text(&BACKUPS_FOLDER);
        Ok((!folder.trim().is_empty()).then(|| folder.trim().to_owned()))
    }

    /// Creates a backup, and the record and event that the caller writes with `record_change`.
    pub(crate) async fn created_backup(
        &self,
        maker: BackupMaker,
        person: Option<UserId>,
    ) -> Result<Created, AppError> {
        let folder = self.backups_folder().await?;
        let now = self.now();
        let created = self
            .services
            .backups
            .create(folder, Purpose::Regular, now)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let run = BackupRun {
            id: BackupId::generate(&mut self.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?,
            event: BackupEvent::Made,
            name: created.name.clone(),
            size: i64::try_from(created.size).unwrap_or(i64::MAX),
            checksum: created.checksum,
            maker,
            person,
            at: now,
            lost_from: None,
            lost_to: None,
        };
        let entity = run
            .entity()
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let event = self.event(person.map_or(EventActor::System, by), entity, run.changes())?;
        Ok(Created {
            item: BackupItem {
                name: created.name,
                size: created.size,
            },
            run,
            event,
        })
    }
}
