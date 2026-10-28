//! The count of the statements a database has run, and in test builds their text while a capture
//! is on.

use std::sync::atomic::{AtomicU64, Ordering};

use super::{Dialect, Statement};

/// The statements a database has run: how many, and in test builds also their text once a test has
/// asked for it (`start_capture`), so a test that counts a window of statements can say which ones
/// were in it (M5-F65). Without a capture nothing is kept, so a long test run does not grow it.
#[derive(Debug, Default)]
pub(super) struct Statements {
    count: AtomicU64,
    /// The text of the statements since the last `take`; `None` while no capture is on.
    #[cfg(feature = "testing")]
    captured: std::sync::Mutex<Option<Vec<String>>>,
}

impl Statements {
    pub(super) fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    #[cfg(not(feature = "testing"))]
    pub(super) fn record(&self, _statement: &Statement<'_>, _dialect: Dialect) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts the statement and, while a capture is on, keeps its text. The count moves under the
    /// lock of the capture, so `take` sees a count and a text that belong together.
    #[cfg(feature = "testing")]
    pub(super) fn record(&self, statement: &Statement<'_>, dialect: Dialect) {
        let mut captured = self.lock();
        if let Some(texts) = captured.as_mut() {
            texts.push(statement.build(dialect).0);
        }
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    #[cfg(feature = "testing")]
    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Vec<String>>> {
        self.captured
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Starts keeping the text of the statements (and forgets what was kept).
    #[cfg(feature = "testing")]
    pub(super) fn start_capture(&self) {
        *self.lock() = Some(Vec::new());
    }

    /// The count now and the text of the statements since the capture started or the last `take`;
    /// the text is emptied.
    #[cfg(feature = "testing")]
    pub(super) fn take(&self) -> (u64, Vec<String>) {
        let mut captured = self.lock();
        let texts = captured.as_mut().map(std::mem::take).unwrap_or_default();
        (self.count(), texts)
    }
}

#[cfg(all(test, feature = "testing"))]
mod tests {
    use sea_query::SelectStatement;

    use super::*;

    fn select() -> SelectStatement {
        let mut select = SelectStatement::new();
        select.expr(sea_query::Expr::val(1_i64));
        select
    }

    #[test]
    fn perf_05_statements_are_counted_but_their_text_is_kept_only_during_a_capture() {
        let statements = Statements::default();
        let query = select();
        statements.record(&Statement::from(&query), Dialect::Sqlite);
        assert_eq!(statements.take(), (1, Vec::new()), "no capture, no text");
        statements.start_capture();
        statements.record(&Statement::from(&query), Dialect::Sqlite);
        statements.record(&Statement::from(&query), Dialect::Postgres);
        let (count, texts) = statements.take();
        assert_eq!(count, 3);
        assert_eq!(texts.len(), 2, "one text for each statement of the capture");
        assert!(texts[0].contains("SELECT"));
        assert_eq!(
            statements.take(),
            (3, Vec::new()),
            "a take empties what was kept"
        );
    }
}
