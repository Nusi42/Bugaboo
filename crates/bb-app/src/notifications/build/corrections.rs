//! The notification of a correction by an administrator (PRIV-06, ADR-0044, decision 9): a part of
//! the reader of the outbox, kept apart so the reader stays small.

use bb_core::event::{DomainEvent, FieldChange};
use bb_core::id::UserId;
use bb_core::notification::{NotificationKind, Once};

use super::{Hears, Keeps, Round, Subject};
use crate::auth::{AppError, AuthStore, Ctx};
use crate::member;

impl<S: AuthStore> Ctx<'_, S> {
    /// A person's name or address changed. When an administrator corrected it (the event changes
    /// the name or the address and holds a reason), the person hears of it at once, with the
    /// reason and nothing else of the event, whatever they chose: they cannot switch it off. A
    /// change of their own tells nobody.
    pub(super) async fn person_event(
        &self,
        round: &mut Round,
        event: &DomainEvent,
    ) -> Result<(), AppError> {
        let corrects = event.changes.iter().any(|change| {
            [member::person::NAME, member::person::EMAIL].contains(&change.field.as_str())
        });
        let reason: Vec<FieldChange> = event
            .changes
            .iter()
            .filter(|change| change.field == member::REASON)
            .cloned()
            .collect();
        if !corrects || reason.is_empty() {
            return Ok(());
        }
        let Ok(person) = UserId::from_bytes(event.entity.id.to_bytes()) else {
            return Ok(());
        };
        let told = DomainEvent {
            changes: reason,
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
            kinds: vec![Some(NotificationKind::DetailsCorrected)],
            follows: false,
            once: Once::Event(event.id),
        }];
        self.deliver(round, &subject, hears).await
    }
}
