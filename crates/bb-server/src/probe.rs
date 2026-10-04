//! The adapter that measures the running server for the status page and `/health/ready`
//! (OPS-08, M1-F19, M1-F20). Only the server knows the database, the certificate and the
//! security log together, so this is where they meet.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::Instant;

use bb_api::tls::certificate_end;
use bb_app::ports::BoxFuture;
use bb_app::status::{CertificateState, DatabaseKind, LogBreak, LogState, ServerProbe, Snapshot};
use bb_store::{BreakReason, Database, LogSeal, Verification};
use jiff::Timestamp;

/// How many bytes of the hash of the last log entry the status page shows: enough to compare it
/// with a copy kept somewhere else, short enough to read out loud.
const HEAD_HASH_BYTES: usize = 8;

/// Measures the server.
pub(crate) struct Probe {
    database: Database,
    seal: LogSeal,
    kind: DatabaseKind,
    /// The certificate file when the server speaks HTTPS itself.
    certificate: Option<PathBuf>,
    started: Instant,
    started_at: Timestamp,
}

impl Probe {
    /// Starts counting uptime now.
    pub(crate) fn new(
        database: Database,
        seal: LogSeal,
        kind: DatabaseKind,
        certificate: Option<PathBuf>,
    ) -> Self {
        Self {
            database,
            seal,
            kind,
            certificate,
            started: Instant::now(),
            started_at: Timestamp::now(),
        }
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
                let mut text = String::new();
                for byte in &hash.as_bytes()[..HEAD_HASH_BYTES] {
                    // Writing to a String cannot fail.
                    let _ = write!(text, "{byte:02x}");
                }
                (*seq, text)
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
            let security_log = log_state(&self.database.verify_security_log(&self.seal).await);
            let certificate = self
                .certificate
                .as_deref()
                .and_then(certificate_end)
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
        let hash = bb_core::security_log::Digest::new([0xAB; 32]);
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
