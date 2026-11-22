//! The notification of a sign-in refused for the day (SEC-14): a part of the reader of the outbox,
//! kept apart so the reader stays small.

use bb_core::event::DomainEvent;
use bb_core::id::UserId;
use bb_core::notification::{NotificationKind, Once};

use super::{Hears, Keeps, Round, Subject};
use crate::auth::{AppError, AuthStore, Ctx};
use crate::member::person::SIGN_IN_CAPPED;

impl<S: AuthStore> Ctx<'_, S> {
    /// Sign-in to the person's account from networks it never signed in from was refused for the
    /// rest of the day: the person hears of it at once, whatever they chose, because somebody is
    /// trying to get into their account.
    pub(super) async fn sign_in_capped_event(
        &self,
        round: &mut Round,
        event: &DomainEvent,
    ) -> Result<(), AppError> {
        let capped = event
            .changes
            .iter()
            .any(|change| change.field == SIGN_IN_CAPPED);
        let Ok(person) = UserId::from_bytes(event.entity.id.to_bytes()) else {
            return Ok(());
        };
        if !capped {
            return Ok(());
        }
        let subject = Subject {
            event,
            issue: None,
            record: Some(event.entity.id),
            keeps: Keeps::Nothing,
        };
        let hears = vec![Hears {
            person,
            kinds: vec![Some(NotificationKind::SignInCapped)],
            follows: false,
            once: Once::Event(event.id),
        }];
        self.deliver(round, &subject, hears).await
    }
}
