//! The notifications of *Download my data* (ADR-0044, decisions 2 and 4): a part of the reader of
//! the outbox, kept apart so the reader stays small.

use bb_core::data_export::ExportState;
use bb_core::event::{DomainEvent, FieldChange};
use bb_core::id::DataExportId;
use bb_core::notification::{NotificationKind, Once};

use super::{Hears, Keeps, Round, Subject};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::event_members::text_set;
use crate::member::data_export as member;

impl<S: AuthStore> Ctx<'_, S> {
    /// An export changed where it stands. When its file is ready, whoever asked for it hears of it.
    /// When an administrator asked for somebody else's data, that person hears of it at once, with
    /// the reason and nothing else of the event, whatever they chose: they cannot switch it off.
    pub(super) async fn data_export_event(
        &self,
        round: &mut Round,
        event: &DomainEvent,
    ) -> Result<(), AppError> {
        let Some(state) = text_set(&event.changes, member::STATE).and_then(ExportState::from_code)
        else {
            return Ok(());
        };
        let Ok(id) = DataExportId::from_bytes(event.entity.id.to_bytes()) else {
            return Ok(());
        };
        // An export deleted before the reader came to it tells nobody any more.
        let Some(export) = self
            .store()
            .data_export(id)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(());
        };
        let (person, kind, kept) = match state {
            ExportState::Ready => (export.asked_by, NotificationKind::ExportReady, Vec::new()),
            ExportState::Waiting if export.by_somebody_else() => {
                let reason: Vec<FieldChange> = event
                    .changes
                    .iter()
                    .filter(|change| change.field == member::REASON)
                    .cloned()
                    .collect();
                (
                    export.person,
                    NotificationKind::ExportByAdministrator,
                    reason,
                )
            }
            ExportState::Waiting | ExportState::Failed => return Ok(()),
        };
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
            kinds: vec![Some(kind)],
            follows: false,
            once: Once::Event(event.id),
        }];
        self.deliver(round, &subject, hears).await
    }
}
