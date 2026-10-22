//! The job `mail.send` (NTF-02, PRIV-08, SEC-16, ADR-0033 §3): one e-mail, made when the job runs
//! and checked again as it is made.
//!
//! The payload names who and about what, never a text. When the job runs, e-mail must still be on,
//! the person must still want this kind of e-mail, may still see the issue, and sees only the
//! fields and names they may see now. Changes to one issue that came within five minutes of the
//! first are in the same e-mail (the job runs five minutes after the first of them). Addresses
//! never go into a log (DATA-LOGS): a failure is logged as a class.

use bb_core::ErrorCode;
use bb_core::id::{InvitationId, IssueId, UserId};
use bb_core::mail::{MailSecurity, would_send_secrets_in_the_clear};
use bb_core::notification::Notification;
use bb_core::notification::{MAIL_ENTRIES_MAX, MailState, Via};
use bb_core::person::Person;
use bb_core::settings::{
    Layers, MAIL_ENABLED, MAIL_FROM, MAIL_HOST, MAIL_PORT, MAIL_SECURITY, MAIL_SHORT, MAIL_USERNAME,
};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zeroize::Zeroizing;

use super::compose::compose;
use super::plan::wants;
use super::read::Reading;
use super::types::NotificationItem;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::mailer::{MailFailure, MailMessage, MailServer};
use bb_core::wording::Region;

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
        if !layers.yes(&MAIL_ENABLED) {
            return Ok(None);
        }
        match self.mail_server(layers).await {
            // The settings would send a secret in the clear: nothing goes, and the test e-mail
            // tells the administrator why.
            Err(AppError::Refused(ErrorCode::MailInsecure)) => Ok(None),
            other => other,
        }
    }

    /// The mail server as the settings and the stored password say now, whether or not e-mail is
    /// switched on, and a refusal (`mail.insecure`) for settings that would send a secret in the
    /// clear. `None` while the host or the sender is not entered.
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
        let username = layers.text(&MAIL_USERNAME).trim().to_owned();
        if would_send_secrets_in_the_clear(
            security,
            &host,
            !username.is_empty() || !password.is_empty(),
        ) {
            return Err(AppError::Refused(ErrorCode::MailInsecure));
        }
        Ok(Some(MailServer {
            host,
            port,
            security,
            username,
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
                .finish_mail(recipient, issue, self.now(), MailState::Never)
                .await
                .map_err(|error| internal::<S>(&error))?;
        }
        // A sign-up whose e-mail did not go leaves nothing behind but a link that runs out.
        Ok(())
    }

    /// The items of the e-mail of a person about an issue (or no issue): what waits, checked again
    /// for what the person may see and may want now (§2), and whether more waits than the e-mail
    /// holds. A notification the person does not want or may not see any more is done with: its
    /// e-mail never goes.
    async fn mail_items(
        &self,
        person: &Person,
        issue: Option<IssueId>,
        until: Timestamp,
    ) -> Result<(Vec<NotificationItem>, bool), AppError> {
        let failed = |error| internal::<S>(&error);
        let person_layers = self.setting_layers_of(Some(person.id)).await?;
        let actor = Actor {
            person: person.clone(),
            key: None,
        };
        let mut reading = self.reading_for(&actor).await?;
        loop {
            let waiting = self
                .store()
                .waiting_mail(
                    person.id,
                    issue,
                    until,
                    u64::try_from(MAIL_ENTRIES_MAX + 1).unwrap_or(u64::MAX),
                )
                .await
                .map_err(failed)?;
            let mut items = Vec::new();
            let mut unwanted = Vec::new();
            for row in waiting.iter().take(MAIL_ENTRIES_MAX) {
                match self.mail_item(&person_layers, &mut reading, row).await? {
                    Some(item) => items.push(item),
                    None => unwanted.push(row.id),
                }
            }
            self.store()
                .set_mail_state(&unwanted, MailState::Never)
                .await
                .map_err(failed)?;
            let more = waiting.len() > MAIL_ENTRIES_MAX;
            // Only unwanted ones in this round: the next ones may be wanted.
            if !items.is_empty() || !more {
                return Ok((items, more));
            }
        }
    }

    /// The item of one waiting notification, or none when the person does not want it by e-mail or
    /// may not see it now.
    async fn mail_item(
        &self,
        layers: &Layers,
        reading: &mut Reading<'_>,
        row: &Notification,
    ) -> Result<Option<NotificationItem>, AppError> {
        if wants(layers, row.kind, Via::Mail) {
            self.notification_item(reading, row).await
        } else {
            Ok(None)
        }
    }

    /// Sends the e-mail with the notifications of a person about one issue that wait. What does
    /// not fit in it is in the app: the e-mail says so, and every notification that waited up to
    /// now is done with, so a second e-mail does not repeat the first.
    async fn send_changes(
        &self,
        recipient: UserId,
        issue: Option<IssueId>,
    ) -> Result<(), AppError> {
        let failed = |error| internal::<S>(&error);
        let until = self.now();
        let finish = |state| async move {
            self.store()
                .finish_mail(recipient, issue, until, state)
                .await
                .map(|_| ())
                .map_err(failed)
        };
        let layers = self.setting_layers_of(None).await?;
        let Some(server) = self.mail_server_if_on(&layers).await? else {
            // E-mail is off, or the mail server is not entered: nothing goes (PRIV-08).
            return finish(MailState::Never).await;
        };
        let Some(person) = self.store().person(recipient).await.map_err(failed)? else {
            return finish(MailState::Never).await;
        };
        let (items, more) = self.mail_items(&person, issue, until).await?;
        let composed = compose(
            &items,
            more,
            layers.yes(&MAIL_SHORT),
            &Region::of(&self.setting_layers_of(Some(recipient)).await?),
            &self.services.public_url,
        );
        let Some(composed) = composed else {
            // Nothing is left to say (another job of the same batch sent it, or none is wanted).
            return Ok(());
        };
        let message = MailMessage {
            to: person.email,
            subject: composed.subject,
            text: composed.text,
        };
        match self.services.mailer.send(&server, &message).await {
            Ok(()) => finish(MailState::Sent).await,
            Err(MailFailure::Rejected) => {
                finish(MailState::Never).await?;
                Err(send_failure_for_job(MailFailure::Rejected))
            }
            // Another try may work: the notifications keep waiting, and the job runs again.
            Err(failure) => Err(send_failure_for_job(failure)),
        }
    }
}
