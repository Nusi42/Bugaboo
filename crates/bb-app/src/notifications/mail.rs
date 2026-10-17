//! The job `mail.send` (NTF-02, PRIV-08, SEC-16, ADR-0033 §3): one e-mail, made when the job runs and
//! checked again as it is made.
//!
//! The payload names who and about what, never a text. When the job runs, e-mail must still be on,
//! the person must still want this kind of e-mail, may still see the issue, and sees only the
//! fields and names they may see now. Changes to one issue that came within five minutes of the
//! first are in the same e-mail (the job runs five minutes after the first of them). Addresses
//! never go into a log (DATA-LOGS): a failure is logged as a class.

use bb_core::ErrorCode;
use bb_core::id::{InvitationId, IssueId, UserId};
use bb_core::mail::MailSecurity;
use bb_core::notification::{MAIL_ENTRIES_MAX, MailState, Via};
use bb_core::settings::{
    Layers, MAIL_ENABLED, MAIL_FROM, MAIL_HOST, MAIL_PORT, MAIL_SECURITY, MAIL_SHORT, MAIL_USERNAME,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zeroize::Zeroizing;

use super::compose::compose;
use super::prefs::wants;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::mailer::{MailFailure, MailMessage, MailServer};

/// What a job of the kind `mail.send` is for. The payload names who and about what, never a text:
/// the e-mail is made when the job runs, from what the person may see then.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "what", rename_all = "snake_case")]
pub(crate) enum MailJob {
    /// The changes to one issue (or the notifications about no issue) that wait for a person.
    Changes {
        /// Who they are for.
        recipient: UserId,
        /// The issue; none for the notifications that are about no issue.
        issue: Option<IssueId>,
    },
    /// The e-mail of somebody who signed up: the link that makes the account, or the word that
    /// the address has an account. The link is made when the job runs, so it is in no payload; the
    /// address is, until the job is done (and with a dead job, for its keep period).
    SignUp {
        /// The address as typed.
        email: String,
        /// The ID the link will have.
        invitation: InvitationId,
    },
}

impl MailJob {
    /// The payload of the job.
    pub(crate) fn to_value(&self) -> Value {
        // A payload of IDs and words always has a JSON form.
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    /// The job a payload stands for.
    pub(crate) fn of_value(payload: &Value) -> Option<Self> {
        serde_json::from_value(payload.clone()).ok()
    }
}

/// What a failure to send says to the one who asked (the test e-mail).
pub(crate) fn send_failure(failure: MailFailure) -> AppError {
    AppError::Refused(match failure {
        MailFailure::Rejected => ErrorCode::MailRejected,
        MailFailure::Unavailable => ErrorCode::MailUnavailable,
    })
}

/// What a failure to send says to the queue: a refusal for good is a refusal, and a mail server
/// that cannot be reached is a failure, so the job is tried again.
pub(crate) fn send_failure_for_job(failure: MailFailure) -> AppError {
    match failure {
        MailFailure::Rejected => send_failure(failure),
        MailFailure::Unavailable => {
            AppError::Internal("the mail server cannot be reached".to_owned())
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The address of a page of the app, from the public address of the server.
    pub(crate) fn app_address(&self, path: &str) -> String {
        format!("{}{path}", self.services.public_url)
    }

    /// The mail server when e-mail is switched on and the server is entered: the only way a job
    /// gets one (PRIV-08).
    pub(crate) async fn mail_server_if_on(
        &self,
        layers: &Layers,
    ) -> Result<Option<MailServer>, AppError> {
        if layers.yes(&MAIL_ENABLED) {
            self.mail_server(layers).await
        } else {
            Ok(None)
        }
    }

    /// The mail server as the settings and the stored password say now, whether or not e-mail is
    /// switched on. `None` while the host or the sender is not entered.
    pub(crate) async fn mail_server(
        &self,
        layers: &Layers,
    ) -> Result<Option<MailServer>, AppError> {
        let host = layers.text(&MAIL_HOST).trim().to_owned();
        let from = layers.text(&MAIL_FROM).trim().to_owned();
        if host.is_empty() || from.is_empty() {
            return Ok(None);
        }
        let Ok(port) = u16::try_from(layers.number(&MAIL_PORT)) else {
            return Ok(None);
        };
        let security =
            MailSecurity::from_code(&layers.text(&MAIL_SECURITY)).unwrap_or(MailSecurity::StartTls);
        let password = self
            .store()
            .mail_password()
            .await
            .map_err(|error| internal::<S>(&error))?
            .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
            .unwrap_or_default();
        Ok(Some(MailServer {
            host,
            port,
            security,
            username: layers.text(&MAIL_USERNAME).trim().to_owned(),
            password: Zeroizing::new(password),
            from,
        }))
    }

    /// Does the work of a job of the kind `mail.send`.
    ///
    /// # Errors
    /// A refusal when the job can never succeed (a payload that names nothing, an address the mail
    /// server turns away for good), an internal error when another try may (the mail server cannot
    /// be reached).
    pub(crate) async fn run_mail_job(&self, payload: &Value) -> Result<(), AppError> {
        match MailJob::of_value(payload) {
            Some(MailJob::Changes { recipient, issue }) => {
                self.send_changes(recipient, issue).await
            }
            Some(MailJob::SignUp { email, invitation }) => {
                self.send_sign_up_mail(&email, invitation).await
            }
            None => Err(AppError::Refused(ErrorCode::InvalidRequest)),
        }
    }

    /// What happens when an e-mail job is on the dead-letter list: no e-mail will go for the
    /// notifications that waited for it.
    pub(crate) async fn mail_job_gave_up(&self, payload: &Value) -> Result<(), AppError> {
        if let Some(MailJob::Changes { recipient, issue }) = MailJob::of_value(payload) {
            self.store()
                .give_up_mail(recipient, issue)
                .await
                .map_err(|error| internal::<S>(&error))?;
        }
        // A sign-up whose e-mail did not go leaves nothing behind but a link that runs out.
        Ok(())
    }

    /// Sends the e-mail with the notifications of a person about one issue that wait.
    async fn send_changes(
        &self,
        recipient: UserId,
        issue: Option<IssueId>,
    ) -> Result<(), AppError> {
        let failed = |error| internal::<S>(&error);
        let layers = self.setting_layers_of(None).await?;
        let Some(server) = self.mail_server_if_on(&layers).await? else {
            // E-mail is off, or the mail server is not entered: nothing goes (PRIV-08).
            self.store()
                .give_up_mail(recipient, issue)
                .await
                .map_err(failed)?;
            return Ok(());
        };
        let waiting = self
            .store()
            .waiting_mail(
                recipient,
                issue,
                self.now(),
                u64::try_from(MAIL_ENTRIES_MAX + 1).unwrap_or(u64::MAX),
            )
            .await
            .map_err(failed)?;
        if waiting.is_empty() {
            // Another job of the same batch sent them.
            return Ok(());
        }
        let ids: Vec<_> = waiting.iter().map(|row| row.id).collect();
        let Some(person) = self.store().person(recipient).await.map_err(failed)? else {
            self.store()
                .set_mail_state(&ids, MailState::Never)
                .await
                .map_err(failed)?;
            return Ok(());
        };
        // Checked again: what the person may see and may want now (§2).
        let person_layers = self.setting_layers_of(Some(recipient)).await?;
        let actor = Actor {
            person: person.clone(),
            key: None,
        };
        let mut reading = self.reading_for(&actor).await?;
        let mut items = Vec::new();
        for row in waiting.iter().take(MAIL_ENTRIES_MAX) {
            if wants(&person_layers, row.kind, Via::Mail)
                && let Some(item) = self.notification_item(&mut reading, row).await?
            {
                items.push(item);
            }
        }
        let composed = compose(&items, layers.yes(&MAIL_SHORT), &self.services.public_url);
        let Some(composed) = composed else {
            self.store()
                .set_mail_state(&ids, MailState::Never)
                .await
                .map_err(failed)?;
            return Ok(());
        };
        let message = MailMessage {
            to: person.email,
            subject: composed.subject,
            text: composed.text,
        };
        match self.services.mailer.send(&server, &message).await {
            Ok(()) => {
                self.store()
                    .set_mail_state(&ids, MailState::Sent)
                    .await
                    .map_err(failed)?;
                Ok(())
            }
            Err(MailFailure::Rejected) => {
                self.store()
                    .set_mail_state(&ids, MailState::Never)
                    .await
                    .map_err(failed)?;
                Err(send_failure_for_job(MailFailure::Rejected))
            }
            // Another try may work: the notifications keep waiting, and the job runs again.
            Err(failure) => Err(send_failure_for_job(failure)),
        }
    }
}
