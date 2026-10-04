//! The outbox (ADR-0007): the list of events that consumers still have to read, and how far each
//! consumer has come.

use jiff::Timestamp;

use crate::id::EventId;
use crate::personal::Entity;
use crate::technical::TechnicalName;

/// One event waiting for its consumers. Written in the same transaction as the event.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct OutboxEntry {
    /// The place in the outbox, counting up.
    #[not_personal]
    pub position: i64,
    /// The event.
    #[not_personal]
    pub event_id: EventId,
    /// When the entry was written.
    #[not_personal]
    pub created_at: Timestamp,
}

/// How far a consumer has read (history, notifications, webhooks, search index, live updates).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct OutboxPosition {
    /// The consumer, like `notifications`.
    #[not_personal]
    pub consumer: TechnicalName,
    /// The last position it has finished.
    #[not_personal]
    pub position: i64,
    /// When it last moved on.
    #[not_personal]
    pub updated_at: Timestamp,
}
