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

/// What the events after the run did to one field of one record of it.
fn last_after(
    later: &[(i64, DomainEvent)],
    place: i64,
    entity: &EntityRef,
    field: &str,
) -> Option<DomainEvent> {
    later
        .iter()
        .filter(|(at, event)| {
            *at > place
                && event.entity == *entity
                && event.changes.iter().any(|change| change.field == field)
        })
        .map(|(_, event)| event.clone())
        .next_back()
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

    /// Who undid the run and when, when it was undone: the fact the undo wrote on the run.
    pub(super) async fn undone_of(
        &self,
        held: &Evaluation,
        run: RunId,
    ) -> Result<Option<UndoneView>, AppError> {
        let kind = EntityKind::from(RecordKind::ChangeRun);
        let events = self
            .events_after_run(run, &kind, &[EntityId::from(run)])
            .await?;
        let Some(done) = events
            .iter()
            .map(|(_, event)| event)
            .rfind(|event| event.changes.iter().any(|change| change.field == "undone"))
        else {
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

    /// What became of a field of an event of the run, given the events after the run.
    pub(super) async fn outcome_of(
        &self,
        held: &Evaluation,
        run: RunId,
        place: i64,
        event: &DomainEvent,
        field: &str,
        later: &[(i64, DomainEvent)],
    ) -> Result<FieldOutcome, AppError> {
        let reason = bb_core::run::undo_reason(run);
        let Some(last) = last_after(later, place, &event.entity, field) else {
            return Ok(FieldOutcome::Kept {
                kind: UndoSkipKind::NotUndoable,
                by: None,
                at: None,
            });
        };
        if last.reason.as_deref() == Some(reason.as_str()) {
            return Ok(FieldOutcome::WentBack);
        }
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
