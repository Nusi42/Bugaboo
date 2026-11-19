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
//! signing up cannot fill somebody's inbox): the e-mail goes the way of every link e-mail
//! (`link_mail`). With e-mail off nothing can be sent, so signing up is
//! refused.

use bb_auth::{EmailAddress, Purpose, domain_is_listed};
use bb_core::ErrorCode;
use bb_core::app_paths;
use bb_core::event::{Actor as EventActor, RecordKind};
use bb_core::id::{InvitationId, JobId};
use bb_core::keeping::Step;
use bb_core::message::{self, Message};
use bb_core::security_log::Digest;
use bb_core::sign_in::SecurityAction;
use bb_core::sign_up_invitation::{KEEP_HOURS, MAIL_MEMORY, SignUpInvitation, lasts};
use jiff::{SignedDuration, Timestamp};

use super::ctx::Ctx;
use super::error::{AppError, Stops, change_failed};
use super::events::{changed, entity};
use super::link_mail::{Capped, LinkMail, MailText};
use super::store::AuthStore;
use super::types::{Done, SignUp};
use crate::jobs::JobKind;
use crate::keeping::KEEP_STEP_ROWS;
use crate::notifications::MailJob;
use crate::ports::ChangeStore;
use crate::store_macros::{reads, writes};

/// What signing up needs from storage.
pub trait SignUpStore: ChangeStore + 'static {
    reads! {
        /// How many e-mails with a link went since this moment (the cap of the server per hour).
        fn sign_up_mail_since(since: Timestamp) -> u64;
    }

    writes! {
        /// Says that an e-mail with a link goes to the address with this digest, for the `link` of
        /// a sign-up or of a new address: `true` when none went since `since` (or the one that went
        /// was for this very link, a second try of the same job). It is one statement, so two jobs
        /// for one address cannot both get `true`.
        fn claim_link_mail(
            address: &Digest,
            link: InvitationId,
            since: Timestamp,
            now: Timestamp
        ) -> bool;
        /// Stores a link, replacing the one with the same ID.
        fn put_sign_up_invitation(invitation: &SignUpInvitation) -> ();
        /// Deletes at most `limit` of the links that were made before `before` and are used up or
        /// have run out; returns how many.
        fn delete_old_sign_up_invitations(before: Timestamp, now: Timestamp, limit: u64) -> u64;
        /// Forgets at most `limit` of the e-mails that went before `before`; returns how many. The
        /// memory of an e-mail is needed only for `MAIL_MEMORY` (GDPR: nothing kept without a use).
        fn forget_old_sign_up_mail(before: Timestamp, limit: u64) -> u64;
    }
}

/// Asks for an account with an address (nothing is made until the link is opened).
///
/// # Errors
/// `auth.sign-up-refused` when signing up is off, when the domain of the address is not on the
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

    /// Sends the e-mail of a sign-up (the work of a job of the kind `mail.send`): the link that
    /// makes the account, or, for an address that has an account, the word that it has one. The
    /// link is made now and works for 24 hours from now.
    pub(crate) async fn send_sign_up_mail(
        &self,
        address: &str,
        invitation: InvitationId,
    ) -> Result<(), AppError> {
        // Signing up was switched off (or the domain taken off the list) meanwhile: nothing goes.
        let Some(email) = EmailAddress::parse(address) else {
            return Ok(());
        };
        if !self.sign_up_allows(&email).await? {
            return Ok(());
        }
        let mail = LinkMail {
            address,
            link: invitation,
            lasts: lasts(),
            exists: MailText {
                subject: Message::from(message::MAIL_SIGN_UP_EXISTS_SUBJECT),
                text: message::mail_sign_up_exists_text(self.app_address(app_paths::SIGN_IN)),
            },
            capped: Capped::WhileAnybodyMaySignUp,
        };
        self.send_link_mail(mail, |link| async move {
            let now = self.now();
            let made = SignUpInvitation {
                id: invitation,
                token_hash: link.secret.hash(Purpose::Invitation),
                email: address.to_owned(),
                created_at: now,
                expires_at: link.expires_at,
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
            Ok(MailText {
                subject: Message::from(message::MAIL_SIGN_UP_SUBJECT),
                text: message::mail_sign_up_text(
                    self.app_address(&link.path(app_paths::INVITATION)),
                    bb_core::sign_up_invitation::HOURS,
                ),
            })
        })
        .await
    }

    /// Deletes the links of sign-ups that were used or ran out a day ago, and the memory of the
    /// e-mails that went longer ago than they are needed. Returns how many links went.
    pub(crate) async fn delete_old_sign_up_links(&self) -> Result<Step, AppError> {
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
                    let mail =
                        S::forget_old_sign_up_mail(transaction, resent, KEEP_STEP_ROWS).await?;
                    let links =
                        S::delete_old_sign_up_invitations(transaction, before, now, KEEP_STEP_ROWS)
                            .await?;
                    Ok(Step::of(mail, KEEP_STEP_ROWS).and(Step::of(links, KEEP_STEP_ROWS)))
                })
            })
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
        Ok(deleted)
    }
}
