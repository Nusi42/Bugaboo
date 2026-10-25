//! From the events to the issues whose texts changed (ISS-13, ADR-0007). Pure: the consumer
//! (`build.rs`) reads events and hands the store what this says. The store makes the rows from the
//! texts as they are stored when it runs, not from the event, so an event read twice, or late,
//! changes nothing.

use bb_core::event::{DomainEvent, EntityKind, RecordKind};
use bb_core::id::{AttachmentId, CommentId, IssueId};

use crate::files::member as file_member;
use crate::history::issue_in;
use crate::member::comment;

/// What an event touches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Touched {
    /// An issue: its title, description, trash or own fields.
    Issue(IssueId),
    /// A comment whose issue the event does not name.
    Comment(CommentId),
    /// A file whose issue the event does not name.
    File(AttachmentId),
}

/// The record an event touches, if it is one whose text a search looks into.
#[must_use]
pub fn touched(event: &DomainEvent) -> Option<Touched> {
    let kind_is = |kind: RecordKind| event.entity.kind == EntityKind::from(kind);
    let bytes = event.entity.id.to_bytes();
    if kind_is(RecordKind::Issue) {
        return IssueId::from_bytes(bytes).ok().map(Touched::Issue);
    }
    if kind_is(RecordKind::Comment) {
        return issue_in(&event.changes, comment::ISSUE, true)
            .map(Touched::Issue)
            .or_else(|| CommentId::from_bytes(bytes).ok().map(Touched::Comment));
    }
    if kind_is(RecordKind::Attachment) {
        return issue_in(&event.changes, file_member::ISSUE, true)
            .map(Touched::Issue)
            .or_else(|| AttachmentId::from_bytes(bytes).ok().map(Touched::File));
    }
    None
}
