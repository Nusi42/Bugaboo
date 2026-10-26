//! From the events to the issues whose texts changed (ISS-13, ADR-0007). Pure: the consumer
//! (`build.rs`) reads events and hands the store what this says. The store makes the rows from the
//! texts as they are stored when it runs, not from the event, so an event read twice, or late,
//! changes nothing.

use bb_core::event::{DomainEvent, EntityKind, RecordKind};
use bb_core::id::{AttachmentId, CommentId, IssueId};

use crate::files::member as file_member;
use crate::history::issue_in;
use crate::member::comment as comment_member;

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
        return issue_in(&event.changes, comment_member::ISSUE, true)
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

#[cfg(test)]
mod tests {
    use bb_core::event::{Actor, Channel, EntityRef, FieldChange};
    use bb_core::id::{EntityId, EventId, SequentialIds, UserId};
    use jiff::Timestamp;
    use serde_json::json;

    use super::*;

    /// An event about a record of this kind, with these changes: what a use case writes.
    fn event_about(kind: RecordKind, id: EntityId, changes: Vec<FieldChange>) -> DomainEvent {
        let mut source = SequentialIds::default();
        DomainEvent {
            id: EventId::generate(&mut source).unwrap(),
            time: Timestamp::UNIX_EPOCH,
            actor: Actor::Person(UserId::generate(&mut source)),
            channel: Channel::Api,
            entity: EntityRef {
                kind: EntityKind::from(kind),
                id,
            },
            changes,
            reason: None,
        }
    }

    fn set(field: &str, value: &serde_json::Value) -> FieldChange {
        FieldChange {
            field: field.to_owned(),
            before: None,
            after: Some(value.clone()),
        }
    }

    #[test]
    fn iss_13_an_event_about_an_issue_touches_that_issue() {
        let mut source = SequentialIds::default();
        let issue = IssueId::generate(&mut source).unwrap();
        let event = event_about(RecordKind::Issue, issue.into(), Vec::new());
        assert_eq!(touched(&event), Some(Touched::Issue(issue)));
    }

    #[test]
    fn iss_13_a_new_comment_names_its_issue_and_an_edit_names_the_comment() {
        let mut source = SequentialIds::default();
        let (issue, comment) = (
            IssueId::generate(&mut source).unwrap(),
            CommentId::generate(&mut source).unwrap(),
        );
        let written = event_about(
            RecordKind::Comment,
            comment.into(),
            vec![set(comment_member::ISSUE, &json!(issue.to_string()))],
        );
        assert_eq!(touched(&written), Some(Touched::Issue(issue)));
        // An edit and the trash name the comment only: the store finds its issue.
        let edited = event_about(
            RecordKind::Comment,
            comment.into(),
            vec![set("text", &json!("new words"))],
        );
        assert_eq!(touched(&edited), Some(Touched::Comment(comment)));
    }

    #[test]
    fn iss_13_a_file_names_its_issue_when_the_event_does_and_itself_when_it_does_not() {
        let mut source = SequentialIds::default();
        let (issue, file) = (
            IssueId::generate(&mut source).unwrap(),
            AttachmentId::generate(&mut source).unwrap(),
        );
        let with_issue = event_about(
            RecordKind::Attachment,
            file.into(),
            vec![set(file_member::ISSUE, &json!(issue.to_string()))],
        );
        assert_eq!(touched(&with_issue), Some(Touched::Issue(issue)));
        let alone = event_about(RecordKind::Attachment, file.into(), Vec::new());
        assert_eq!(touched(&alone), Some(Touched::File(file)));
    }

    #[test]
    fn iss_13_an_event_about_anything_else_touches_nothing() {
        let mut source = SequentialIds::default();
        let project = EntityId::from(IssueId::generate(&mut source).unwrap());
        assert_eq!(
            touched(&event_about(RecordKind::Project, project, Vec::new())),
            None
        );
    }
}
