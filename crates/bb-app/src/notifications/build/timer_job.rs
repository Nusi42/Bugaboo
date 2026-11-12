//! The notification of a timer that runs long (ADR-0035, decision 3): a part of the reader of the
//! outbox, kept apart so the reader stays small.

use bb_core::event::DomainEvent;
use bb_core::id::UserId;
use bb_core::notification::{NotificationKind, Once};

use super::{Hears, Keeps, Round, Subject};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::history::issue_in;
use crate::member::timer;

impl<S: AuthStore> Ctx<'_, S> {
    /// The job of a long timer marked the timer (ADR-0035, decision 3): its person hears of it
    /// once, while they see the issue it runs on. No other change of a timer tells anybody.
    pub(super) async fn timer_event(
        &self,
        round: &mut Round,
        event: &DomainEvent,
    ) -> Result<(), AppError> {
        let warned = event
            .changes
            .iter()
            .any(|change| change.field == timer::WARNED_AT);
        let Ok(person) = UserId::from_bytes(event.entity.id.to_bytes()) else {
            return Ok(());
        };
        let Some(issue_id) = issue_in(&event.changes, timer::ISSUE, true).filter(|_| warned) else {
            return Ok(());
        };
        let failed = |error| internal::<S>(&error);
        let Some(issue) = self.store().issue(issue_id).await.map_err(failed)? else {
            return Ok(());
        };
        let subject = Subject {
            event,
            issue: Some(&issue),
            record: None,
            keeps: Keeps::Nothing,
        };
        let hears = vec![Hears {
            person,
            kinds: vec![Some(NotificationKind::LongTimer)],
            follows: false,
            once: Once::Event(event.id),
        }];
        self.deliver(round, &subject, hears).await
    }
}
