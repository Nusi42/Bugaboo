//! Backups (OPS-07, ADR-0046 decision 3): the list of the backups with their records, and *Create a
//! backup now*. Both need *Change organization settings* and a session; an access key or an agent
//! is turned away. A backup is restored from the command line of the server, with the server
//! stopped (`bugaboo restore <file>`): there is no route that restores.
//!
//! *Create a backup now* answers `202` at once: the long lane of the jobs makes the backup, and the
//! list says while it runs (`asked`), then shows the backup and its record, or that it failed.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use bb_app::backups::{AskedBackup, BackupItem, Backups, CreateBackup, ListBackupsQuery};
use bb_core::backup::name::Purpose;
use bb_core::backup::run::{AskedBackupState, BackupRun};
use bb_core::hex;
use bb_core::id::UserId;
use bb_policy::{Masked, PersonRef};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, NotSignedInAnswer, Session, answer_authorized,
};
use crate::errors::ApiError;

/// A backup file in the folder of the backups.
#[derive(Serialize, ToSchema)]
pub(crate) struct BackupFileBody {
    /// The name of the file, such as `bugaboo-20261118T031500Z-0.6.0.backup`.
    name: String,
    /// The moment of the copy: every change saved before it is in the backup.
    moment: String,
    /// The version of Bugaboo that created it.
    version: String,
    /// Whether a restore created it of what was there before (kept for the keep period of the
    /// security log).
    before_restore: bool,
    /// The size in bytes.
    size: u64,
}

impl From<&BackupItem> for BackupFileBody {
    fn from(item: &BackupItem) -> Self {
        Self {
            name: item.name.to_string(),
            moment: item.name.moment.to_string(),
            version: item.name.version.clone(),
            before_restore: item.name.purpose == Purpose::BeforeRestore,
            size: item.size,
        }
    }
}

/// The record of a backup created or restored.
#[derive(Serialize, ToSchema)]
pub(crate) struct BackupRunBody {
    /// `made` or `restored`.
    event: String,
    /// The name of the backup.
    name: String,
    /// Its size in bytes.
    size: i64,
    /// Its SHA-256, as 64 hex digits.
    checksum: String,
    /// `schedule`, `command-line`, `person` or `upgrade`.
    maker: String,
    /// The person who pressed *Create a backup now*, as the viewer may see them.
    person: Option<PersonRef>,
    /// When it happened.
    at: String,
    /// For a restore: the first entry of the security log that is gone.
    lost_from: Option<i64>,
    /// For a restore: the last entry of the security log that is gone.
    lost_to: Option<i64>,
}

impl BackupRunBody {
    fn of(run: &BackupRun, people: &[(UserId, PersonRef)]) -> Self {
        Self {
            event: run.event.code().to_owned(),
            name: run.name.to_string(),
            size: run.size,
            checksum: hex::encode(run.checksum.as_bytes()),
            maker: run.maker.code().to_owned(),
            person: run.person.and_then(|person| {
                people
                    .iter()
                    .find(|(known, _)| *known == person)
                    .map(|(_, shown)| shown.clone())
            }),
            at: run.at.to_string(),
            lost_from: run.lost_from,
            lost_to: run.lost_to,
        }
    }
}

/// A backup asked for with *Create a backup now* that is not made yet.
#[derive(Serialize, ToSchema)]
pub(crate) struct AskedBackupBody {
    /// `running` while its job waits or works; `failed` when the job gave up and no backup was
    /// made since.
    status: AskedBackupState,
    /// When it was asked for.
    asked_at: String,
}

impl From<&AskedBackup> for AskedBackupBody {
    fn from(asked: &AskedBackup) -> Self {
        Self {
            status: asked.state,
            asked_at: asked.asked_at.to_string(),
        }
    }
}

/// The backups and the newest records.
#[derive(Serialize, ToSchema)]
pub(crate) struct BackupsBody {
    /// The backup files, newest first.
    files: Vec<BackupFileBody>,
    /// The newest records of backups created and restored, newest first.
    runs: Vec<BackupRunBody>,
    /// The backup asked for with *Create a backup now* that is not made yet; `null` when none is
    /// running and the last one asked for did not fail.
    #[schema(required = true)]
    asked: Option<AskedBackupBody>,
}

impl Masked for BackupsBody {}
impl Masked for BackupFileBody {}
impl Masked for AskedBackupBody {}

impl From<&Backups> for BackupsBody {
    fn from(backups: &Backups) -> Self {
        Self {
            files: backups.files.iter().map(BackupFileBody::from).collect(),
            runs: backups
                .runs
                .iter()
                .map(|run| BackupRunBody::of(run, &backups.people))
                .collect(),
            asked: backups.asked.as_ref().map(AskedBackupBody::from),
        }
    }
}

/// The backups and their records. Needs *Change organization settings*, from a session.
#[utoipa::path(
    get,
    path = "/api/v1/admin/backups",
    operation_id = "listBackups",
    tag = "admin",
    responses(
        (status = 200, description = "The backups.", body = BackupsBody),
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn list_backups(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let backups = auth
        .api()?
        .list_backups(info, ListBackupsQuery { session_token })
        .await?;
    answer_authorized(&backups.map(|backups| BackupsBody::from(&backups)))
}

/// Asks for a backup now, which is made in the background while people go on working; the answer
/// comes at once. The list of backups says while it runs (`asked`), and shows the backup when it is
/// made, or that it failed. Needs *Change organization settings*, from a session.
#[utoipa::path(
    post,
    path = "/api/v1/admin/backups",
    operation_id = "createBackup",
    tag = "admin",
    responses(
        (status = 202, description = "The backup is asked for and being made.", body = AskedBackupBody),
        (status = 409, description = "A backup is being created already (backup.running).", body = bb_core::Problem, content_type = "application/problem+json"),
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn create_backup(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let asked = auth
        .api()?
        .create_backup(info, CreateBackup { session_token })
        .await?;
    let mut answer = answer_authorized(&asked.map(|asked| AskedBackupBody::from(&asked)))?;
    *answer.status_mut() = StatusCode::ACCEPTED;
    Ok(answer)
}

/// The routes of backups.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(list_backups, create_backup))
}
