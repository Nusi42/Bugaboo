//! From the events to the rows of the history (ISS-06, ADR-0007): which events belong in the
//! history of which issue. Pure: the consumer (`build.rs`) reads events and stores what this says.
//!
//! - An event about an **issue** is a change of that issue. When it puts the issue under another one
//!   (the member `parent`) or takes it out, the history of the parent gets a row as well, with the
//!   member `sub-issue`.
//! - An event about a **comment** is the row of a comment when it is the one that wrote the comment;
//!   later events about the comment (a new text, the trash) change the comment itself, which the
//!   history reads, and add no row.
//! - An event about a **relation** is a row in the history of both issues.
//!
//! What a reader may see of the changes in a row is decided when the history is read, with the
//! mask of the reader, never here.

use bb_core::event::{Actor, DomainEvent, EntityKind, FieldChange, RecordKind};
use bb_core::history::{HistoryEntry, HistoryKind};
use bb_core::id::IssueId;
use bb_core::technical::TechnicalName;

use crate::issues::member as issue_member;
use crate::member::{comment, relation};

/// The ID an event names in the member `field` (before or after), if it names an issue.
pub(crate) fn issue_in(changes: &[FieldChange], field: &str, after: bool) -> Option<IssueId> {
    let change = changes.iter().find(|change| change.field == field)?;
    let value = if after { &change.after } else { &change.before };
    value.as_ref()?.as_str()?.parse().ok()
}

/// The rows of the history that an event makes, at its place in the outbox. None for an event that
/// belongs in no history.
#[must_use]
pub(crate) fn rows_of(position: i64, event: &DomainEvent) -> Vec<HistoryEntry> {
    let kind_is = |kind: RecordKind| event.entity.kind == EntityKind::from(kind);
    let Ok(record_kind) = TechnicalName::new(event.entity.kind.as_str()) else {
        return Vec::new();
    };
    let row = |issue: IssueId, kind: HistoryKind, changes: Vec<FieldChange>| HistoryEntry {
        event: event.id,
        issue,
        position,
        at: event.time,
        by: match event.actor {
            Actor::Person(person) => Some(person),
            Actor::System => None,
        },
        kind,
        record: event.entity.id,
        record_kind: record_kind.clone(),
        changes,
    };
    if kind_is(RecordKind::Issue) {
        let Ok(issue) = IssueId::from_bytes(event.entity.id.to_bytes()) else {
            return Vec::new();
        };
        let mut rows = vec![row(issue, HistoryKind::Change, event.changes.clone())];
        // The parent: one row for the issue it left and one for the issue it joined.
        for (parent, joined) in [
            (issue_in(&event.changes, issue_member::PARENT, false), false),
            (issue_in(&event.changes, issue_member::PARENT, true), true),
        ] {
            if let Some(parent) = parent {
                let shown = Some(serde_json::json!(issue.to_string()));
                rows.push(row(
                    parent,
                    HistoryKind::Change,
                    vec![FieldChange {
                        field: issue_member::SUB_ISSUE.to_owned(),
                        before: if joined { None } else { shown.clone() },
                        after: if joined { shown } else { None },
                    }],
                ));
            }
        }
        return rows;
    }
    if kind_is(RecordKind::Comment) {
        // Only the event that wrote the comment: it did not count the version up.
        if event.counted_version() {
            return Vec::new();
        }
        return issue_in(&event.changes, comment::ISSUE, true)
            .map(|issue| vec![row(issue, HistoryKind::Comment, Vec::new())])
            .unwrap_or_default();
    }
    if kind_is(RecordKind::Relation) {
        let mut rows = Vec::new();
        for field in [relation::FROM, relation::TO] {
            let issue = issue_in(&event.changes, field, true)
                .or_else(|| issue_in(&event.changes, field, false));
            if let Some(issue) = issue {
                rows.push(row(issue, HistoryKind::Change, event.changes.clone()));
            }
        }
        return rows;
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use bb_core::event::{Channel, EntityRef};
    use bb_core::id::{EventId, SequentialIds, UserId};
    use serde_json::json;

    use super::*;

    fn event(kind: RecordKind, changes: Vec<FieldChange>) -> DomainEvent {
        let mut ids = SequentialIds::default();
        DomainEvent {
            id: EventId::generate(&mut ids).unwrap(),
            time: "2027-01-15T08:00:00Z".parse().unwrap(),
            actor: Actor::Person(UserId::generate(&mut ids)),
            channel: Channel::Ui,
            entity: EntityRef {
                kind: kind.into(),
                id: IssueId::generate(&mut ids).unwrap().into(),
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
    fn iss_06_an_event_about_an_issue_is_a_change_in_that_issue_only() {
        let made = event(RecordKind::Issue, vec![set("title", &json!("A"))]);
        let rows = rows_of(7, &made);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].position, 7);
        assert_eq!(rows[0].kind, HistoryKind::Change);
        assert_eq!(rows[0].record, made.entity.id);
    }

    #[test]
    fn iss_06_putting_an_issue_under_another_is_in_both_histories() {
        let mut ids = SequentialIds::default();
        let parent = IssueId::generate(&mut ids).unwrap();
        let moved = event(
            RecordKind::Issue,
            vec![set(issue_member::PARENT, &json!(parent.to_string()))],
        );
        let rows = rows_of(1, &moved);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].issue, parent);
        assert_eq!(rows[1].changes[0].field, issue_member::SUB_ISSUE);
        assert!(rows[1].changes[0].before.is_none() && rows[1].changes[0].after.is_some());
    }

    #[test]
    fn iss_06_only_the_event_that_wrote_a_comment_makes_its_row() {
        let mut ids = SequentialIds::default();
        let issue = IssueId::generate(&mut ids).unwrap();
        let written = event(
            RecordKind::Comment,
            vec![set(comment::ISSUE, &json!(issue.to_string()))],
        );
        let rows = rows_of(1, &written);
        assert_eq!(
            (rows.len(), rows[0].kind, rows[0].issue),
            (1, HistoryKind::Comment, issue)
        );
        let edited = event(
            RecordKind::Comment,
            vec![
                set(comment::ISSUE, &json!(issue.to_string())),
                bb_core::version::counted_up(1),
            ],
        );
        assert!(rows_of(2, &edited).is_empty());
    }

    #[test]
    fn iss_04_a_relation_is_in_the_history_of_both_issues_and_so_is_taking_it_away() {
        let mut ids = SequentialIds::default();
        let (from, to) = (
            IssueId::generate(&mut ids).unwrap(),
            IssueId::generate(&mut ids).unwrap(),
        );
        let added = event(
            RecordKind::Relation,
            vec![
                set(relation::FROM, &json!(from.to_string())),
                set(relation::TO, &json!(to.to_string())),
            ],
        );
        let issues: Vec<IssueId> = rows_of(1, &added).iter().map(|row| row.issue).collect();
        assert_eq!(issues, [from, to]);
        let removed = event(
            RecordKind::Relation,
            vec![
                FieldChange {
                    field: relation::FROM.to_owned(),
                    before: Some(json!(from.to_string())),
                    after: None,
                },
                FieldChange {
                    field: relation::TO.to_owned(),
                    before: Some(json!(to.to_string())),
                    after: None,
                },
            ],
        );
        let issues: Vec<IssueId> = rows_of(2, &removed).iter().map(|row| row.issue).collect();
        assert_eq!(issues, [from, to]);
    }

    #[test]
    fn iss_06_an_event_about_anything_else_is_in_no_history() {
        let other = event(RecordKind::Project, vec![set("name", &json!("A"))]);
        assert!(rows_of(1, &other).is_empty());
    }
}
