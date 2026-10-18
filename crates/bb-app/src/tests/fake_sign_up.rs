//! The links of sign-ups, in the in-memory store.

use bb_core::id::InvitationId;
use bb_core::security_log::Digest;
use bb_core::sign_in::Invitation;
use bb_core::sign_up_invitation::{SignUpInvitation, SignUpMail};
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, Tx};
use crate::auth::SignUpStore;

impl FakeStore {
    /// The link with the secret that has this hash, as an invitation.
    pub(super) fn sign_up_link_by_hash(&self, hash: &Digest) -> Option<Invitation> {
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
    async fn claim_sign_up_mail(
        tx: &mut Tx,
        address: &Digest,
        invitation: InvitationId,
        since: Timestamp,
        now: Timestamp,
    ) -> Result<bool, FakeError> {
        let rows = &mut tx.state.sign_up_mail;
        match rows.iter_mut().find(|known| known.address == *address) {
            Some(known) if known.sent_at <= since || known.invitation == invitation => {
                known.invitation = invitation;
                known.sent_at = now;
                Ok(true)
            }
            Some(_) => Ok(false),
            None => {
                rows.push(SignUpMail {
                    address: *address,
                    invitation,
                    sent_at: now,
                });
                Ok(true)
            }
        }
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

    async fn forget_old_sign_up_mail(tx: &mut Tx, before: Timestamp) -> Result<u64, FakeError> {
        let count = tx.state.sign_up_mail.len();
        tx.state
            .sign_up_mail
            .retain(|known| known.sent_at >= before);
        Ok(u64::try_from(count - tx.state.sign_up_mail.len()).unwrap_or(u64::MAX))
    }
}
