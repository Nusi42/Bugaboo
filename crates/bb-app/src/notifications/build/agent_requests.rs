//! The notification of a request for agents (ADR-0029, amendment M9): a part of the reader of the
//! outbox, kept apart so the reader stays small.

use bb_core::agent_request::RequestState;
use bb_core::event::DomainEvent;
use bb_core::notification::{NotificationKind, Once};

use super::{Hears, Keeps, Round, Subject};
use crate::auth::{AppError, AuthStore, Ctx};

impl<S: AuthStore> Ctx<'_, S> {
    /// A program asked the person for agents: the person hears of it at once, whatever they
    /// chose, because a request may lead to keys in their name. The answers tell nobody.
    pub(super) async fn agent_request_event(
        &self,
        round: &mut Round,
        event: &DomainEvent,
    ) -> Result<(), AppError> {
        let asked = event.changes.iter().any(|change| {
            change.field == "state"
                && change.after.as_ref().and_then(|value| value.as_str())
                    == Some(RequestState::Waiting.code())
        });
        if !asked {
            return Ok(());
        }
        let Some(person) = (match &event.actor {
            bb_core::event::Actor::Person(person) => Some(*person),
            _ => None,
        }) else {
            return Ok(());
        };
        // The person asked through a key or an app of theirs, so the event names them as who did it;
        // they are told all the same.
        let told = DomainEvent {
            actor: bb_core::event::Actor::System,
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
            kinds: vec![Some(NotificationKind::AgentsAskedFor)],
            follows: false,
            once: Once::Event(event.id),
        }];
        self.deliver(round, &subject, hears).await
    }
}
