//! What became of the changes of a run once it was undone (AI-38): the run says who undid it and
//! when, and each field says whether it went back or stayed, and who changed it afterwards.

use bb_core::event::{DomainEvent, EntityKind, EntityRef, RecordKind};
use bb_core::id::{EntityId, RunId};
use bb_core::people_kinds::PeopleKind;
use bb_core::run::UndoSkipKind;
use bb_policy::{Evaluation, PeopleContext, PersonRef};
use jiff::Timestamp;
use serde::Serialize;

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// Who undid a run, and when.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UndoneView {
    /// The person, as the kind *Authors* says for the reader.
    pub by: PersonRef,
    /// When.
    pub at: Timestamp,
}

/// What became of one field of an undone run.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum FieldOutcome {
    /// The old value is back.
    WentBack,
    /// The field stayed as it is.
    Kept {
        /// Why.
        kind: UndoSkipKind,
        /// Who changed it afterwards, when somebody did.
        by: Option<PersonRef>,
        /// When.
        at: Option<Timestamp>,
    },
}

/// The events after the run that touched one field of one record of it, oldest first.
fn after_the_run<'a>(
    later: &'a [(i64, DomainEvent)],
    place: i64,
    entity: &'a EntityRef,
    field: &'a str,
) -> impl Iterator<Item = &'a DomainEvent> {
    later
        .iter()
        .filter(move |(at, event)| {
            *at > place
                && event.entity == *entity
                && event.changes.iter().any(|change| change.field == field)
        })
        .map(|(_, event)| event)
}

/// The name an undo gives to the fact that a field stayed, on the event it writes on the run: with
/// the reason as its value, so the page can say why long after.
pub(super) fn kept_key(entity: &EntityRef, field: &str) -> String {
    format!("kept/{}/{field}", entity.id)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The events about these records that came after the run, outside it.
    pub(super) async fn events_after_run(
        &self,
        run: RunId,
        kind: &EntityKind,
        ids: &[EntityId],
    ) -> Result<Vec<(i64, DomainEvent)>, AppError> {
        let first = self
            .store()
            .run_events(run, None, 1)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let Some((_, first)) = first.first() else {
            return Ok(Vec::new());
        };
        self.store()
            .events_outside_run_after(run, first.id, kind, ids)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// The events the undos of the run wrote on it, oldest first.
    async fn undo_events(&self, run: RunId) -> Result<Vec<DomainEvent>, AppError> {
        let kind = EntityKind::from(RecordKind::ChangeRun);
        Ok(self
            .events_after_run(run, &kind, &[EntityId::from(run)])
            .await?
            .into_iter()
            .map(|(_, event)| event)
            .filter(|event| event.changes.iter().any(|change| change.field == "undone"))
            .collect())
    }

    /// Who undid the run and when, when it was undone: the fact the undo wrote on the run.
    pub(super) async fn undone_of(
        &self,
        held: &Evaluation,
        run: RunId,
    ) -> Result<Option<UndoneView>, AppError> {
        let events = self.undo_events(run).await?;
        let Some(done) = events.last() else {
            return Ok(None);
        };
        let Some(person) = done.actor.person() else {
            return Ok(None);
        };
        let by = self
            .person_ref(held, PeopleKind::Authors, PeopleContext::Ordinary, person)
            .await?;
        Ok(Some(UndoneView { by, at: done.time }))
    }

    /// Why each field that stayed, stayed, as the undos of the run wrote it down.
    pub(super) async fn kept_reasons(&self, run: RunId) -> Result<KeptReasons, AppError> {
        let mut reasons = KeptReasons::new();
        for event in self.undo_events(run).await? {
            for change in &event.changes {
                if let Some(kind) = change
                    .after
                    .as_ref()
                    .and_then(serde_json::Value::as_str)
                    .and_then(UndoSkipKind::from_code)
                    .filter(|_| change.field.starts_with("kept/"))
                {
                    reasons.insert(change.field.clone(), kind);
                }
            }
        }
        Ok(reasons)
    }

    /// What became of a field of an event of the run, given the events after the run: it went back
    /// when an event of the undo put it back, however it was changed after that; otherwise it
    /// stayed, for the reason the undo wrote down, and a change of somebody else names who.
    pub(super) async fn outcome_of(
        &self,
        held: &Evaluation,
        run: RunId,
        place: i64,
        event: &DomainEvent,
        field: &str,
        (later, kept): (&[(i64, DomainEvent)], &KeptReasons),
    ) -> Result<FieldOutcome, AppError> {
        let reason = bb_core::run::undo_reason(run);
        let touching: Vec<&DomainEvent> =
            after_the_run(later, place, &event.entity, field).collect();
        if touching
            .iter()
            .any(|each| each.reason.as_deref() == Some(reason.as_str()))
        {
            return Ok(FieldOutcome::WentBack);
        }
        let recorded = kept.get(&kept_key(&event.entity, field)).copied();
        if let Some(kind) = recorded.filter(|kind| *kind != UndoSkipKind::ChangedSince) {
            return Ok(FieldOutcome::Kept {
                kind,
                by: None,
                at: None,
            });
        }
        let Some(last) = touching.last() else {
            return Ok(FieldOutcome::Kept {
                kind: recorded.unwrap_or(UndoSkipKind::NotUndoable),
                by: None,
                at: None,
            });
        };
        let by = match last.actor.person() {
            Some(person) => Some(
                self.person_ref(held, PeopleKind::Authors, PeopleContext::Ordinary, person)
                    .await?,
            ),
            None => None,
        };
        Ok(FieldOutcome::Kept {
            kind: UndoSkipKind::ChangedSince,
            by,
            at: Some(last.time),
        })
    }
}

/// Why the fields that an undo left stayed, by the name `kept_key` makes.
pub(super) type KeptReasons = std::collections::HashMap<String, UndoSkipKind>;
