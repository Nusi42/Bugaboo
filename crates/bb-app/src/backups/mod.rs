//! Backups as the use cases see them (OPS-07, ADR-0046 decisions 3 and 6): the list of the
//! backups, *Create a backup now*, and the schedule that creates one every day at the quiet hour
//! and keeps the newest of seven days and four weeks. Restoring is a command of the server, with
//! the server stopped; no use case restores.
//!
//! *Create a backup now* only asks: the job `backups.now` of the long lane makes the backup
//! (ADR-0017), because a copy of a large organization takes far longer than a request may. The
//! list says while it runs and when it failed; the backup and its record appear when it is made.
//!
//! The bytes are the business of the port [`BackupFiles`] (the server copies the database and the
//! files); the records go through [`BackupStore`] with their event, and every backup writes its
//! line to the security log.

mod ports;
mod schedule;

use std::sync::atomic::{AtomicBool, Ordering};

use bb_core::backup::name::{BackupName, Purpose};
use bb_core::backup::run::{
    AskedBackupState, BackupEvent, BackupMaker, BackupRun, asked_changes, entity_of,
};
use bb_core::error_code::ErrorCode;
use bb_core::event::{Actor as EventActor, DomainEvent};
use bb_core::id::{BackupId, JobId, UserId};
use bb_core::job::{Job, JobState};
use bb_core::network::NetworkPrefix;
use bb_core::people_kinds::PeopleKind;
use bb_core::settings::{BACKUPS_FOLDER, SettingDefinition, SettingValue};
use bb_policy::{Authorized, Masked, PeopleContext, Permission, PersonRef, Place};
use jiff::Timestamp;
use serde_json::{Value, json};

pub use ports::{BackupFile, BackupFiles, BackupStore, KeepLease, NoBackupFiles};

use crate::auth::{AppError, AuthStore, Ctx, SecurityEvent, Stops, by, change_failed, internal};
use crate::jobs::JobKind;
use crate::projects::Why;

/// The keys of the payload of the job that makes a backup asked for (`backups.now`).
mod payload {
    /// The ID its record gets, the one the event of the ask names.
    pub(super) const BACKUP: &str = "backup";
    /// The person who asked.
    pub(super) const PERSON: &str = "person";
    /// The network the person asked from, for the line of the security log.
    pub(super) const NETWORK: &str = "network";
}

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

/// A backup asked for with *Create a backup now* that is not made yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AskedBackup {
    /// Whether it runs or failed.
    pub state: AskedBackupState,
    /// When it was asked for.
    pub asked_at: Timestamp,
}

/// The backups and their records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Backups {
    /// The backup files, newest first.
    pub files: Vec<BackupItem>,
    /// The newest records of backups created and restored, newest first.
    pub runs: Vec<BackupRun>,
    /// The people the records name, as the viewer may see them (*Who sees people*, and *Former
    /// user* after a removal), once for each person.
    pub people: Vec<(UserId, PersonRef)>,
    /// The backup asked for with the button that is not made yet: while its job waits or works,
    /// and after it failed until a backup is made. `None` otherwise.
    pub asked: Option<AskedBackup>,
}

impl Masked for Backups {}
impl Masked for BackupItem {}
impl Masked for AskedBackup {}

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
    let asked = ctx.asked_backup(&runs).await?;
    let mut people: Vec<(UserId, PersonRef)> = Vec::new();
    for person in runs.iter().filter_map(|run| run.person) {
        if people.iter().all(|(known, _)| *known != person) {
            let shown = ctx
                .person_ref(
                    &access,
                    PeopleKind::Authors,
                    PeopleContext::Ordinary,
                    person,
                )
                .await?;
            people.push((person, shown));
        }
    }
    Ok(access.authorize(Backups {
        files,
        runs,
        people,
        asked,
    }))
}

/// *Create a backup now*: asks for a backup, which the long lane of the jobs makes while people go
/// on working (`backups.now`, ADR-0017). The answer comes at once; the list of backups says while
/// it runs, and shows the backup when it is made or that it failed.
///
/// # Errors
/// `auth.access-key-not-allowed` for an access key or an agent, `access.forbidden` without
/// *Change organization settings*, `backup.running` while a backup asked for is not made yet or
/// this server makes one.
pub async fn create_backup<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateBackup,
) -> Result<Authorized<AskedBackup>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    if ctx.services.backup_running.load(Ordering::Acquire) {
        return Err(AppError::Refused(ErrorCode::BackupRunning));
    }
    let now = ctx.now();
    let id = ctx.new_backup_id()?;
    let person = actor.person.id;
    let entity = entity_of(id).map_err(|error| AppError::Internal(error.to_string()))?;
    let event = ctx.event(by(person), entity, asked_changes(BackupMaker::Person))?;
    let job =
        JobId::generate(&mut ctx.ids()).map_err(|error| AppError::Internal(error.to_string()))?;
    let kind = JobKind::BackupNow.name()?;
    let mut asked = json!({
        payload::BACKUP: id.to_string(),
        payload::PERSON: person.to_string(),
    });
    if let Some(network) = &ctx.request.network {
        asked[payload::NETWORK] = json!(network.as_str());
    }
    let why = Why::default();
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // Looked at inside the change, under its lock, so two asks at the same moment
                // never both queue a backup.
                S::lock_backup_asks(transaction).await?;
                let newest = S::newest_job_in(transaction, JobKind::BackupNow).await?;
                if asked_of(newest, &[])
                    .is_some_and(|asked| asked.state == AskedBackupState::Running)
                {
                    return Err(stopped.stop::<S>(ErrorCode::BackupRunning));
                }
                S::enqueue_job(transaction, job, &kind, &asked, now, now).await
            })
        })
        .await
        .map_err(|error| why.said_or_failed::<S>(&error))?;
    Ok(access.authorize(AskedBackup {
        state: AskedBackupState::Running,
        asked_at: now,
    }))
}

/// The mark that a backup of this server is being made, taken back when it is dropped.
struct OneAtATime<'a>(&'a AtomicBool);

impl<'a> OneAtATime<'a> {
    /// The mark, or `None` while another backup holds it.
    fn take(running: &'a AtomicBool) -> Option<Self> {
        running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self(running))
    }
}

impl Drop for OneAtATime<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// A backup that was created, with its record and the event that records it.
pub(crate) struct Created {
    pub(crate) run: BackupRun,
    pub(crate) event: DomainEvent,
}

/// What [`record_backup_run`] wrote: the line of the security log, and whether it was written.
pub struct Recorded<E> {
    /// The line of the security log that points to the record.
    pub line: SecurityEvent,
    /// Whether the line was written; the record is written either way.
    pub written: Result<(), E>,
}

/// Writes the record of a backup made or restored together with its event, and then its line in
/// the security log, which points to the record (ADR-0046, decisions 3 and 4). This is the one way
/// a backup is recorded: by the route, the schedule, the command line and the restore.
///
/// # Errors
/// The error of the record, which is then not written. A line that could not be written after the
/// record is in [`Recorded::written`]: what already happened is not undone (SEC-15).
pub async fn record_backup_run<S: AuthStore>(
    store: &S,
    run: &BackupRun,
    event: &DomainEvent,
    network: Option<NetworkPrefix>,
) -> Result<Recorded<S::Error>, S::Error> {
    let stored = run.clone();
    store
        .record_change(event, move |transaction| {
            Box::pin(async move { S::insert_backup_run(transaction, &stored).await })
        })
        .await?;
    let line = SecurityEvent {
        time: run.at,
        person: run.person,
        action: run.security_action(),
        subject: Some(event.entity.clone()),
        network,
    };
    let written = store.append_security_log(&line).await;
    Ok(Recorded { line, written })
}

/// Where the backup asked for with the button stands, from the newest job of `backups.now` and the
/// records `runs` (see [`Backups::asked`]).
fn asked_of(newest: Option<Job>, runs: &[BackupRun]) -> Option<AskedBackup> {
    newest.and_then(|job| {
        let made_since = runs
            .iter()
            .any(|run| run.event == BackupEvent::Made && run.at >= job.created_at);
        let state = match job.state {
            JobState::Queued => AskedBackupState::Running,
            JobState::Dead if !made_since => AskedBackupState::Failed,
            JobState::Dead => return None,
        };
        Some(AskedBackup {
            state,
            asked_at: job.created_at,
        })
    })
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Refuses a folder for backups that the system says lies on a share of another computer (a
    /// mapped network drive, a mounted share), which the format of the setting cannot see in the
    /// text of the path; the server looks again when a backup runs (ADR-0046).
    ///
    /// # Errors
    /// `setting.folder-not-local` for such a folder.
    pub(crate) fn refuse_backups_elsewhere(
        &self,
        definition: &SettingDefinition,
        value: &SettingValue,
    ) -> Result<(), AppError> {
        match value {
            SettingValue::Text(folder)
                if definition.key() == BACKUPS_FOLDER.definition().key()
                    && !folder.trim().is_empty()
                    && self.services.backups.on_another_computer(folder.trim()) =>
            {
                Err(AppError::Refused(ErrorCode::SettingFolderNotLocal))
            }
            _ => Ok(()),
        }
    }

    /// The folder the setting names, or `None` for the backups folder of the data folder.
    pub(crate) async fn backups_folder(&self) -> Result<Option<String>, AppError> {
        let layers = self.setting_layers_of(None).await?;
        let folder = layers.text(&BACKUPS_FOLDER);
        Ok((!folder.trim().is_empty()).then(|| folder.trim().to_owned()))
    }

    /// A new ID for the record of a backup.
    pub(crate) fn new_backup_id(&self) -> Result<BackupId, AppError> {
        BackupId::generate(&mut self.ids()).map_err(|error| AppError::Internal(error.to_string()))
    }

    /// The backup asked for with the button that is not made yet (see [`Backups::asked`]): the
    /// newest job of `backups.now` while it waits or works, or when it died after the newest of
    /// `runs` was made.
    async fn asked_backup(&self, runs: &[BackupRun]) -> Result<Option<AskedBackup>, AppError> {
        let newest = self
            .store()
            .newest_job(JobKind::BackupNow)
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(asked_of(newest, runs))
    }

    /// The job `backups.now`: makes the backup a person asked for, with the ID the ask gave it.
    /// A job that runs again after its backup was recorded does nothing.
    pub(crate) async fn run_backup_now(&self, job: &Job) -> Result<(), AppError> {
        let text = |key: &str| job.payload.get(key).and_then(Value::as_str);
        let unreadable = || AppError::Refused(ErrorCode::InvalidRequest);
        let id: BackupId = text(payload::BACKUP)
            .and_then(|id| id.parse().ok())
            .ok_or_else(unreadable)?;
        let person: UserId = text(payload::PERSON)
            .and_then(|id| id.parse().ok())
            .ok_or_else(unreadable)?;
        let network = text(payload::NETWORK).and_then(NetworkPrefix::from_stored);
        let recorded = self
            .store()
            .backup_runs(LISTED_RUNS)
            .await
            .map_err(|error| internal::<S>(&error))?;
        if recorded.iter().any(|run| run.id == id) {
            return Ok(());
        }
        self.remove_backup_leftovers().await?;
        let created = self
            .created_backup(id, BackupMaker::Person, Some(person), self.keep_lease(job))
            .await?;
        self.record_run(&created, network).await
    }

    /// Creates a backup with the record `id`, and the record and event that the caller writes with
    /// `record_run`; with `keep`, the job that makes it keeps its lease meanwhile. One backup is
    /// made at a time on a server: while one is made, the button and the schedule are turned away.
    pub(crate) async fn created_backup<'k>(
        &'k self,
        id: BackupId,
        maker: BackupMaker,
        person: Option<UserId>,
        keep: Option<KeepLease<'k>>,
    ) -> Result<Created, AppError> {
        let _running = OneAtATime::take(&self.services.backup_running)
            .ok_or(AppError::Refused(ErrorCode::BackupRunning))?;
        let folder = self.backups_folder().await?;
        // The backup takes a copy of the replay list: the lines that wait go into it first.
        self.write_replay_list().await;
        let now = self.now();
        let created = self
            .services
            .backups
            .create(folder, Purpose::Regular, now, keep)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let run = BackupRun::new(id, BackupEvent::Made, &created, maker, person, now);
        let entity = run
            .entity()
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let event = self.event(person.map_or(EventActor::System, by), entity, run.changes())?;
        Ok(Created { run, event })
    }

    /// Records a backup this server created, with [`record_backup_run`], its line of the security
    /// log naming `network`; a line that could not be written is counted and reported like every
    /// other (`note`).
    pub(crate) async fn record_run(
        &self,
        created: &Created,
        network: Option<NetworkPrefix>,
    ) -> Result<(), AppError> {
        let recorded = record_backup_run(self.store(), &created.run, &created.event, network)
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
        self.log_written(&recorded.line, recorded.written);
        Ok(())
    }
}
