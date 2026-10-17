//! The accounts that wait for a confirmed address, in the in-memory store.

use bb_core::confirmation::AddressConfirmation;
use bb_core::id::UserId;
use bb_core::security_log::Digest;
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, Tx};
use crate::notifications::ConfirmationStore;

impl ConfirmationStore for FakeStore {
    async fn address_confirmation_by_hash(
        &self,
        hash: &Digest,
    ) -> Result<Option<AddressConfirmation>, FakeError> {
        Ok(self.read(|state| {
            state
                .confirmations
                .iter()
                .find(|known| known.token_hash == *hash)
                .cloned()
        }))
    }

    async fn unconfirmed(&self, person: UserId) -> Result<Option<AddressConfirmation>, FakeError> {
        Ok(self.read(|state| {
            state
                .confirmations
                .iter()
                .find(|known| known.person == person)
                .cloned()
        }))
    }

    async fn renew_confirmation(
        &self,
        person: UserId,
        hash: &Digest,
        expires_at: Timestamp,
    ) -> Result<bool, FakeError> {
        let mut shared = self.lock();
        let Some(found) = shared
            .state
            .confirmations
            .iter_mut()
            .find(|known| known.person == person)
        else {
            return Ok(false);
        };
        found.token_hash = *hash;
        found.expires_at = expires_at;
        Ok(true)
    }

    async fn stale_sign_ups(
        &self,
        before: Timestamp,
        limit: u64,
    ) -> Result<Vec<UserId>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<&AddressConfirmation> = state
                .confirmations
                .iter()
                .filter(|known| known.expires_at < before)
                .collect();
            found.sort_by_key(|known| known.expires_at);
            found
                .into_iter()
                .take(usize::try_from(limit).unwrap_or(usize::MAX))
                .map(|known| known.person)
                .collect()
        }))
    }

    async fn put_address_confirmation(
        tx: &mut Tx,
        confirmation: &AddressConfirmation,
    ) -> Result<(), FakeError> {
        tx.state
            .confirmations
            .retain(|known| known.person != confirmation.person);
        tx.state.confirmations.push(confirmation.clone());
        Ok(())
    }

    async fn use_address_confirmation(
        tx: &mut Tx,
        hash: &Digest,
        now: Timestamp,
    ) -> Result<bool, FakeError> {
        let before = tx.state.confirmations.len();
        tx.state
            .confirmations
            .retain(|known| !(known.token_hash == *hash && known.expires_at > now));
        Ok(tx.state.confirmations.len() < before)
    }

    async fn delete_unconfirmed_person(tx: &mut Tx, person: UserId) -> Result<bool, FakeError> {
        let before = tx.state.confirmations.len();
        tx.state
            .confirmations
            .retain(|known| known.person != person);
        if tx.state.confirmations.len() == before {
            return Ok(false);
        }
        tx.state.handles.retain(|known| known.person != person);
        tx.state.people.retain(|(known, _)| known.id != person);
        Ok(true)
    }
}
