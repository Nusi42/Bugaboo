//! Backups (OPS-07, ADR-0046 decision 3): the list of the backups with their records, and *Create a
//! backup now*. Both need *Change organization settings* and a session; an access key or an agent
//! is turned away. A backup is restored from the command line of the server, with the server
//! stopped (`bugaboo restore <file>`): there is no route that restores.

use axum::extract::State;
use axum::response::Response;
use bb_app::backups::{BackupItem, Backups, CreateBackup, ListBackupsQuery};
use bb_core::backup::name::Purpose;
use bb_core::backup::run::BackupRun;
use bb_core::hex;
use bb_policy::Masked;
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
    /// The person who pressed *Create a backup now*.
    person: Option<String>,
    /// When it happened.
    at: String,
    /// For a restore: the first entry of the security log that is gone.
    lost_from: Option<i64>,
    /// For a restore: the last entry of the security log that is gone.
    lost_to: Option<i64>,
}

impl From<&BackupRun> for BackupRunBody {
    fn from(run: &BackupRun) -> Self {
        Self {
            event: run.event.code().to_owned(),
            name: run.name.to_string(),
            size: run.size,
            checksum: hex::encode(run.checksum.as_bytes()),
            maker: run.maker.code().to_owned(),
            person: run.person.map(|person| person.to_string()),
            at: run.at.to_string(),
            lost_from: run.lost_from,
            lost_to: run.lost_to,
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
}

impl Masked for BackupsBody {}
impl Masked for BackupFileBody {}

impl From<&Backups> for BackupsBody {
    fn from(backups: &Backups) -> Self {
        Self {
            files: backups.files.iter().map(BackupFileBody::from).collect(),
            runs: backups.runs.iter().map(BackupRunBody::from).collect(),
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

/// Creates a backup now, while people go on working. Needs *Change organization settings*, from a
/// session.
#[utoipa::path(
    post,
    path = "/api/v1/admin/backups",
    operation_id = "createBackup",
    tag = "admin",
    responses(
        (status = 200, description = "The backup that was created.", body = BackupFileBody),
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
    let created = auth
        .api()?
        .create_backup(info, CreateBackup { session_token })
        .await?;
    answer_authorized(&created.map(|item| BackupFileBody::from(&item)))
}

/// The routes of backups.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(list_backups, create_backup))
}
