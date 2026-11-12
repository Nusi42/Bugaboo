//! The notification of changed work times (ADR-0033, amendment M8): a part of the reader of the
//! outbox, kept apart so the reader stays small.

use bb_core::event::{DomainEvent, FieldChange};
use bb_core::id::UserId;
use bb_core::notification::{NotificationKind, Once};

use super::{Hears, Keeps, Round, Subject};
use crate::auth::{AppError, AuthStore, Ctx};
use crate::event_members::text_set;
use crate::member::work_times;

impl<S: AuthStore> Ctx<'_, S> {
    /// Somebody else entered, changed or removed an away time of a person, or changed their usual
    /// work days or work hours: the person hears of it, and nobody else. The row keeps the days and
    /// the member that says what was done, never more: an event holds no note, and a change by the
    /// person themselves tells nobody.
    pub(super) async fn work_times_event(
        &self,
        round: &mut Round,
        event: &DomainEvent,
    ) -> Result<(), AppError> {
        let Some(person) = text_set(&event.changes, work_times::PERSON)
            .and_then(|text| text.parse::<UserId>().ok())
        else {
            return Ok(());
        };
        if event.actor.person() == Some(person) {
            return Ok(());
        }
        let kept: Vec<FieldChange> = event
            .changes
            .iter()
            .filter(|change| matches!(change.field.as_str(), work_times::AWAY | work_times::USUAL))
            .cloned()
            .collect();
        if kept.is_empty() {
            return Ok(());
        }
        let told = DomainEvent {
            changes: kept,
            ..event.clone()
        };
        let subject = Subject {
            event: &told,
            issue: None,
            record: Some(event.entity.id),
            keeps: Keeps::All,
        };
        let hears = vec![Hears {
            person,
            kinds: vec![Some(NotificationKind::WorkTimesChanged)],
            follows: false,
            once: Once::Event(event.id),
        }];
        self.deliver(round, &subject, hears).await
    }
}
