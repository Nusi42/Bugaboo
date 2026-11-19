//! Opening a session inside a change (PRIV-04, ADR-0044, decision 5): only for a person who is
//! still active, so a sign-in that races *Switch off* stops instead of leaving a session that
//! works.

use bb_core::ErrorCode;
use bb_core::id::SessionId;
use bb_core::sign_in::SignInSession;

use super::AuthStore;
use crate::projects::Why;

/// Opens `session` inside a change, only for a person who is still active: a sign-in that races
/// *Switch off* stops instead of leaving a session that works (ADR-0044, decision 5). Every change
/// that opens a session opens it here.
pub(crate) async fn open_session_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    session: &SignInSession,
) -> Result<(), S::Error> {
    if S::standing_in(transaction, session.person).await?.is_some() {
        return Err(S::aborted());
    }
    S::insert_session(transaction, session).await
}

/// Replaces the session that waits for the second step by `session`, inside a change: a pending
/// session that is gone (signed out, or ended by *Switch off*) stops the change.
pub(super) async fn follow_pending_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    pending: SessionId,
    session: &SignInSession,
) -> Result<(), S::Error> {
    if !S::delete_session(transaction, pending).await? {
        return Err(S::aborted());
    }
    open_session_in::<S>(transaction, session).await
}

/// [`follow_pending_in`] for a second step with a code: a code used up already is a wrong code, but
/// a pending session that is gone or a person switched off meanwhile is no try with a wrong code,
/// so the change stops as `auth.not-signed-in` and the security log does not count it as one.
pub(super) async fn follow_pending_or_signed_out<S: AuthStore>(
    transaction: &mut S::Transaction,
    pending: SessionId,
    session: &SignInSession,
    why: &Why,
) -> Result<(), S::Error> {
    follow_pending_in::<S>(transaction, pending, session)
        .await
        .map_err(|error| {
            if S::is_aborted(&error) {
                why.stop::<S>(ErrorCode::NotSignedIn)
            } else {
                error
            }
        })
}
