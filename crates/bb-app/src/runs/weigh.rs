//! The one evaluation of *Undo the run* (ADR-0029, decision 7): which fields of a run go back and
//! which stay. The preview and the undo both come here, so what the person looked at is what is
//! done.
//!
//! The rule: a field that any event outside the run changed after the run's first change to it is
//! left as it is and named, never written over.

use std::collections::BTreeMap;

use bb_core::event::{DomainEvent, EntityRef};
use bb_core::id::EventId;
use bb_core::run::{UndoSkip, UndoSkipKind, UndoVersion};
use serde_json::Value;

/// What a run did to one field of one record, from the value before its first change to the value
/// after its last.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Net {
    /// The record.
    pub(crate) entity: EntityRef,
    /// The field (the member of the event).
    pub(crate) field: String,
    /// Where the run's first change to this field stands in the order of commits.
    pub(crate) place: i64,
    /// The value before the run, none when the field had none.
    pub(crate) before: Option<Value>,
    /// The value after the run.
    pub(crate) after: Option<Value>,
}

/// The net changes of a run, built from its events in the order they were committed.
#[derive(Default)]
pub(crate) struct NetOfRun {
    fields: BTreeMap<(String, [u8; 16], String), Net>,
    first: Option<EventId>,
    last: Option<EventId>,
    events: u64,
}

impl NetOfRun {
    /// Adds the next event of the run.
    pub(crate) fn add(&mut self, place: i64, event: &DomainEvent) {
        self.first.get_or_insert(event.id);
        self.last = Some(event.id);
        self.events += 1;
        for change in &event.changes {
            // The count of the version goes up with every change; it is not something to put back.
            if change.field == bb_core::version::FIELD {
                continue;
            }
            let key = (
                event.entity.kind.as_str().to_owned(),
                event.entity.id.to_bytes(),
                change.field.clone(),
            );
            self.fields
                .entry(key)
                .and_modify(|net| net.after.clone_from(&change.after))
                .or_insert_with(|| Net {
                    entity: event.entity.clone(),
                    field: change.field.clone(),
                    place,
                    before: change.before.clone(),
                    after: change.after.clone(),
                });
        }
    }

    /// The first event of the run, the place the events outside it are looked for after.
    pub(crate) const fn first_event(&self) -> Option<EventId> {
        self.first
    }

    /// The changes that left a field different from how the run found it.
    pub(crate) fn changed(&self) -> impl Iterator<Item = &Net> {
        self.fields.values().filter(|net| net.before != net.after)
    }

    /// The records the run changed, by kind.
    pub(crate) fn records(&self) -> BTreeMap<String, Vec<bb_core::id::EntityId>> {
        let mut by_kind: BTreeMap<String, Vec<bb_core::id::EntityId>> = BTreeMap::new();
        for net in self.changed() {
            let ids = by_kind
                .entry(net.entity.kind.as_str().to_owned())
                .or_default();
            if !ids.contains(&net.entity.id) {
                ids.push(net.entity.id);
            }
        }
        by_kind
    }
}

/// What *Undo the run* would do: the fields that go back, the fields that stay, and the version of
/// this answer.
pub(crate) struct Evaluation {
    /// The fields that go back, in no particular order.
    pub(crate) back: Vec<Net>,
    /// The fields that stay, with why.
    pub(crate) skips: Vec<UndoSkip>,
    /// Differs as soon as the run or anything after it changed.
    pub(crate) version: UndoVersion,
}

/// Decides each field of the run against the events outside the run that came after (`later`, with
/// their places in the order of commits).
pub(crate) fn weigh(net: &NetOfRun, later: &[(i64, DomainEvent)]) -> Evaluation {
    let mut back = Vec::new();
    let mut skips = Vec::new();
    for each in net.changed() {
        let changed_since = later
            .iter()
            .filter(|(place, event)| {
                *place > each.place
                    && event.entity == each.entity
                    && event
                        .changes
                        .iter()
                        .any(|change| change.field == each.field)
            })
            .map(|(_, event)| event)
            .next_back();
        match changed_since {
            Some(event) => skips.push(UndoSkip {
                entity: each.entity.clone(),
                field: each.field.clone(),
                kind: UndoSkipKind::ChangedSince,
                by: Some(event.actor),
                at: Some(event.time),
            }),
            None => back.push(each.clone()),
        }
    }
    let last_later = later
        .iter()
        .map(|(_, event)| event.id.to_string())
        .next_back()
        .unwrap_or_default();
    let version = UndoVersion(format!(
        "{}:{}:{}:{}",
        net.last.map(|id| id.to_string()).unwrap_or_default(),
        net.events,
        later.len(),
        last_later
    ));
    Evaluation {
        back,
        skips,
        version,
    }
}

#[cfg(test)]
mod tests {
    use bb_core::event::{Actor, Channel, EntityKind, FieldChange};
    use bb_core::id::{SequentialIds, UserId};
    use serde_json::json;

    use super::*;

    fn event(
        ids: &mut SequentialIds,
        entity: EntityRef,
        field: &str,
        before: Value,
        after: Value,
        run: bool,
    ) -> DomainEvent {
        DomainEvent {
            id: EventId::generate(ids).unwrap(),
            time: jiff::Timestamp::UNIX_EPOCH,
            actor: Actor::Person(UserId::generate(ids)),
            channel: Channel::Api,
            entity,
            changes: vec![FieldChange {
                field: field.to_owned(),
                before: Some(before),
                after: Some(after),
            }],
            reason: None,
            run: run.then(|| bb_core::id::RunId::generate(ids).unwrap()),
        }
    }

    fn issue(ids: &mut SequentialIds) -> EntityRef {
        EntityRef {
            kind: EntityKind::new("issue").unwrap(),
            id: UserId::generate(ids).into(),
        }
    }

    #[test]
    fn a_field_changed_twice_goes_back_to_how_the_run_found_it() {
        let mut ids = SequentialIds::default();
        let record = issue(&mut ids);
        let mut net = NetOfRun::default();
        net.add(
            1,
            &event(
                &mut ids,
                record.clone(),
                "status",
                json!("a"),
                json!("b"),
                true,
            ),
        );
        net.add(
            2,
            &event(&mut ids, record, "status", json!("b"), json!("c"), true),
        );
        let result = weigh(&net, &[]);
        assert_eq!(result.back.len(), 1);
        assert_eq!(result.back[0].before, Some(json!("a")));
        assert_eq!(result.back[0].after, Some(json!("c")));
    }

    #[test]
    fn a_field_changed_back_by_the_run_itself_has_nothing_to_undo() {
        let mut ids = SequentialIds::default();
        let record = issue(&mut ids);
        let mut net = NetOfRun::default();
        net.add(
            1,
            &event(
                &mut ids,
                record.clone(),
                "status",
                json!("a"),
                json!("b"),
                true,
            ),
        );
        net.add(
            2,
            &event(&mut ids, record, "status", json!("b"), json!("a"), true),
        );
        assert!(weigh(&net, &[]).back.is_empty());
    }

    #[test]
    fn ai_38_a_field_somebody_changed_afterwards_stays_and_is_named() {
        let mut ids = SequentialIds::default();
        let record = issue(&mut ids);
        let mut net = NetOfRun::default();
        net.add(
            1,
            &event(
                &mut ids,
                record.clone(),
                "status",
                json!("a"),
                json!("b"),
                true,
            ),
        );
        net.add(
            1,
            &event(
                &mut ids,
                record.clone(),
                "priority",
                json!("low"),
                json!("high"),
                true,
            ),
        );
        let later = [
            (
                5,
                event(
                    &mut ids,
                    record.clone(),
                    "status",
                    json!("b"),
                    json!("c"),
                    false,
                ),
            ),
            // Before the run's change: not a change behind its back.
            (
                0,
                event(
                    &mut ids,
                    record,
                    "priority",
                    json!("x"),
                    json!("low"),
                    false,
                ),
            ),
        ];
        let result = weigh(&net, &later);
        assert_eq!(result.back.len(), 1);
        assert_eq!(result.back[0].field, "priority");
        assert_eq!(result.skips.len(), 1);
        assert_eq!(result.skips[0].field, "status");
        assert_eq!(result.skips[0].kind, UndoSkipKind::ChangedSince);
        // The version changes when anything came after.
        assert_ne!(result.version, weigh(&net, &[]).version);
    }
}
