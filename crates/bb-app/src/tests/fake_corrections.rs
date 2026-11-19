//! The corrections of a person's name and address, in the in-memory store.

use bb_core::e_mail_change::EmailChange;
use bb_core::id::UserId;
use bb_core::security_log::Digest;
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, Tx, remove_at_most};
use crate::people::CorrectionStore;

impl CorrectionStore for FakeStore {
    async fn e_mail_change_by_digest(
        &self,
        digest: &Digest,
    ) -> Result<Option<EmailChange>, FakeError> {
        Ok(self.read(|state| {
            state
                .e_mail_changes
                .iter()
                .find(|known| known.token_digest == *digest)
                .cloned()
        }))
    }

    async fn set_person_name(tx: &mut Tx, person: UserId, name: &str) -> Result<(), FakeError> {
        let found = tx
            .state
            .people
            .iter_mut()
            .find(|(known, _)| known.id == person)
            .ok_or(FakeError::Aborted)?;
        name.clone_into(&mut found.0.display_name);
        Ok(())
    }

    async fn set_person_email(tx: &mut Tx, person: UserId, email: &str) -> Result<(), FakeError> {
        if tx
            .state
            .people
            .iter()
            .any(|(known, _)| known.email == email && known.id != person)
        {
            return Err(FakeError::Duplicate);
        }
        let found = tx
            .state
            .people
            .iter_mut()
            .find(|(known, _)| known.id == person)
            .ok_or(FakeError::Aborted)?;
        email.clone_into(&mut found.0.email);
        Ok(())
    }

    async fn put_e_mail_change(tx: &mut Tx, change: &EmailChange) -> Result<(), FakeError> {
        let rows = &mut tx.state.e_mail_changes;
        rows.retain(|known| known.person != change.person);
        rows.push(change.clone());
        Ok(())
    }

    async fn take_e_mail_change(tx: &mut Tx, digest: &Digest) -> Result<bool, FakeError> {
        let rows = &mut tx.state.e_mail_changes;
        let before = rows.len();
        rows.retain(|known| known.token_digest != *digest);
        Ok(rows.len() < before)
    }

    async fn drop_e_mail_change(tx: &mut Tx, person: UserId) -> Result<(), FakeError> {
        tx.state
            .e_mail_changes
            .retain(|known| known.person != person);
        Ok(())
    }

    async fn delete_old_e_mail_changes(
        tx: &mut Tx,
        now: Timestamp,
        limit: u64,
    ) -> Result<u64, FakeError> {
        Ok(remove_at_most(
            &mut tx.state.e_mail_changes,
            limit,
            |known| !known.works_at(now),
        ))
    }
}
