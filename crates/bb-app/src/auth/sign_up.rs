//! Signing up on one's own (AUTH-06, M1-F38, ADR-0033 §3): a self-invitation.
//!
//! The person gives only an address. Nothing is stored for the address but a job: no person, no
//! password, no row that names it. When the job runs it makes a link like the one an administrator
//! makes for an invitation (`SignUpInvitation`, 24 hours, used once) and mails it; opening the link
//! goes through the invitation page and `accept_invitation`, where the person chooses a name and a
//! password and the account is made. So nobody can make an account on an address they cannot read,
//! and there is one way to join.
//!
//! The answer is the same for every address. An address that has an account gets an e-mail that
//! says so instead of a link, and an address that was sent a link a moment ago gets none (so
//! signing up cannot fill somebody's inbox). With e-mail off nothing can be sent, so signing up is
//! refused.

use bb_auth::{EmailAddress, Purpose, SecretToken, address_digest, domain_is_listed};
use bb_core::ErrorCode;
use bb_core::app_paths;
use bb_core::event::{Actor as EventActor, RecordKind};
use bb_core::id::{InvitationId, JobId};
use bb_core::message::{self, Message};
use bb_core::security_log::Digest;
use bb_core::sign_in::SecurityAction;
use bb_core::sign_up_invitation::{
    ADDRESSES_PER_HOUR_MAX, KEEP_HOURS, MAIL_MEMORY, MAILS_CAP_WINDOW, RESEND_AFTER_MINUTES,
    SignUpInvitation, lasts,
};
use jiff::{SignedDuration, Timestamp};

use super::ctx::Ctx;
use super::error::{AppError, Stops, change_failed, internal};
use super::events::{changed, entity};
use super::store::AuthStore;
use super::types::{Done, SignUp};
use crate::jobs::JobKind;
use crate::mailer::MailMessage;
use crate::notifications::{MailJob, send_failure_for_job};
use crate::ports::ChangeStore;
use crate::store_macros::{reads, writes};

/// What signing up needs from storage.
pub trait SignUpStore: ChangeStore + 'static {
    reads! {
        /// How many e-mails went for sign-ups since this moment (the cap of the server per hour).
        fn sign_up_mail_since(since: Timestamp) -> u64;
    }

    writes! {
        /// Says that an e-mail goes to the address with this digest for the sign-up `invitation`:
        /// `true` when none went since `since` (or the one that went was for this very sign-up, a
        /// second try of the same job). It is one statement, so two jobs for one address cannot
        /// both get `true`.
        fn claim_sign_up_mail(
            address: &Digest,
            invitation: InvitationId,
            since: Timestamp,
            now: Timestamp
        ) -> bool;
        /// Stores a link, replacing the one with the same ID.
        fn put_sign_up_invitation(invitation: &SignUpInvitation) -> ();
        /// Deletes the links that were made before `before` and are used up or have run out;
        /// returns how many.
        fn delete_old_sign_up_invitations(before: Timestamp, now: Timestamp) -> u64;
        /// Forgets the e-mails that went before `before`; returns how many. The memory of an
        /// e-mail is needed only for `MAIL_MEMORY` (GDPR: nothing kept without a use).
        fn forget_old_sign_up_mail(before: Timestamp) -> u64;
    }
}

/// Asks for an account with an address (nothing is made until the link is opened).
///
/// # Errors
/// `auth.sign_up_refused` when signing up is off, when the domain of the address is not on the
/// list, and when e-mail is off (so no link can be sent); `request.invalid` for an address that
/// cannot be one. Whether the address has an account is never told.
pub async fn sign_up<S: AuthStore>(ctx: &Ctx<'_, S>, input: SignUp) -> Result<Done, AppError> {
    let refused = || AppError::Refused(ErrorCode::SignUpRefused);
    let email = EmailAddress::parse(&input.email).ok_or(ErrorCode::InvalidRequest)?;
    if !ctx.sign_up_allows(&email).await? || !ctx.can_send_mail().await? {
        ctx.note(SecurityAction::SignUpRefused, None, None).await;
        return Err(refused());
    }
    let invitation = InvitationId::generate(&mut ctx.ids());
    let job = (
        JobId::generate(&mut ctx.ids()).map_err(|error| AppError::Internal(error.to_string()))?,
        JobKind::MailSend.name()?,
        MailJob::SignUp {
            email: email.as_str().to_owned(),
            invitation,
        }
        .to_value(),
    );
    // The change names no address: the address is only in the job, until the job has run.
    let event = ctx.event(
        EventActor::System,
        entity(RecordKind::Invitation, invitation),
        vec![changed("sign-up")],
    )?;
    let (store, now) = (ctx.store(), ctx.now());
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                let (id, kind, payload) = &job;
                S::enqueue_job(transaction, *id, kind, payload, now, now).await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(SecurityAction::SignUpAccepted, None, None).await;
    Ok(Done)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether an e-mail can go: e-mail is on and the mail server is entered.
    pub(crate) async fn can_send_mail(&self) -> Result<bool, AppError> {
        let layers = self.setting_layers_of(None).await?;
        Ok(self.mail_server_if_on(&layers).await?.is_some())
    }

    /// Whether signing up is on for this address: the switch is on and its domain is on the list.
    /// A link that is out is judged by this again when it is used and when its e-mail is made, so
    /// switching signing up off stops the links that are out.
    pub(crate) async fn sign_up_allows(&self, email: &EmailAddress) -> Result<bool, AppError> {
        let settings = self.settings().await?;
        let listed = settings.sign_up_domains.trim().is_empty()
            || domain_is_listed(email, &settings.sign_up_domains);
        Ok(settings.sign_up_enabled && listed)
    }

    /// How many addresses a sign-up e-mail went to in the window of the cap, while the cap counts:
    /// only while anybody may sign up. With a list of approved domains the people who may get an
    /// e-mail are the organization's own, and a cap would turn them away.
    async fn addresses_in_the_cap_window(&self) -> Result<Option<u64>, AppError> {
        if !self.settings().await?.sign_up_domains.trim().is_empty() {
            return Ok(None);
        }
        let since = self
            .now()
            .checked_sub(MAILS_CAP_WINDOW)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        self.store()
            .sign_up_mail_since(since)
            .await
            .map(Some)
            .map_err(|error| internal::<S>(&error))
    }

    /// Takes the one e-mail an address gets in `RESEND_AFTER_MINUTES` minutes, for the sign-up
    /// `invitation`. `false` when one went already.
    async fn claim_sign_up_mail(
        &self,
        address: &str,
        invitation: InvitationId,
    ) -> Result<bool, AppError> {
        let now = self.now();
        let since = now
            .checked_sub(SignedDuration::from_mins(RESEND_AFTER_MINUTES))
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let digest = address_digest(address);
        let forget_before = now
            .checked_sub(MAIL_MEMORY)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let event = self.event(
            EventActor::System,
            entity(RecordKind::Invitation, invitation),
            vec![changed("sign-up-mail")],
        )?;
        self.store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    // What is no use any more goes with the next e-mail, not only with the daily job.
                    S::forget_old_sign_up_mail(transaction, forget_before).await?;
                    S::claim_sign_up_mail(transaction, &digest, invitation, since, now).await
                })
            })
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))
    }

    /// Sends the e-mail of a sign-up (the work of a job of the kind `mail.send`): the link that
    /// makes the account, or, for an address that has an account, the word that it has one. The
    /// link is made now and works for 24 hours from now.
    pub(crate) async fn send_sign_up_mail(
        &self,
        address: &str,
        invitation: InvitationId,
    ) -> Result<(), AppError> {
        let failed = |error| internal::<S>(&error);
        let layers = self.setting_layers_of(None).await?;
        let Some(server) = self.mail_server_if_on(&layers).await? else {
            // E-mail was switched off meanwhile: nothing goes (PRIV-08).
            return Ok(());
        };
        // Signing up was switched off (or the domain taken off the list) meanwhile: nothing goes.
        let Some(email) = EmailAddress::parse(address) else {
            return Ok(());
        };
        if !self.sign_up_allows(&email).await? {
            return Ok(());
        }
        let sent = self.addresses_in_the_cap_window().await?;
        if sent.is_some_and(|sent| sent >= ADDRESSES_PER_HOUR_MAX) {
            // The server has sent to as many addresses this hour as it will: this one is dropped.
            return Ok(());
        }
        if !self.claim_sign_up_mail(address, invitation).await? {
            // An e-mail went to this address a moment ago: signing up cannot fill the inbox.
            return Ok(());
        }
        if sent.is_some_and(|sent| sent + 1 >= ADDRESSES_PER_HOUR_MAX) {
            // Written once, when the cap is reached, and not for every e-mail turned away.
            self.note(SecurityAction::SignUpMailCapped, None, None)
                .await;
        }
        let has_account = self
            .store()
            .account_by_email(address)
            .await
            .map_err(failed)?
            .is_some();
        let message = if has_account {
            MailMessage {
                to: address.to_owned(),
                subject: Message::from(message::MAIL_SIGN_UP_EXISTS_SUBJECT).render(),
                text: message::mail_sign_up_exists_text(self.app_address(app_paths::SIGN_IN))
                    .render(),
            }
        } else {
            let now = self.now();
            let secret = SecretToken::generate()?;
            let made = SignUpInvitation {
                id: invitation,
                token_hash: secret.hash(Purpose::Invitation),
                email: address.to_owned(),
                created_at: now,
                expires_at: now
                    .checked_add(lasts())
                    .map_err(|error| AppError::Internal(error.to_string()))?,
                used_at: None,
            };
            let event = self.event(
                EventActor::System,
                entity(RecordKind::Invitation, invitation),
                vec![changed("sign-up-link")],
            )?;
            self.store()
                .record_change(&event, move |transaction| {
                    Box::pin(async move { S::put_sign_up_invitation(transaction, &made).await })
                })
                .await
                .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
            let link = format!("{}#{}", app_paths::INVITATION, secret.encode());
            MailMessage {
                to: address.to_owned(),
                subject: Message::from(message::MAIL_SIGN_UP_SUBJECT).render(),
                text: message::mail_sign_up_text(
                    self.app_address(&link),
                    bb_core::sign_up_invitation::HOURS,
                )
                .render(),
            }
        };
        self.services
            .mailer
            .send(&server, &message)
            .await
            .map_err(send_failure_for_job)
    }

    /// Deletes the links of sign-ups that were used or ran out a day ago, and the memory of the
    /// e-mails that went longer ago than they are needed. Returns how many links went.
    pub(crate) async fn delete_old_sign_up_links(&self) -> Result<u64, AppError> {
        let now = self.now();
        let before = now
            .checked_sub(SignedDuration::from_hours(KEEP_HOURS))
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let resent = now
            .checked_sub(MAIL_MEMORY)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let event = self.event(
            EventActor::System,
            entity(
                RecordKind::Invitation,
                InvitationId::generate(&mut self.ids()),
            ),
            vec![changed("old-sign-up-links")],
        )?;
        let deleted = self
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    S::forget_old_sign_up_mail(transaction, resent).await?;
                    S::delete_old_sign_up_invitations(transaction, before, now).await
                })
            })
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
        Ok(deleted)
    }
}
