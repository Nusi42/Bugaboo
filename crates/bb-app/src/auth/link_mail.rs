//! The e-mail that carries a link of one use to an address (AUTH-06, PRIV-06): the link of a sign-up
//! and the link to a new e-mail address go the same way.
//!
//! An address gets at most one such e-mail in `RESEND_AFTER_MINUTES` minutes, and the server sends
//! to at most `ADDRESSES_PER_HOUR_MAX` addresses an hour (for sign-ups only while anybody may sign
//! up; for new addresses always), so neither way can fill somebody's inbox. The link is stored whether the address has an account or not, so a
//! taken address leaves the same trace as a free one; only a free one is sent the link, and an
//! address with an account gets the word that it has one instead.

use bb_core::event::{Actor as EventActor, RecordKind};
use bb_core::id::InvitationId;
use bb_core::message::Message;
use bb_core::sign_in::SecurityAction;
use bb_core::sign_up_invitation::{
    ADDRESSES_PER_HOUR_MAX, MAIL_MEMORY, MAILS_CAP_WINDOW, RESEND_AFTER_MINUTES,
};
use jiff::SignedDuration;

use super::ctx::Ctx;
use super::error::{AppError, Stops, change_failed, internal};
use super::events::{changed, entity};
use super::one_use_link::OneUseLink;
use super::store::AuthStore;
use crate::keeping::KEEP_STEP_ROWS;
use crate::mailer::MailMessage;
use crate::notifications::send_failure_for_job;

/// The subject and the text of an e-mail, before they are written out.
pub(crate) struct MailText {
    /// The subject.
    pub(crate) subject: Message,
    /// The text.
    pub(crate) text: Message,
}

/// When the cap of the server on the addresses it writes to in an hour counts for a link e-mail.
#[derive(Clone, Copy)]
pub(crate) enum Capped {
    /// While anybody may sign up: with a list of approved domains the people who may get a sign-up
    /// e-mail are the organization's own, and a cap would turn them away.
    WhileAnybodyMaySignUp,
    /// Always: a new address can be anywhere.
    Always,
}

/// An e-mail with a link of one use to an address.
pub(crate) struct LinkMail<'a> {
    /// Where it goes.
    pub(crate) address: &'a str,
    /// Its line in the memory of link e-mails: a second try of the same job sends again.
    pub(crate) link: InvitationId,
    /// How long the link works.
    pub(crate) lasts: SignedDuration,
    /// The e-mail an address with an account gets instead of the link.
    pub(crate) exists: MailText,
    /// When the cap of the server counts for it.
    pub(crate) capped: Capped,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// How many addresses a link e-mail went to in the window of the cap, while the cap counts
    /// (`capped`); none while it does not.
    async fn addresses_in_the_cap_window(&self, capped: Capped) -> Result<Option<u64>, AppError> {
        if matches!(capped, Capped::WhileAnybodyMaySignUp)
            && !self.settings().await?.sign_up_domains.trim().is_empty()
        {
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

    /// Takes the one link e-mail an address gets in `RESEND_AFTER_MINUTES` minutes, within the cap
    /// of the server. `false` when one went to the address a moment ago or the cap is reached: then
    /// nothing goes.
    async fn claim_link_mail(
        &self,
        address: &str,
        link: InvitationId,
        capped: Capped,
    ) -> Result<bool, AppError> {
        let sent = self.addresses_in_the_cap_window(capped).await?;
        if sent.is_some_and(|sent| sent >= ADDRESSES_PER_HOUR_MAX) {
            return Ok(false);
        }
        let now = self.now();
        let since = now
            .checked_sub(SignedDuration::from_mins(RESEND_AFTER_MINUTES))
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let digest = self.services.digests.address(address)?;
        let forget_before = now
            .checked_sub(MAIL_MEMORY)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let event = self.event(
            EventActor::System,
            entity(RecordKind::Invitation, link),
            vec![changed("link-mail")],
        )?;
        let claimed = self
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    // What is no use any more goes with the next e-mail, not only with the daily job.
                    S::forget_old_sign_up_mail(transaction, forget_before, KEEP_STEP_ROWS).await?;
                    S::claim_link_mail(transaction, &digest, link, since, now).await
                })
            })
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
        if claimed && sent.is_some_and(|sent| sent + 1 >= ADDRESSES_PER_HOUR_MAX) {
            // Written once, when the cap is reached, and not for every e-mail turned away.
            self.note(SecurityAction::SignUpMailCapped, None, None)
                .await;
        }
        Ok(claimed)
    }

    /// Sends an e-mail with a link of one use (the work of a job of the kind `mail.send`). `store`
    /// keeps the new link and answers the e-mail that carries it; it runs for every address, and
    /// an address with an account gets `exists` instead. Nothing goes while e-mail is off, or when
    /// the address had a link e-mail a moment ago.
    pub(crate) async fn send_link_mail<Store, Stored>(
        &self,
        mail: LinkMail<'_>,
        store: Store,
    ) -> Result<(), AppError>
    where
        Store: FnOnce(OneUseLink) -> Stored,
        Stored: Future<Output = Result<MailText, AppError>>,
    {
        let layers = self.setting_layers_of(None).await?;
        let Some(server) = self.mail_server_if_on(&layers).await? else {
            // E-mail was switched off meanwhile: nothing goes (PRIV-08).
            return Ok(());
        };
        if !self
            .claim_link_mail(mail.address, mail.link, mail.capped)
            .await?
        {
            return Ok(());
        }
        let with_link = store(OneUseLink::new(self.now(), mail.lasts)?).await?;
        let has_account = self
            .store()
            .account_by_email(mail.address)
            .await
            .map_err(|error| internal::<S>(&error))?
            .is_some();
        let chosen = if has_account { mail.exists } else { with_link };
        let message = MailMessage {
            to: mail.address.to_owned(),
            subject: chosen.subject.render(),
            text: chosen.text.render(),
        };
        self.services
            .mailer
            .send(&server, &message)
            .await
            .map_err(send_failure_for_job)
    }
}
