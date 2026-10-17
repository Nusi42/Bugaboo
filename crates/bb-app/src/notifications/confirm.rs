//! Confirming an e-mail address (M1-F38, M4-Q10, ADR-0033 §3): once e-mail is on, an account made by
//! signing up on one's own works only after the person has opened the link sent to the address.
//!
//! The account waits with a row in `address_confirmation`. The link is made when the e-mail is sent
//! (the job `mail.send`), works for 24 hours, and is used up by the one who opens it. Until then
//! the person cannot sign in; after the 24 hours the account is removed by the job
//! `sessions.purge`, or replaced when somebody signs up with the address again, so nobody can keep
//! an address by signing up for it. Accounts made by an invitation or at the first start have no
//! such row: the invitation went to the address already. With e-mail off, signing up works as it
//! did and the security chapter keeps the accepted risk that the address is not confirmed.

use bb_auth::{Purpose, SecretToken};
use bb_core::ErrorCode;
use bb_core::app_paths;
use bb_core::confirmation::{AddressConfirmation, lasts};
use bb_core::event::{Actor as EventActor, RecordKind};
use bb_core::id::UserId;
use bb_core::message::{self, Message};
use bb_core::security_log::Digest;
use bb_core::settings::MAIL_ENABLED;
use bb_core::sign_in::SecurityAction;
use jiff::Timestamp;

use super::prefs::yes;
use crate::auth::{
    AccountStore, AppError, AuthStore, Ctx, Done, Stops, by, change_failed, changed, entity,
    internal,
};
use crate::mailer::{MailFailure, MailMessage};
use crate::store_macros::{reads, writes};

use super::mail::send_failure;

/// How many accounts one round of removing the unconfirmed ones reads.
const STALE_AT_ONCE: u64 = 100;

/// What confirming an address needs from storage.
pub trait ConfirmationStore: AccountStore + 'static {
    reads! {
        /// The confirmation with the link whose secret has this hash.
        fn address_confirmation_by_hash(hash: &Digest) -> Option<AddressConfirmation>;
        /// The confirmation an account waits for, if it waits for one.
        fn unconfirmed(person: UserId) -> Option<AddressConfirmation>;
        /// Gives the confirmation of a person a new link: its hash and the moment it runs out.
        /// `false` when the person waits for none. Bookkeeping of the job that sends the link.
        fn renew_confirmation(person: UserId, hash: &Digest, expires_at: Timestamp) -> bool;
        /// The accounts whose time to confirm ran out before `before`, at most `limit`.
        fn stale_sign_ups(before: Timestamp, limit: u64) -> Vec<UserId>;
    }

    writes! {
        /// Makes the account of a person wait for its address to be confirmed.
        fn put_address_confirmation(confirmation: &AddressConfirmation) -> ();
        /// Uses up the confirmation with this hash if its time has not run out; `false` otherwise.
        fn use_address_confirmation(hash: &Digest, now: Timestamp) -> bool;
        /// Deletes the account of a person who waits for a confirmation, with all that belongs to
        /// it; `false` when the person waits for none (an account that works is never deleted).
        fn delete_unconfirmed_person(person: UserId) -> bool;
    }
}

/// Confirm the address of an account with the secret of the link.
pub struct ConfirmAddress {
    /// The secret after the `#` of the link.
    pub secret: String,
}

/// An account that waits for its address, as it is first stored: the link is made later, when the
/// e-mail is sent, so the hash is one that no secret has.
///
/// # Errors
/// Fails when the system gives no secure random bytes.
pub(crate) fn waiting_account(
    person: UserId,
    now: Timestamp,
) -> Result<AddressConfirmation, AppError> {
    let expires_at = now
        .checked_add(lasts())
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(AddressConfirmation {
        person,
        token_hash: SecretToken::generate()?.hash(Purpose::AddressConfirmation),
        created_at: now,
        expires_at,
    })
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether an account made by signing up must be confirmed: e-mail is on and the mail server
    /// is entered, so the link can go.
    pub(crate) async fn confirming_sign_ups(&self) -> Result<bool, AppError> {
        let layers = self.setting_layers_of(None).await?;
        if !yes(&layers, &MAIL_ENABLED) {
            return Ok(false);
        }
        Ok(self.mail_server(&layers).await?.is_some())
    }

    /// The account with this address that waited for its confirmation and ran out of time, if
    /// there is one: signing up with the address replaces it.
    pub(crate) async fn stale_sign_up_of(&self, address: &str) -> Result<Option<UserId>, AppError> {
        let failed = |error| internal::<S>(&error);
        let Some(account) = self
            .store()
            .account_by_email(address)
            .await
            .map_err(failed)?
        else {
            return Ok(None);
        };
        let waiting = self
            .store()
            .unconfirmed(account.person.id)
            .await
            .map_err(failed)?;
        Ok(waiting
            .filter(|waiting| waiting.expires_at <= self.now())
            .map(|_| account.person.id))
    }

    /// Sends the e-mail with the link that confirms the address of an account that waits (the work
    /// of a job of the kind `mail.send`). The link is made now and works for 24 hours from now.
    pub(crate) async fn send_confirmation(&self, person: UserId) -> Result<(), AppError> {
        let failed = |error| internal::<S>(&error);
        if self
            .store()
            .unconfirmed(person)
            .await
            .map_err(failed)?
            .is_none()
        {
            // Confirmed, replaced or removed meanwhile.
            return Ok(());
        }
        let layers = self.setting_layers_of(None).await?;
        let Some(server) = self.mail_server(&layers).await? else {
            return Ok(());
        };
        let Some(account) = self.store().person(person).await.map_err(failed)? else {
            return Ok(());
        };
        let secret = SecretToken::generate()?;
        let expires_at = self
            .now()
            .checked_add(lasts())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let renewed = self
            .store()
            .renew_confirmation(
                person,
                &secret.hash(Purpose::AddressConfirmation),
                expires_at,
            )
            .await
            .map_err(failed)?;
        if !renewed {
            return Ok(());
        }
        let address = format!(
            "{}{}#{}",
            self.services.public_url,
            app_paths::CONFIRM_ADDRESS,
            secret.encode()
        );
        let message = MailMessage {
            to: account.email,
            subject: Message::from(message::MAIL_CONFIRM_SUBJECT).render(),
            text: message::mail_confirm_text(address).render(),
        };
        match self.services.mailer.send(&server, &message).await {
            Ok(()) => Ok(()),
            Err(MailFailure::Rejected) => Err(send_failure(MailFailure::Rejected)),
            Err(MailFailure::Unavailable) => Err(AppError::Internal(
                "the mail server cannot be reached".to_owned(),
            )),
        }
    }

    /// Removes the accounts that waited for their confirmation and ran out of time. Returns how
    /// many went.
    pub(crate) async fn remove_stale_sign_ups(&self) -> Result<u64, AppError> {
        let mut removed = 0_u64;
        loop {
            let stale = self
                .store()
                .stale_sign_ups(self.now(), STALE_AT_ONCE)
                .await
                .map_err(|error| internal::<S>(&error))?;
            if stale.is_empty() {
                return Ok(removed);
            }
            for person in stale {
                let event = self.event(
                    EventActor::System,
                    entity(RecordKind::Person, person),
                    vec![changed("unconfirmed-account")],
                )?;
                self.store()
                    .record_change(&event, move |transaction| {
                        Box::pin(async move {
                            S::delete_unconfirmed_person(transaction, person)
                                .await
                                .map(|_| ())
                        })
                    })
                    .await
                    .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
                removed += 1;
            }
        }
    }
}

/// Confirms the address of an account with the secret of the link in the e-mail. The link is used
/// up in the same change, so it works once, and the account can be signed in to from then on.
///
/// # Errors
/// `auth.confirmation_invalid` for a link that is unknown, used or run out (it does not say which).
pub async fn confirm_address<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ConfirmAddress,
) -> Result<Done, AppError> {
    let invalid = || AppError::Refused(ErrorCode::ConfirmationInvalid);
    let now = ctx.now();
    let secret = SecretToken::parse(&input.secret);
    let hash = secret
        .as_ref()
        .map(|secret| secret.hash(Purpose::AddressConfirmation));
    let found = match &hash {
        Some(hash) => ctx
            .store()
            .address_confirmation_by_hash(hash)
            .await
            .map_err(|error| internal::<S>(&error))?,
        None => None,
    };
    let (Some(found), Some(hash)) = (found.filter(|found| found.expires_at > now), hash) else {
        ctx.note(SecurityAction::AddressConfirmationRefused, None, None)
            .await;
        return Err(invalid());
    };
    let person = found.person;
    let event = ctx.event(
        by(person),
        entity(RecordKind::Person, person),
        vec![changed("address-confirmed")],
    )?;
    let result = ctx
        .store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if !S::use_address_confirmation(transaction, &hash, now).await? {
                    return Err(S::aborted());
                }
                Ok(())
            })
        })
        .await;
    if let Err(error) = result {
        let error = change_failed::<S>(&error, Stops::both(ErrorCode::ConfirmationInvalid));
        if matches!(error, AppError::Refused(_)) {
            ctx.note(SecurityAction::AddressConfirmationRefused, None, None)
                .await;
        }
        return Err(error);
    }
    ctx.note(
        SecurityAction::AddressConfirmed,
        Some(person),
        Some(entity(RecordKind::Person, person)),
    )
    .await;
    Ok(Done)
}
