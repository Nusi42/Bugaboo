//! The adapter of the port `bb_app::runs::RunRecordStore`: the events of a run, and the events that
//! changed its records behind its back.

use bb_app::runs::RunRecordStore;
use bb_core::event::{DomainEvent, EntityKind};
use bb_core::id::{EntityId, EventId, RunId};
use bb_core::mark::TextMark;

use crate::auth::AuthStorage;
use crate::error::StoreError;

impl RunRecordStore for AuthStorage {
    async fn run_events(
        &self,
        run: RunId,
        after: Option<EventId>,
        limit: u64,
    ) -> Result<Vec<(i64, DomainEvent)>, StoreError> {
        self.database().run_events(run, after, limit).await
    }

    async fn events_outside_run_after(
        &self,
        run: RunId,
        since: EventId,
        kind: &EntityKind,
        ids: &[EntityId],
    ) -> Result<Vec<(i64, DomainEvent)>, StoreError> {
        self.database()
            .events_outside_run_after(run, since, kind, ids)
            .await
    }

    async fn text_marks(
        &self,
        kind: &EntityKind,
        ids: &[EntityId],
    ) -> Result<Vec<TextMark>, StoreError> {
        self.database().text_marks_of(kind, ids).await
    }
}
