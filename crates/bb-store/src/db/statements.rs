//! The count of the statements a database has run, and in test builds their text.

use std::sync::atomic::{AtomicU64, Ordering};

use super::{Dialect, Statement};

/// The statements a database has run: how many, and in test builds also their text, so a test that
/// counts a window of statements can say which ones were in it (M5-F65).
#[derive(Debug, Default)]
pub(super) struct Statements {
    pub(super) count: AtomicU64,
    #[cfg(feature = "testing")]
    log: std::sync::Mutex<Vec<String>>,
}

impl Statements {
    #[cfg(not(feature = "testing"))]
    pub(super) fn record(&self, _statement: &Statement<'_>, _dialect: Dialect) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts the statement and keeps its text. The count moves under the lock of the log, so the
    /// log always has as many entries as the count says.
    #[cfg(feature = "testing")]
    pub(super) fn record(&self, statement: &Statement<'_>, dialect: Dialect) {
        let (sql, _) = statement.build(dialect);
        let mut log = self
            .log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        log.push(sql);
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    #[cfg(feature = "testing")]
    pub(super) fn after(&self, after: u64) -> Vec<String> {
        let log = self
            .log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let from = usize::try_from(after).unwrap_or(usize::MAX);
        log.iter().skip(from).cloned().collect()
    }
}
