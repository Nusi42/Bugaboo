//! The notification of a changed work day (ADR-0035, decision 7): a part of the reader of the
//! outbox, kept apart so the reader stays small.

use bb_core::event::{DomainEvent, FieldChange};
use bb_core::id::UserId;
use bb_core::notification::{NotificationKind, Once};
use serde_json::json;

use super::{Hears, Keeps, Round, Subject};
use crate::auth::{AppError, AuthStore, Ctx};
use crate::event_members::text_set;
use crate::member::work_day;

impl<S: AuthStore> Ctx<'_, S> {
    /// A change of a work day by somebody else: the person of the day hears of it, and nobody
    /// else (ADR-0035, decision 7). The row keeps the date and the reason only, never the times,
    /// and it goes with the day when old work days are deleted.
    pub(super) async fn work_day_event(
        &self,
        round: &mut Round,
        event: &DomainEvent,
    ) -> Result<(), AppError> {
        let Some(person) =
            text_set(&event.changes, work_day::PERSON).and_then(|text| text.parse::<UserId>().ok())
        else {
            return Ok(());
        };
        let mut kept = vec![FieldChange {
            field: work_day::DATE.to_owned(),
            before: None,
            after: text_set(&event.changes, work_day::DATE).map(|date| json!(date)),
        }];
        if let Some(reason) = &event.reason {
            kept.push(FieldChange {
                field: work_day::REASON.to_owned(),
                before: None,
                after: Some(json!(reason)),
            });
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
            kinds: vec![Some(NotificationKind::WorkDayChanged)],
            follows: false,
            once: Once::Event(event.id),
        }];
        self.deliver(round, &subject, hears).await
    }
}
