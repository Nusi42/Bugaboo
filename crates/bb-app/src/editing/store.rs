//! The port of the editing use cases: what they need from storage for edit locks, drafts and the
//! notes for the person who is editing.
//!
//! The lease, the heartbeat and the draft are the state of an open editor, not facts about a record,
//! so they are *bookkeeping* like the time a session was last used: they are methods that run on
//! their own, each atomic, and write no domain event (ADR-0019). What other people must be able to
//! learn from the event log (*Edit anyway*, *Ask … to finish*, an administrator releasing a lock)
//! is written inside `record_change` with the associated functions, so the change and its event
//! are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::edit::{Draft, DraftTarget, EditLock, EditNotice, EditWhat, Hold, Taking};
use bb_core::event::{DomainEvent, EntityRef};
use bb_core::id::UserId;
use jiff::Timestamp;

use crate::ports::ChangeStore;
use crate::store_macros::{queries, reads, writes};

/// What the editing use cases need from storage.
pub trait EditStore: ChangeStore + 'static {
    queries! {
        /// The lock on a thing, if a row for it is stored. The row may have expired already: the
        /// caller judges it with [`EditLock::has_expired`], so a lock never depends on when the
        /// expired rows were deleted.
        fn edit_lock(what: EditWhat) -> Option<EditLock> = edit_lock_in;
        /// The draft of a person for a target, if it is stored and not older than `not_before`
        /// (the keep period: an older one is as good as deleted, whenever the job runs).
        fn draft(
            author: UserId,
            target: DraftTarget,
            not_before: Timestamp
        ) -> Option<Draft> = draft_in;
    }

    reads! {
        /// Takes the lock for `wanted` if the thing is free (no row, or an expired one), or if the
        /// same editor takes it again. Otherwise says who holds it. Expired rows of the thing
        /// are deleted on the way. One step, so two editors never both get it.
        fn take_edit_lock(wanted: &EditLock, now: Timestamp) -> Taking;
        /// Makes the lease of the lock held like this last until `ends`, when it is not expired
        /// at `now`; `false` when the lock is gone, expired or held by another person or with
        /// another token.
        fn renew_edit_lock(
            what: &EditWhat,
            hold: Hold,
            now: Timestamp,
            ends: Timestamp
        ) -> bool;
        /// Lets go of the lock held like this; `false` when there was none.
        fn release_edit_lock(what: &EditWhat, hold: Hold) -> bool;
        /// Stores a draft, replacing the person's older one for the same target.
        fn save_draft(draft: &Draft) -> ();
        /// Deletes the draft of a person for a target; `false` when there was none.
        fn delete_draft(author: UserId, target: &DraftTarget) -> bool;
        /// Deletes the drafts last changed before `before`, the notes of the same age, and the
        /// lock rows that ran out at `now`, so nothing outlives its keep period. Returns how many
        /// drafts went.
        fn purge_drafts(before: Timestamp, now: Timestamp) -> u64;
        /// The newest event about a record that counted its version up (`DomainEvent::
        /// counted_version`), however far back it is: where the conflict finds who changed the
        /// record last, when, and what.
        fn latest_version_event_about(record: &EntityRef) -> Option<DomainEvent>;
        /// Whether a change after `version` of the record changed `field`: an event that counted
        /// the version up past it and holds a change of that field. A long text is saved on a newer
        /// version when only other fields changed since the person started.
        fn field_changed_since(record: &EntityRef, field: &str, version: i64) -> bool;
        /// The notes for a person about a thing, oldest first; they are deleted as they are
        /// read, so each is shown once.
        fn take_edit_notices(recipient: UserId, what: &EditWhat) -> Vec<EditNotice>;
    }

    writes! {
        /// Gives the lock to `wanted` whoever holds it now, and returns the lock it replaced (an
        /// expired one included). *Edit anyway*; the replaced holder's draft is not touched.
        fn replace_edit_lock(wanted: &EditLock) -> Option<EditLock>;
        /// Deletes the lock on a thing whoever holds it, and returns it.
        fn remove_edit_lock(what: &EditWhat) -> Option<EditLock>;
        /// Deletes the lock when it is held like this; `false` when it is not.
        fn remove_edit_lock_held(what: &EditWhat, hold: Hold) -> bool;
        /// Deletes the draft of a person for a target, inside a change.
        fn delete_draft_in(author: UserId, target: &DraftTarget) -> bool;
        /// Stores a note for the person who is editing; a newer note of the same kind for the same
        /// thing replaces the older one.
        fn put_edit_notice(notice: &EditNotice) -> ();
    }
}
