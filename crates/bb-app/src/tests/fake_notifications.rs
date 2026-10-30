//! The notifications of the in-memory store.

use bb_core::event::DomainEvent;
use bb_core::id::{IssueId, NotificationId, UserId};
use bb_core::notification::{MailState, Notification, NotificationFilter, NotificationKind};
use jiff::Timestamp;
use zeroize::Zeroizing;

use super::fake::{FakeError, FakeStore, Tx};
use super::fake_many_changes::job;
use crate::jobs::QueuedJob;
use crate::notifications::NotificationStore;

/// Whether the notification is shown under the filter.
fn shown_under(row: &Notification, filter: NotificationFilter) -> bool {
    row.in_app
        && match filter {
            NotificationFilter::All => true,
            NotificationFilter::Unread => row.read_at.is_none(),
            NotificationFilter::Mentions => row.kind == NotificationKind::Mentioned,
        }
}

impl NotificationStore for FakeStore {
    async fn notification_events(&self, limit: u64) -> Result<Vec<(i64, DomainEvent)>, FakeError> {
        let shared = self.lock();
        let done = usize::try_from(shared.state.notification_position).unwrap_or(usize::MAX);
        Ok(shared
            .events
            .iter()
            .enumerate()
            .skip(done)
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .map(|(at, event)| (i64::try_from(at + 1).unwrap_or(i64::MAX), event.clone()))
            .collect())
    }

    async fn store_notifications(
        &self,
        rows: &[Notification],
        jobs: &[QueuedJob],
        position: i64,
        now: Timestamp,
    ) -> Result<(), FakeError> {
        let mut shared = self.lock();
        for row in rows {
            let there = shared
                .state
                .notifications
                .iter()
                .any(|known| known.recipient == row.recipient && known.once == row.once);
            if !there {
                shared.state.notifications.push(row.clone());
            }
        }
        for queued in jobs {
            shared.state.jobs.push(job(
                queued.id,
                &queued.kind,
                &queued.payload,
                queued.run_at,
                now,
            ));
        }
        shared.state.notification_position = shared.state.notification_position.max(position);
        Ok(())
    }

    async fn notifications_of(
        &self,
        recipient: UserId,
        filter: NotificationFilter,
        before: Option<NotificationId>,
        limit: u64,
    ) -> Result<Vec<Notification>, FakeError> {
        Ok(self.read(|state| {
            // Newest first by the time of the event, then by the ID, as the store reads them.
            let place = |row: &Notification| (row.at, row.id);
            let cursor = before.and_then(|before| {
                state
                    .notifications
                    .iter()
                    .find(|row| row.recipient == recipient && row.id == before)
                    .map(place)
            });
            let mut found: Vec<Notification> = state
                .notifications
                .iter()
                .filter(|row| {
                    row.recipient == recipient
                        && shown_under(row, filter)
                        && match (before, cursor) {
                            (None, _) => true,
                            (Some(_), Some(cursor)) => place(row) < cursor,
                            (Some(_), None) => false,
                        }
                })
                .cloned()
                .collect();
            found.sort_by_key(|row| std::cmp::Reverse(place(row)));
            found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
            found
        }))
    }

    async fn waiting_mail(
        &self,
        recipient: UserId,
        issue: Option<IssueId>,
        until: Timestamp,
        limit: u64,
    ) -> Result<Vec<Notification>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<Notification> = state
                .notifications
                .iter()
                .filter(|row| {
                    row.recipient == recipient
                        && row.issue == issue
                        && row.mail == MailState::Waiting
                        && row.at <= until
                })
                .cloned()
                .collect();
            found.sort_by_key(|row| (row.at, row.id));
            found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
            found
        }))
    }

    async fn set_mail_state(
        &self,
        ids: &[NotificationId],
        state: MailState,
    ) -> Result<(), FakeError> {
        for row in &mut self.lock().state.notifications {
            if ids.contains(&row.id) {
                row.mail = state;
            }
        }
        Ok(())
    }

    async fn finish_mail(
        &self,
        recipient: UserId,
        issue: Option<IssueId>,
        until: Timestamp,
        state: MailState,
    ) -> Result<u64, FakeError> {
        let mut finished = 0;
        for row in &mut self.lock().state.notifications {
            if row.recipient == recipient
                && row.issue == issue
                && row.mail == MailState::Waiting
                && row.at <= until
            {
                row.mail = state;
                finished += 1;
            }
        }
        Ok(finished)
    }

    async fn mail_password(&self) -> Result<Option<Zeroizing<Vec<u8>>>, FakeError> {
        Ok(self.read(|state| state.mail_password.clone().map(Zeroizing::new)))
    }

    async fn put_mail_password(&self, secret: &[u8], _at: Timestamp) -> Result<(), FakeError> {
        self.lock().state.mail_password = Some(secret.to_vec());
        Ok(())
    }

    async fn forget_mail_password(tx: &mut Tx) -> Result<(), FakeError> {
        tx.state.mail_password = None;
        Ok(())
    }

    async fn delete_mail_password(&self) -> Result<(), FakeError> {
        self.lock().state.mail_password = None;
        Ok(())
    }

    async fn purge_notifications(&self, before: Timestamp) -> Result<u64, FakeError> {
        let mut shared = self.lock();
        let kept = shared.state.notifications.len();
        shared.state.notifications.retain(|row| row.at >= before);
        Ok(u64::try_from(kept - shared.state.notifications.len()).unwrap_or(u64::MAX))
    }

    async fn mark_notifications_read(
        tx: &mut Tx,
        recipient: UserId,
        ids: &[NotificationId],
        at: Timestamp,
    ) -> Result<u64, FakeError> {
        let mut marked = 0;
        for row in &mut tx.state.notifications {
            if row.recipient == recipient && ids.contains(&row.id) && row.read_at.is_none() {
                row.read_at = Some(at);
                marked += 1;
            }
        }
        Ok(marked)
    }

    async fn mark_notifications_read_up_to(
        tx: &mut Tx,
        recipient: UserId,
        up_to: NotificationId,
        at: Timestamp,
    ) -> Result<u64, FakeError> {
        let mut marked = 0;
        for row in &mut tx.state.notifications {
            if row.recipient == recipient && row.in_app && row.read_at.is_none() && row.id <= up_to
            {
                row.read_at = Some(at);
                marked += 1;
            }
        }
        Ok(marked)
    }
}
