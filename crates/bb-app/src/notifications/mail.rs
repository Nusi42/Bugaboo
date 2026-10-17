//! The job `mail.send` (NTF-02, PRIV-08, SEC-16, ADR-0033 §3): one e-mail, made when the job runs and
//! checked again as it is made.
//!
//! The payload names who and about what, never a text. When the job runs, e-mail must still be on,
//! the person must still want this kind of e-mail, may still see the issue, and sees only the
//! fields and names they may see now. Changes to one issue that came within five minutes of the
//! first are in the same e-mail (the job runs five minutes after the first of them). Addresses
//! never go into a log (DATA-LOGS): a failure is logged as a class.

use bb_core::ErrorCode;
use bb_core::id::{IssueId, UserId};
use bb_core::mail::MailSecurity;
use bb_core::notification::{MAIL_ENTRIES_MAX, MailState, Via};
use bb_core::settings::{
    Layers, MAIL_ENABLED, MAIL_FROM, MAIL_HOST, MAIL_PORT, MAIL_SECURITY, MAIL_SHORT, MAIL_USERNAME,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zeroize::Zeroizing;

use super::compose::compose;
use super::prefs::{number, text, wants, yes};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::mailer::{MailFailure, MailMessage, MailServer};

/// What a job of the kind `mail.send` is for. The payload names who and about what, never a text:
/// the e-mail is made when the job runs, from what the person may see then.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "what", rename_all = "snake_case")]
pub(crate) enum MailJob {
    /// The changes to one issue (or the notifications about no issue) that wait for a person.
    Changes {
        /// Who they are for.
        recipient: UserId,
        /// The issue; none for the notifications that are about no issue.
        issue: Option<IssueId>,
    },
    /// The e-mail with the link that confirms the address of an account made by signing up
    /// (M1-F38). The link is made when the job runs, so it is in no payload.
    Confirm {
        /// The person whose address waits.
        person: UserId,
    },
}

impl MailJob {
    /// The payload of the job.
    pub(crate) fn to_value(self) -> Value {
        // A payload of IDs and words always has a JSON form.
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    /// The job a payload stands for.
    pub(crate) fn of_value(payload: &Value) -> Option<Self> {
        serde_json::from_value(payload.clone()).ok()
    }
}

/// What a failure to send says to the one who asked (the test e-mail) and to the queue.
pub(crate) fn send_failure(failure: MailFailure) -> AppError {
    AppError::Refused(match failure {
        MailFailure::Rejected => ErrorCode::MailRejected,
        MailFailure::Unavailable => ErrorCode::MailUnavailable,
    })
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The mail server as the settings and the stored password say now, whether or not e-mail is
    /// switched on. `None` while the host or the sender is not entered.
    pub(crate) async fn mail_server(
        &self,
        layers: &Layers,
    ) -> Result<Option<MailServer>, AppError> {
        let (Some(host), Some(from)) = (text(layers, &MAIL_HOST), text(layers, &MAIL_FROM)) else {
            return Ok(None);
        };
        let (host, from) = (host.trim().to_owned(), from.trim().to_owned());
        if host.is_empty() || from.is_empty() {
            return Ok(None);
        }
        let port = number(layers, &MAIL_PORT).and_then(|port| u16::try_from(port).ok());
        let Some(port) = port else {
            return Ok(None);
        };
        let security = text(layers, &MAIL_SECURITY)
            .and_then(|word| MailSecurity::from_code(&word))
            .unwrap_or(MailSecurity::StartTls);
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
            username: text(layers, &MAIL_USERNAME)
                .unwrap_or_default()
                .trim()
                .to_owned(),
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
            Some(MailJob::Confirm { person }) => self.send_confirmation(person).await,
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
        // A confirmation that did not go leaves the account waiting until its time runs out.
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
        let server = if yes(&layers, &MAIL_ENABLED) {
            self.mail_server(&layers).await?
        } else {
            None
        };
        let Some(server) = server else {
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
        let composed = compose(&items, yes(&layers, &MAIL_SHORT), &self.services.public_url);
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
                Err(send_failure(MailFailure::Rejected))
            }
            // Another try may work: the notifications keep waiting, and the job runs again.
            Err(MailFailure::Unavailable) => Err(AppError::Internal(
                "the mail server cannot be reached".to_owned(),
            )),
        }
    }
}
