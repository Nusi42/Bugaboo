//! What only people with the permission see: the status page of the server (OPS-08).

use std::sync::Arc;

use axum::extract::State;
use axum::response::Response;
use bb_app::auth::SeeServerStatusQuery;
use bb_app::status::{
    Access, DataFolderRights, LogBreak, LogState, RunningDatabase, ServerProbe, Snapshot,
};
use bb_core::ErrorCode;
use bb_policy::Masked;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, NotSignedInAnswer, Session, answer_authorized,
};
use crate::errors::ApiError;

/// How the server measures itself, if it can. Without a probe (tests of other parts of the API)
/// the status route answers that the server cannot answer right now.
#[derive(Clone)]
pub struct ProbeHandle(pub(crate) Option<Arc<dyn ServerProbe>>);

impl std::fmt::Debug for ProbeHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProbeHandle")
            .field("ready", &self.0.is_some())
            .finish()
    }
}

impl ProbeHandle {
    /// The probe the status page and `/health/ready` ask.
    #[must_use]
    pub fn new(probe: Arc<dyn ServerProbe>) -> Self {
        Self(Some(probe))
    }

    /// No probe: the status route answers `server.unavailable`.
    #[must_use]
    pub const fn none() -> Self {
        Self(None)
    }

    pub(crate) fn probe(&self) -> Option<&dyn ServerProbe> {
        self.0.as_deref()
    }
}

/// Which database the server uses.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
enum DatabaseKind {
    /// The database inside the data folder.
    BuiltIn,
    /// A SQLite file.
    Sqlite,
    /// A PostgreSQL server.
    Postgres,
}

impl From<RunningDatabase> for DatabaseKind {
    fn from(kind: RunningDatabase) -> Self {
        match kind {
            RunningDatabase::BuiltIn => Self::BuiltIn,
            RunningDatabase::Sqlite => Self::Sqlite,
            RunningDatabase::Postgres => Self::Postgres,
        }
    }
}

/// The database.
#[derive(Serialize, ToSchema)]
struct DatabaseBody {
    /// Which one it is. The address is never shown: it can hold a password.
    kind: DatabaseKind,
    /// Whether it answered.
    ok: bool,
}

/// The HTTPS certificate the server uses itself.
#[derive(Serialize, ToSchema)]
struct CertificateBody {
    /// When it stops being valid.
    expires_at: String,
}

/// Why the chain of the security log does not verify.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
enum LogBreakReason {
    /// An entry is missing, twice or out of order.
    Gap,
    /// An entry does not point to the one before it.
    BrokenLink,
    /// An entry was changed after it was written.
    ChangedContent,
    /// Entries were deleted without a signed checkpoint, or the checkpoint is wrong.
    Checkpoint,
    /// The end of the log was cut off or rewritten.
    Truncated,
    /// The file that vouches for the end of the log was changed.
    Anchor,
    /// The anchor file knows of newer entries than the database has.
    Behind,
}

impl From<LogBreak> for LogBreakReason {
    fn from(reason: LogBreak) -> Self {
        match reason {
            LogBreak::Gap => Self::Gap,
            LogBreak::BrokenLink => Self::BrokenLink,
            LogBreak::ChangedContent => Self::ChangedContent,
            LogBreak::Checkpoint => Self::Checkpoint,
            LogBreak::Truncated => Self::Truncated,
            LogBreak::Anchor => Self::Anchor,
            LogBreak::Behind => Self::Behind,
        }
    }
}

/// The result of checking the security log.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
enum LogResult {
    /// Every entry verifies.
    Valid,
    /// The chain breaks: see `broken_seq` and `reason`.
    Broken,
    /// The log could not be read.
    Unreadable,
}

/// What checking the security log found.
#[derive(Serialize, ToSchema)]
struct SecurityLogBody {
    /// The outcome.
    result: LogResult,
    /// How many entries were checked (when it verifies).
    entries: Option<u64>,
    /// The place of the last entry (when it verifies and has entries).
    head_seq: Option<i64>,
    /// The start of the hash of the last entry, to compare with a copy kept elsewhere.
    head_hash: Option<String>,
    /// Whether the file that vouches for the end of the log exists. A log with entries and no
    /// anchor is a warning: nothing then shows that the end was not cut off.
    anchored: Option<bool>,
    /// The place of the first entry that does not verify (when it breaks).
    broken_seq: Option<i64>,
    /// What is wrong with it (when it breaks).
    reason: Option<LogBreakReason>,
    /// How many entries could not be written since the server started. A failed write does not
    /// stop a sign-in, so a number above zero means the log lags: check the disk and the rights
    /// on the data folder.
    write_failures: u64,
}

impl From<LogState> for SecurityLogBody {
    fn from(state: LogState) -> Self {
        let empty = Self {
            result: LogResult::Unreadable,
            entries: None,
            head_seq: None,
            head_hash: None,
            anchored: None,
            broken_seq: None,
            reason: None,
            write_failures: 0,
        };
        match state {
            LogState::Valid {
                entries,
                head,
                anchored,
            } => Self {
                result: LogResult::Valid,
                entries: Some(entries),
                head_seq: head.as_ref().map(|(seq, _)| *seq),
                head_hash: head.map(|(_, hash)| hash),
                anchored: Some(anchored),
                ..empty
            },
            LogState::Broken { seq, reason } => Self {
                result: LogResult::Broken,
                broken_seq: Some(seq),
                reason: Some(reason.into()),
                ..empty
            },
            LogState::Unreadable => empty,
        }
    }
}

/// Whether other accounts on this computer can open a part of the data folder.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
enum FolderAccess {
    /// Only the account that runs Bugaboo can open it.
    Private,
    /// Other accounts can open it: too open.
    Open,
    /// The rights could not be read, so nothing can be said.
    Unknown,
}

impl From<Access> for FolderAccess {
    fn from(access: Access) -> Self {
        match access {
            Access::Private => Self::Private,
            Access::Open => Self::Open,
            Access::Unknown => Self::Unknown,
        }
    }
}

/// Which parts of the data folder other accounts on this computer can open. Bugaboo only looks;
/// a folder that somebody made or moved keeps its rights.
#[derive(Serialize, ToSchema)]
struct DataFolderOpenBody {
    /// The data folder itself.
    folder: FolderAccess,
    /// The built-in database file and the files next to it.
    database: FolderAccess,
    /// The folder for attachments.
    files: FolderAccess,
    /// The folder for backups.
    backups: FolderAccess,
    /// The file with the setup code of a server without a console.
    setup_code_file: FolderAccess,
}

impl From<DataFolderRights> for DataFolderOpenBody {
    fn from(rights: DataFolderRights) -> Self {
        Self {
            folder: rights.folder.into(),
            database: rights.database.into(),
            files: rights.files.into(),
            backups: rights.backups.into(),
            setup_code_file: rights.setup_code_file.into(),
        }
    }
}

impl Masked for ServerStatusBody {}

/// The status of the server, as the status page shows it.
#[derive(Serialize, ToSchema)]
struct ServerStatusBody {
    /// The version of the program.
    version: String,
    /// When the server started.
    started_at: String,
    /// How long it has been running, in seconds.
    uptime_seconds: u64,
    /// The database.
    database: DatabaseBody,
    /// The HTTPS certificate; missing when the server does not speak HTTPS itself (plain HTTP
    /// behind a proxy) or the certificate cannot be read.
    certificate: Option<CertificateBody>,
    /// The check of the security log.
    security_log: SecurityLogBody,
    /// What of the data folder other accounts on this computer can open.
    data_folder_open: DataFolderOpenBody,
}

impl From<Snapshot> for ServerStatusBody {
    fn from(snapshot: Snapshot) -> Self {
        Self {
            version: snapshot.version,
            started_at: snapshot.started_at.to_string(),
            uptime_seconds: snapshot.uptime.as_secs(),
            database: DatabaseBody {
                kind: snapshot.database.into(),
                ok: snapshot.database_ok,
            },
            certificate: snapshot.certificate.map(|certificate| CertificateBody {
                expires_at: certificate.expires_at.to_string(),
            }),
            security_log: SecurityLogBody {
                write_failures: snapshot.log_write_failures,
                ..snapshot.security_log.into()
            },
            data_folder_open: snapshot.data_folder.into(),
        }
    }
}

/// How the server is doing: version, uptime, database, certificate and the check of the security
/// log. Needs the permission *See how the server is doing*.
#[utoipa::path(
    get,
    path = "/api/v1/admin/status",
    operation_id = "getServerStatus",
    tag = "admin",
    responses((status = 200, description = "The status of the server.", body = ServerStatusBody), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn get_server_status(
    State(auth): State<AuthHandle>,
    State(probe): State<ProbeHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    // A person or an access key that may see the status: a key counts with what it keeps.
    let allowed = auth
        .api()?
        .see_server_status(info, SeeServerStatusQuery { session_token })
        .await?;
    let probe = probe
        .probe()
        .ok_or_else(|| ApiError::new(ErrorCode::Unavailable))?;
    let snapshot = probe.snapshot().await;
    answer_authorized(&allowed.map(|_| ServerStatusBody::from(snapshot)))
}

/// The routes only administrators use.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
    ProbeHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(get_server_status))
}
