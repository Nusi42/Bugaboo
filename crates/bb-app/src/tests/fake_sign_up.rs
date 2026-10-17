//! The links of sign-ups, in the in-memory store.

use bb_core::sign_in::Invitation;
use bb_core::sign_up_invitation::SignUpInvitation;
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, Tx};
use crate::auth::SignUpStore;

impl FakeStore {
    /// The link with the secret that has this hash, as an invitation.
    pub(super) fn sign_up_link_by_hash(
        &self,
        hash: &bb_core::security_log::Digest,
    ) -> Option<Invitation> {
        self.read(|state| {
            state
                .sign_up_invitations
                .iter()
                .find(|known| known.token_hash == *hash)
                .cloned()
                .map(Invitation::from)
        })
    }
}

impl SignUpStore for FakeStore {
    async fn open_sign_up_for(
        &self,
        email: &str,
        now: Timestamp,
    ) -> Result<Option<SignUpInvitation>, FakeError> {
        Ok(self.read(|state| {
            state
                .sign_up_invitations
                .iter()
                .filter(|known| {
                    known.email == email && known.used_at.is_none() && known.expires_at > now
                })
                .max_by_key(|known| known.created_at)
                .cloned()
        }))
    }

    async fn put_sign_up_invitation(
        tx: &mut Tx,
        invitation: &SignUpInvitation,
    ) -> Result<(), FakeError> {
        tx.state
            .sign_up_invitations
            .retain(|known| known.id != invitation.id);
        tx.state.sign_up_invitations.push(invitation.clone());
        Ok(())
    }

    async fn delete_old_sign_up_invitations(
        tx: &mut Tx,
        before: Timestamp,
        now: Timestamp,
    ) -> Result<u64, FakeError> {
        let count = tx.state.sign_up_invitations.len();
        tx.state.sign_up_invitations.retain(|known| {
            !(known.created_at < before && (known.used_at.is_some() || known.expires_at <= now))
        });
        Ok(u64::try_from(count - tx.state.sign_up_invitations.len()).unwrap_or(u64::MAX))
    }
}
