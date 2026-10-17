//! The port of notifications: what the reader, the list and the e-mail jobs need from storage
//! (NTF-01, NTF-02, ADR-0033).

use std::future::Future;

use bb_core::event::DomainEvent;
use bb_core::id::{IssueId, NotificationId, UserId};
use bb_core::notification::{MailState, Notification, NotificationFilter};
use jiff::Timestamp;
use zeroize::Zeroizing;

use crate::comments::CommentStore;
use crate::follow::FollowStore;
use crate::jobs::QueuedJob;
use crate::store_macros::{reads, writes};

/// What the notifications need from storage.
pub trait NotificationStore: FollowStore + CommentStore + 'static {
    reads! {
        /// The next events the reader has not finished, with their places in the outbox, in the
        /// order they were committed, at most `limit`.
        fn notification_events(limit: u64) -> Vec<(i64, DomainEvent)>;
        /// Stores the notifications made from events, each once (one that is there already for
        /// the same person and key is left as it is), queues the jobs that send their e-mail in
        /// the same transaction, and records that the reader has finished everything up to
        /// `position`. A reader that stops half way reads the same events again and makes the
        /// same rows.
        fn store_notifications(
            rows: &[Notification],
            jobs: &[QueuedJob],
            position: i64,
            now: Timestamp
        ) -> ();
        /// The notifications of a person that are shown in the app, newest first: the ones
        /// before `before` (the last of the page before), at most `limit`.
        fn notifications_of(
            recipient: UserId,
            filter: NotificationFilter,
            before: Option<NotificationId>,
            limit: u64
        ) -> Vec<Notification>;
        /// The notifications of a person with these IDs that exist.
        fn notifications_by_ids(recipient: UserId, ids: &[NotificationId]) -> Vec<Notification>;
        /// The notifications of a person about one issue (or about no issue) whose e-mail waits and
        /// that happened at or before `until`, oldest first, at most `limit`.
        fn waiting_mail(
            recipient: UserId,
            issue: Option<IssueId>,
            until: Timestamp,
            limit: u64
        ) -> Vec<Notification>;
        /// Says where the e-mail of these notifications is. Bookkeeping, not a domain change.
        fn set_mail_state(ids: &[NotificationId], state: MailState) -> ();
        /// Says where the e-mail is of every notification of a person about an issue (or no
        /// issue) whose e-mail waits and that happened at or before `until`: it went, or it
        /// never will (the job died, e-mail is off). Returns how many.
        fn finish_mail(
            recipient: UserId,
            issue: Option<IssueId>,
            until: Timestamp,
            state: MailState
        ) -> u64;
        /// The password of the mail server, stored encrypted (SEC-16). The bytes are wiped from
        /// memory when dropped.
        fn mail_password() -> Option<Zeroizing<Vec<u8>>>;
        /// Stores the password of the mail server, replacing the one there. Bookkeeping of a
        /// secret, not a domain change: the use case writes the event of it.
        fn put_mail_password(secret: &[u8], at: Timestamp) -> ();
        /// Deletes the password of the mail server.
        fn delete_mail_password() -> ();
        /// Deletes the notifications that happened before `before` (the keep period, PRIV-05).
        /// Returns how many went.
        fn purge_notifications(before: Timestamp) -> u64;
    }

    writes! {
        /// Deletes the password of the mail server inside a change: the settings that name the mail
        /// server changed, so the password of the old one goes with them.
        fn forget_mail_password() -> ();
        /// Marks these notifications of a person as read; the ones read already keep their date.
        /// Returns how many it marked.
        fn mark_notifications_read(
            recipient: UserId,
            ids: &[NotificationId],
            at: Timestamp
        ) -> u64;
        /// Marks every unread notification of a person as read, up to and including this one (the
        /// IDs grow with time, so the later ones are left alone). Returns how many.
        fn mark_notifications_read_up_to(
            recipient: UserId,
            up_to: NotificationId,
            at: Timestamp
        ) -> u64;
    }
}
