//! The adapter that measures the running server for the status page and `/health/ready`
//! (OPS-08, M1-F19, M1-F20). Only the server knows the database, the certificate and the
//! security log together, so this is where they meet.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bb_api::tls::ServedCertificate;
use bb_app::ports::BoxFuture;
use bb_app::status::{
    CertificateState, DataFolderRights, DatabaseKind, LogBreak, LogState, LogWrites, ServerProbe,
    Snapshot,
};
use bb_core::security_log::Digest;
use bb_store::{BreakReason, Database, LogSeal, Verification};
use jiff::Timestamp;
use tokio::sync::Mutex;

use crate::data_folder::DataFolder;

/// How many bytes of the hash of the last log entry the status page shows: enough to compare it
/// with a copy kept somewhere else, short enough to read out loud.
const HEAD_HASH_BYTES: usize = 8;

/// How the status page checks the security log (M1-F60). The command checks the whole log every
/// time; a page that people open often reads only what is new since its last good check.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LogChecking {
    /// How long an answer is reused when the server has written nothing to the log since.
    pub(crate) keep: Duration,
    /// How often the whole log is read again, so that an old entry that was changed is found
    /// within a day also by the page, not only by the command.
    pub(crate) full_every: Duration,
    /// How long the rights of the data folder are kept: looking at them can start a system tool
    /// for each part (Windows), which a page that people open often must not do every time.
    pub(crate) rights_keep: Duration,
}

impl Default for LogChecking {
    fn default() -> Self {
        Self {
            keep: Duration::from_secs(5),
            full_every: Duration::from_hours(24),
            rights_keep: Duration::from_mins(1),
        }
    }
}

/// The end of the log as the last good check left it.
struct Good {
    head: (i64, Digest),
    /// When the whole log was last read (not only the new entries).
    full_at: Instant,
}

/// The last answer, and what it was made under.
struct Last {
    state: LogState,
    at: Instant,
    /// How many entries the server had written when it was made: a write makes it old.
    written: u64,
}

/// What the status page remembers between its visits.
#[derive(Default)]
struct LogMemory {
    good: Option<Good>,
    last: Option<Last>,
}

/// Measures the server.
pub(crate) struct Probe {
    database: Database,
    seal: LogSeal,
    kind: DatabaseKind,
    /// The certificate in use when the server speaks HTTPS itself: the status page shows the end
    /// of the one it serves now, also after a renewal (M1-F03).
    certificate: Option<Arc<ServedCertificate>>,
    log_writes: Arc<LogWrites>,
    /// The data folder, whose rights the status page looks at every time.
    data_folder: PathBuf,
    log_checking: LogChecking,
    log_memory: Mutex<LogMemory>,
    rights_memory: Mutex<Option<(Instant, DataFolderRights)>>,
    started: Instant,
    started_at: Timestamp,
}

impl Probe {
    /// Starts counting uptime now.
    pub(crate) fn new(
        database: Database,
        seal: LogSeal,
        kind: DatabaseKind,
        certificate: Option<Arc<ServedCertificate>>,
        log_writes: Arc<LogWrites>,
        data_folder: PathBuf,
    ) -> Self {
        Self {
            database,
            seal,
            kind,
            certificate,
            log_writes,
            data_folder,
            log_checking: LogChecking::default(),
            log_memory: Mutex::new(LogMemory::default()),
            rights_memory: Mutex::new(None),
            started: Instant::now(),
            started_at: Timestamp::now(),
        }
    }

    /// The same probe with other times for reusing and for re-reading the log (the tests).
    #[cfg(test)]
    pub(crate) fn with_log_checking(mut self, checking: LogChecking) -> Self {
        self.log_checking = checking;
        self
    }

    /// What of the data folder other accounts can open. Looked at on a thread of its own (it reads
    /// files and may start a system tool) and kept for a short time. A look that fails is
    /// *unknown* for every part, never private.
    async fn check_rights(&self) -> DataFolderRights {
        let mut memory = self.rights_memory.lock().await;
        let now = Instant::now();
        if let Some((at, rights)) = *memory
            && now.duration_since(at) < self.log_checking.rights_keep
        {
            return rights;
        }
        let folder = self.data_folder.clone();
        let rights = tokio::task::spawn_blocking(move || {
            DataFolder::existing(&folder)
                .map_or_else(DataFolderRights::unknown, |data| data.rights())
        })
        .await
        .unwrap_or_else(|_| DataFolderRights::unknown());
        *memory = Some((now, rights));
        rights
    }

    /// Checks the security log as the status page does: from the end of the last good check, and
    /// only when the last answer is old or the server has written to the log since. A check that
    /// finds anything wrong is forgotten, so the next one reads everything again.
    async fn check_log(&self) -> LogState {
        let mut memory = self.log_memory.lock().await;
        let written = self.log_writes.written();
        let now = Instant::now();
        if let Some(last) = &memory.last
            && last.written == written
            && now.duration_since(last.at) < self.log_checking.keep
        {
            return last.state.clone();
        }
        let known = memory
            .good
            .as_ref()
            .filter(|good| now.duration_since(good.full_at) < self.log_checking.full_every);
        let continued = known.is_some();
        let full_at = known.map_or(now, |good| good.full_at);
        let result = self
            .database
            .verify_security_log_since(&self.seal, known.map(|good| good.head))
            .await;
        memory.good = match &result {
            Ok(Verification::Valid {
                head: Some(head), ..
            }) => Some(Good {
                head: *head,
                full_at: if continued { full_at } else { now },
            }),
            _ => None,
        };
        let state = log_state(&result);
        memory.last = Some(Last {
            state: state.clone(),
            at: now,
            written,
        });
        state
    }
}

/// The kind of database for a database address from the start-up settings (`None`: built in).
pub(crate) fn database_kind(url: Option<&str>) -> DatabaseKind {
    match url {
        None => DatabaseKind::BuiltIn,
        Some(url) if url.starts_with("sqlite:") => DatabaseKind::Sqlite,
        Some(_) => DatabaseKind::Postgres,
    }
}

/// The result of verifying the security log as the status page and the command line show it.
pub(crate) fn log_state(result: &Result<Verification, bb_store::StoreError>) -> LogState {
    match result {
        Ok(Verification::Valid {
            entries,
            head,
            anchored,
        }) => LogState::Valid {
            entries: *entries,
            head: head.as_ref().map(|(seq, hash)| {
                (
                    *seq,
                    bb_core::hex::encode(&hash.as_bytes()[..HEAD_HASH_BYTES]),
                )
            }),
            anchored: *anchored,
        },
        Ok(Verification::Broken { seq, reason }) => LogState::Broken {
            seq: *seq,
            reason: match reason {
                BreakReason::Gap => LogBreak::Gap,
                BreakReason::BrokenLink => LogBreak::BrokenLink,
                BreakReason::ChangedContent => LogBreak::ChangedContent,
                BreakReason::Checkpoint => LogBreak::Checkpoint,
                BreakReason::Truncated => LogBreak::Truncated,
                BreakReason::Anchor => LogBreak::Anchor,
                BreakReason::Behind => LogBreak::Behind,
            },
        },
        Err(_) => LogState::Unreadable,
    }
}

impl ServerProbe for Probe {
    fn database_ok(&self) -> BoxFuture<'_, bool> {
        Box::pin(async move { self.database.ping().await.is_ok() })
    }

    fn snapshot(&self) -> BoxFuture<'_, Snapshot> {
        Box::pin(async move {
            let database_ok = self.database.ping().await.is_ok();
            let security_log = self.check_log().await;
            let certificate = self
                .certificate
                .as_deref()
                .and_then(ServedCertificate::expires_at)
                .and_then(|seconds| Timestamp::from_second(seconds).ok())
                .map(|expires_at| CertificateState { expires_at });
            Snapshot {
                version: env!("CARGO_PKG_VERSION").to_owned(),
                started_at: self.started_at,
                uptime: self.started.elapsed(),
                database: self.kind,
                database_ok,
                certificate,
                security_log,
                log_write_failures: self.log_writes.failed(),
                data_folder: self.check_rights().await,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_08_the_kind_of_database_never_depends_on_the_address_text_after_the_scheme() {
        assert_eq!(database_kind(None), DatabaseKind::BuiltIn);
        assert_eq!(database_kind(Some("sqlite:x.db")), DatabaseKind::Sqlite);
        assert_eq!(
            database_kind(Some("postgres://user:secret@host/db")),
            DatabaseKind::Postgres
        );
        assert_eq!(
            database_kind(Some("postgresql://host/db")),
            DatabaseKind::Postgres
        );
    }

    #[test]
    fn ops_08_a_broken_log_is_reported_with_its_place_and_reason() {
        let broken = Ok(Verification::Broken {
            seq: 7,
            reason: BreakReason::Truncated,
        });
        assert_eq!(
            log_state(&broken),
            LogState::Broken {
                seq: 7,
                reason: LogBreak::Truncated
            }
        );
    }

    #[test]
    fn ops_08_a_valid_log_shows_the_start_of_its_last_hash() {
        let hash = Digest::new([0xAB; 32]);
        let valid = Ok(Verification::Valid {
            entries: 3,
            head: Some((3, hash)),
            anchored: false,
        });
        assert_eq!(
            log_state(&valid),
            LogState::Valid {
                entries: 3,
                head: Some((3, "abababababababab".to_owned())),
                anchored: false
            }
        );
    }
}
