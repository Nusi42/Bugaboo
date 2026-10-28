//! The rows of the history and the place of the reader that builds them (ISS-06, ADR-0007).

use bb_app::history::HistoryStore as _;
use bb_core::history::{CONSUMER, HistoryEntry, HistoryKind};
use bb_core::id::{EntityId, EventId, IssueId};
use bb_core::technical::TechnicalName;
use bb_testing::{TestResult, each_database};

use sea_query::Iden as _;

use crate::schema::OutboxPosition;
use crate::test_support::{at, fixture, ids};

/// Rows and the place move in one transaction (M5-F101): a reader that catches the history up takes
/// the write lock of the database once, not twice, so it waits once behind the people who write and
/// holds the others up once. A move of the place that fails therefore stores no row either.
#[tokio::test]
async fn iss_06_the_rows_of_the_history_and_the_place_of_the_reader_are_stored_together()
-> TestResult {
    each_database("iss_06_history_rows_with_place", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let issue = IssueId::generate(&mut source)?;
        let entry = HistoryEntry {
            event: EventId::generate(&mut source)?,
            issue,
            position: 1,
            at: at(1),
            by: None,
            kind: HistoryKind::Change,
            record: EntityId::from(issue),
            record_kind: TechnicalName::new("issue")?,
            changes: Vec::new(),
        };
        // The place of the reader is a table of its own; putting it away makes the move fail.
        let places = OutboxPosition::Table.to_string();
        let away = format!("{places}_away");
        storage
            .database()
            .rename_table_for_tests(&places, &away)
            .await?;
        let refused = storage
            .store_history(std::slice::from_ref(&entry), 1, at(2))
            .await;
        storage
            .database()
            .rename_table_for_tests(&away, &places)
            .await?;
        assert!(refused.is_err(), "the move of the place was broken");
        assert!(
            storage
                .history_page(issue, None, None, 0, 10)
                .await?
                .is_empty(),
            "the rows stay out when the place cannot move"
        );
        storage.store_history(&[entry], 1, at(3)).await?;
        assert_eq!(
            storage.history_page(issue, None, None, 0, 10).await?.len(),
            1
        );
        let consumer = TechnicalName::new(CONSUMER)?;
        assert_eq!(storage.database().outbox_position(&consumer).await?, 1);
        Ok(())
    })
    .await
}
