//! The history of an issue (ISS-06): the read model that an outbox consumer builds from the domain
//! events. One row for each event that belongs in the history of an issue; an event that belongs in
//! two (a relation, a sub-issue) has a row for each issue. The row keeps the event's changes as
//! they were written: what a reader may see of them is decided when the history is read, with the
//! mask of the reader, never when it is built.

use jiff::Timestamp;

use crate::event::FieldChange;
use crate::id::{EntityId, EventId, IssueId, UserId};
use crate::personal::Entity;
use crate::technical::TechnicalName;

/// The name of the outbox consumer that builds the history (ADR-0007): its place in the outbox is
/// kept under this name.
pub const CONSUMER: &str = "history";

/// How many events the consumer reads in one go.
pub const BATCH: u64 = 200;

words! {
    /// What a row of the history is.
    pub enum HistoryKind {
        /// A comment was written; the row names the comment, and the text is read from it.
        Comment => "comment",
        /// Fields of the issue changed, or something about it (a relation, a sub-issue).
        Change => "change",
    }
}

words_in_api!(
    HistoryKind,
    "What a row of the history is: a comment or a change."
);

words! {
    /// The tabs of the history of an issue.
    pub enum HistoryTab {
        /// Comments and changes together.
        Everything => "everything",
        /// Only the comments.
        Comments => "comments",
        /// Only the changes.
        Changes => "changes",
    }
}

words_in_api!(
    HistoryTab,
    "Which rows of the history of an issue to list: everything, only the comments, or only the changes."
);

/// One row of the history of an issue.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct HistoryEntry {
    /// The event the row was made from: a row is made once for each event and issue.
    #[not_personal]
    pub event: EventId,
    /// The issue whose history it belongs to.
    #[not_personal]
    pub issue: IssueId,
    /// The place of the event in the outbox, which is the order of the history.
    #[not_personal]
    pub position: i64,
    /// When it happened.
    #[not_personal]
    pub at: Timestamp,
    /// Who did it; nobody for Bugaboo itself.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub by: Option<UserId>,
    /// What it is.
    #[not_personal]
    pub kind: HistoryKind,
    /// The record the event is about: for a comment its ID, for a relation the relation.
    #[not_personal]
    pub record: EntityId,
    /// The kind of record, as the event names it.
    #[not_personal]
    pub record_kind: TechnicalName,
    /// What changed. It can hold anything a person typed.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub changes: Vec<FieldChange>,
}
