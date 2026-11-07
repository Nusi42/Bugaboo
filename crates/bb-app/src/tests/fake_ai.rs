//! The setup of AI in the in-memory store: the key and the agreement.

use bb_core::ai::Agreement;
use jiff::Timestamp;
use zeroize::Zeroizing;

use super::fake::{FakeError, FakeStore, Tx};
use crate::ai::AiStore;

impl AiStore for FakeStore {
    async fn ai_key(&self) -> Result<Option<Zeroizing<Vec<u8>>>, FakeError> {
        Ok(self.read(|state| state.ai_key.clone().map(Zeroizing::new)))
    }

    async fn put_ai_key(&self, secret: &[u8], _at: Timestamp) -> Result<(), FakeError> {
        self.lock().state.ai_key = Some(secret.to_vec());
        Ok(())
    }

    async fn delete_ai_key(&self) -> Result<(), FakeError> {
        self.lock().state.ai_key = None;
        Ok(())
    }

    async fn ai_agreement(&self) -> Result<Option<Agreement>, FakeError> {
        Ok(self.read(|state| state.ai_agreement.clone()))
    }

    async fn put_ai_agreement(tx: &mut Tx, agreement: &Agreement) -> Result<(), FakeError> {
        tx.state.ai_agreement = Some(agreement.clone());
        Ok(())
    }

    async fn delete_ai_agreement(tx: &mut Tx) -> Result<(), FakeError> {
        tx.state.ai_agreement = None;
        Ok(())
    }

    async fn forget_ai_key(tx: &mut Tx) -> Result<(), FakeError> {
        tx.state.ai_key = None;
        Ok(())
    }
}
