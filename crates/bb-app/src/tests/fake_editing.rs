//! The editing part of the in-memory store: edit locks, drafts and the notes for people who are
//! editing. The decisions are the ones of `bb_core::edit`, applied to lists.

use bb_core::edit::{Draft, DraftTarget, EditLock, EditNotice, EditWhat, Hold, Taking};
use bb_core::event::{DomainEvent, EntityRef};
use bb_core::id::UserId;
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, State, Tx};
use crate::editing::EditStore;

fn lock_on<'a>(state: &'a State, what: &EditWhat) -> Option<&'a EditLock> {
    state.locks.iter().find(|lock| lock.what() == *what)
}

fn draft_of<'a>(state: &'a State, author: UserId, target: &DraftTarget) -> Option<&'a Draft> {
    state
        .drafts
        .iter()
        .find(|draft| draft.author == author && draft.target == *target)
}

fn remove_lock(state: &mut State, what: &EditWhat) -> Option<EditLock> {
    let at = state.locks.iter().position(|lock| lock.what() == *what)?;
    Some(state.locks.remove(at))
}

fn remove_draft(state: &mut State, author: UserId, target: &DraftTarget) -> bool {
    let before = state.drafts.len();
    state
        .drafts
        .retain(|draft| !(draft.author == author && draft.target == *target));
    state.drafts.len() != before
}

impl EditStore for FakeStore {
    async fn take_edit_lock(&self, wanted: &EditLock, now: Timestamp) -> Result<Taking, FakeError> {
        let mut shared = self.lock();
        let state = &mut shared.state;
        let existing = lock_on(state, &wanted.what()).cloned();
        let taking = Taking::decide(existing.as_ref(), wanted, now);
        if let Taking::Granted(lock) = &taking {
            remove_lock(state, &lock.what());
            state.locks.push(lock.clone());
        }
        Ok(taking)
    }

    async fn renew_edit_lock(
        &self,
        what: &EditWhat,
        hold: Hold,
        now: Timestamp,
        ends: Timestamp,
    ) -> Result<bool, FakeError> {
        let mut shared = self.lock();
        let renewed =
            shared.state.locks.iter_mut().find(|lock| {
                lock.what() == *what && lock.is_held_by(hold) && !lock.has_expired(now)
            });
        Ok(renewed.is_some_and(|lock| {
            lock.expires_at = ends;
            true
        }))
    }

    async fn release_edit_lock(&self, what: &EditWhat, hold: Hold) -> Result<bool, FakeError> {
        let mut shared = self.lock();
        let before = shared.state.locks.len();
        shared
            .state
            .locks
            .retain(|lock| !(lock.what() == *what && lock.is_held_by(hold)));
        Ok(shared.state.locks.len() != before)
    }

    async fn save_draft(&self, draft: &Draft) -> Result<(), FakeError> {
        let mut shared = self.lock();
        remove_draft(&mut shared.state, draft.author, &draft.target);
        shared.state.drafts.push(draft.clone());
        Ok(())
    }

    async fn delete_draft(&self, author: UserId, target: &DraftTarget) -> Result<bool, FakeError> {
        Ok(remove_draft(&mut self.lock().state, author, target))
    }

    async fn purge_drafts(&self, before: Timestamp, now: Timestamp) -> Result<u64, FakeError> {
        let mut shared = self.lock();
        shared.state.locks.retain(|lock| !lock.has_expired(now));
        let drafts = shared.state.drafts.len();
        shared.state.drafts.retain(|draft| draft.saved_at >= before);
        shared.state.notices.retain(|notice| notice.at >= before);
        Ok(u64::try_from(drafts - shared.state.drafts.len()).unwrap_or(u64::MAX))
    }

    async fn latest_version_event_about(
        &self,
        record: &EntityRef,
    ) -> Result<Option<DomainEvent>, FakeError> {
        Ok(self
            .lock()
            .events
            .iter()
            .rev()
            .find(|event| event.entity == *record && event.counted_version())
            .cloned())
    }

    async fn field_changed_since(
        &self,
        record: &EntityRef,
        field: &str,
        version: i64,
    ) -> Result<bool, FakeError> {
        Ok(self
            .lock()
            .events
            .iter()
            .any(|event| event.entity == *record && event.changes_field_after(field, version)))
    }

    async fn take_edit_notices(
        &self,
        recipient: UserId,
        what: &EditWhat,
    ) -> Result<Vec<EditNotice>, FakeError> {
        let mut shared = self.lock();
        let (mine, others): (Vec<_>, Vec<_>) = std::mem::take(&mut shared.state.notices)
            .into_iter()
            .partition(|notice| {
                notice.recipient == recipient
                    && notice.record == what.record
                    && notice.part == what.part
            });
        shared.state.notices = others;
        let mut mine = mine;
        mine.sort_by_key(|notice| notice.at);
        Ok(mine)
    }

    async fn edit_lock_in(tx: &mut Tx, what: EditWhat) -> Result<Option<EditLock>, FakeError> {
        Ok(lock_on(&tx.state, &what).cloned())
    }

    async fn draft_in(
        tx: &mut Tx,
        author: UserId,
        target: DraftTarget,
        not_before: Timestamp,
    ) -> Result<Option<Draft>, FakeError> {
        Ok(draft_of(&tx.state, author, &target)
            .filter(|draft| draft.saved_at >= not_before)
            .cloned())
    }

    async fn replace_edit_lock(
        tx: &mut Tx,
        wanted: &EditLock,
    ) -> Result<Option<EditLock>, FakeError> {
        let former = remove_lock(&mut tx.state, &wanted.what());
        tx.state.locks.push(wanted.clone());
        Ok(former)
    }

    async fn remove_edit_lock(tx: &mut Tx, what: &EditWhat) -> Result<Option<EditLock>, FakeError> {
        Ok(remove_lock(&mut tx.state, what))
    }

    async fn remove_edit_lock_held(
        tx: &mut Tx,
        what: &EditWhat,
        hold: Hold,
    ) -> Result<bool, FakeError> {
        let before = tx.state.locks.len();
        tx.state
            .locks
            .retain(|lock| !(lock.what() == *what && lock.is_held_by(hold)));
        Ok(tx.state.locks.len() != before)
    }

    async fn delete_draft_in(
        tx: &mut Tx,
        author: UserId,
        target: &DraftTarget,
    ) -> Result<bool, FakeError> {
        Ok(remove_draft(&mut tx.state, author, target))
    }

    async fn put_edit_notice(tx: &mut Tx, notice: &EditNotice) -> Result<(), FakeError> {
        tx.state.notices.retain(|known| {
            !(known.recipient == notice.recipient
                && known.record == notice.record
                && known.part == notice.part
                && known.kind == notice.kind)
        });
        tx.state.notices.push(notice.clone());
        Ok(())
    }
}
