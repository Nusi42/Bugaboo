//! What the status page of the administrators shows (OPS-08), and the port that measures it.
//!
//! The use case is small: the HTTP layer asks a [`ServerProbe`] for one [`Snapshot`]. The probe is
//! an adapter in `bb-server`, because only the server knows its database, its certificate and the
//! security log together.

use std::time::Duration;

use jiff::Timestamp;

use crate::ports::BoxFuture;

/// Which database the server uses. The address is never part of it: it can hold a password.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabaseKind {
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
    pub database: DatabaseKind,
    /// Whether the database answered.
    pub database_ok: bool,
    /// The certificate, when the server itself speaks HTTPS and the certificate could be read.
    pub certificate: Option<CertificateState>,
    /// The result of checking the security log.
    pub security_log: LogState,
}

/// Measures the server for the status page and for `/health/ready`.
pub trait ServerProbe: Send + Sync {
    /// Whether the database answers. Cheap: `/health/ready` calls it often.
    fn database_ok(&self) -> BoxFuture<'_, bool>;

    /// Everything the status page shows. This checks the whole security log, so it is not for
    /// frequent calls.
    fn snapshot(&self) -> BoxFuture<'_, Snapshot>;
}
