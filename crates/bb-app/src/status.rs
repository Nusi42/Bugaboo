//! What the status page of the administrators shows (OPS-08), and the port that measures it.
//!
//! The use case is small: the HTTP layer asks a [`ServerProbe`] for one [`Snapshot`]. The probe is
//! an adapter in `bb-server`, because only the server knows its database, its certificate and the
//! security log together.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use jiff::Timestamp;

use crate::ports::BoxFuture;

/// Which database the server uses. The address is never part of it: it can hold a password.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunningDatabase {
    /// The database inside the data folder.
    BuiltIn,
    /// A SQLite file chosen by the person who runs the server.
    Sqlite,
    /// A PostgreSQL server.
    Postgres,
}

/// Why the chain of the security log does not verify. One value per reason the store knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogBreak {
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
    /// The anchor file knows of newer entries than the database has: the database was restored
    /// from an older backup, lost its last writes, or the end of the log was cut off.
    Behind,
}

/// How many security-log entries the server has written, and how many it could not write, since
/// it started (SEC-15, M1-F40). A failed write does not refuse the request (the decision of M1),
/// so this count is how the status page can tell that the log lags. The number of entries
/// written also tells the status page that the log has changed since it last looked.
#[derive(Debug, Default)]
pub struct LogWrites {
    written: AtomicU64,
    failed: AtomicU64,
}

impl LogWrites {
    /// Notes the result of one write.
    pub(crate) fn record(&self, written: bool) {
        let count = if written { &self.written } else { &self.failed };
        count.fetch_add(1, Ordering::Relaxed);
    }

    /// How many entries were written since the server started.
    #[must_use]
    pub fn written(&self) -> u64 {
        self.written.load(Ordering::Relaxed)
    }

    /// How many entries could not be written since the server started.
    #[must_use]
    pub fn failed(&self) -> u64 {
        self.failed.load(Ordering::Relaxed)
    }
}

/// What checking the security log found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogState {
    /// Every entry verifies.
    Valid {
        /// How many entries were checked.
        entries: u64,
        /// The place of the last entry and the start of its hash, if the log has entries.
        head: Option<(i64, String)>,
        /// Whether the anchor file vouches for the end of the log. Without it, nothing does.
        anchored: bool,
    },
    /// The chain breaks.
    Broken {
        /// The place of the first entry that does not verify.
        seq: i64,
        /// What is wrong with it.
        reason: LogBreak,
    },
    /// The log could not be read at all (the database or the anchor file failed).
    Unreadable,
}

/// Whether other accounts on this computer can open a part of the data folder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Access {
    /// Only the account that runs Bugaboo (and the administrators of the computer).
    #[default]
    Private,
    /// Other accounts can open it: too open.
    Open,
    /// The rights could not be read (the system tool failed, or its answer was not understood).
    /// It is never taken for private.
    Unknown,
}

impl Access {
    /// `Open` when `open` is true.
    #[must_use]
    pub const fn of(open: bool) -> Self {
        if open { Self::Open } else { Self::Private }
    }

    /// Whether it is too open.
    #[must_use]
    pub const fn is_open(self) -> bool {
        matches!(self, Self::Open)
    }

    /// The one of the two that needs more attention: open before unknown before private.
    #[must_use]
    pub const fn worse(self, other: Self) -> Self {
        match (self, other) {
            (Self::Open, _) | (_, Self::Open) => Self::Open,
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            _ => Self::Private,
        }
    }
}

/// Which parts of the data folder other accounts on this computer can open. Bugaboo only looks: a
/// folder that a person made or moved keeps its rights (SEC-16, M1-F65).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DataFolderRights {
    /// The data folder itself.
    pub folder: Access,
    /// The built-in database file and the files next to it.
    pub database: Access,
    /// The folder for attachments.
    pub files: Access,
    /// The folder for backups.
    pub backups: Access,
    /// The file with the setup code of a server without a console.
    pub setup_code_file: Access,
}

impl DataFolderRights {
    /// Nothing could be read: every part is unknown.
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            folder: Access::Unknown,
            database: Access::Unknown,
            files: Access::Unknown,
            backups: Access::Unknown,
            setup_code_file: Access::Unknown,
        }
    }

    /// Whether the rights of anything could not be read.
    #[must_use]
    pub const fn any_unknown(&self) -> bool {
        matches!(self.folder, Access::Unknown)
            || matches!(self.database, Access::Unknown)
            || matches!(self.files, Access::Unknown)
            || matches!(self.backups, Access::Unknown)
            || matches!(self.setup_code_file, Access::Unknown)
    }

    /// Whether anything is too open.
    #[must_use]
    pub const fn any(&self) -> bool {
        self.folder.is_open()
            || self.database.is_open()
            || self.files.is_open()
            || self.backups.is_open()
            || self.setup_code_file.is_open()
    }
}

/// The HTTPS certificate in use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CertificateState {
    /// When the certificate stops being valid.
    pub expires_at: Timestamp,
}

/// Everything the status page shows, measured at one moment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// The version of the program.
    pub version: String,
    /// When the server started.
    pub started_at: Timestamp,
    /// How long it has been running.
    pub uptime: Duration,
    /// Which database it uses.
    pub database: RunningDatabase,
    /// Whether the database answered.
    pub database_ok: bool,
    /// The certificate, when the server itself speaks HTTPS and the certificate could be read.
    pub certificate: Option<CertificateState>,
    /// The result of checking the security log.
    pub security_log: LogState,
    /// How many entries of the security log could not be written since the server started.
    /// Signing in goes on when a write fails (the decision of M1), so this is the only sign.
    pub log_write_failures: u64,
    /// Which parts of the data folder other accounts on this computer can open.
    pub data_folder: DataFolderRights,
}

/// Measures the server for the status page and for `/health/ready`.
pub trait ServerProbe: Send + Sync {
    /// Whether the database answers. Cheap: `/health/ready` calls it often.
    fn database_ok(&self) -> BoxFuture<'_, bool>;

    /// Everything the status page shows. The security log is checked from the end of the last
    /// good check, not from the start.
    fn snapshot(&self) -> BoxFuture<'_, Snapshot>;
}
