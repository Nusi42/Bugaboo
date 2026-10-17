//! The mail server of the organization (NTF-02, PRIV-08, SEC-16, ADR-0033 §3): its password, which
//! is stored encrypted and is no setting, and *Send me a test e-mail*.
//!
//! The rest of the mail server is in the settings registry (`mail.*`, changed like any setting by
//! whoever holds *Change organization settings*). E-mail stays off until an administrator has
//! entered the server and switched it on.

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::mail::{MailServerReadiness, PASSWORD_MAX};
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Masked, Permission, Place};
use serde::Serialize;

use super::mail::send_failure;
use crate::auth::{
    AppError, AuthStore, Ctx, Done, Stops, by, change_failed, changed, entity, fixed_entity,
    internal,
};
use crate::mailer::MailMessage;
use bb_core::message::{self, Message};

/// Enter, change or remove the password of the mail server.
pub struct SetMailPassword {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The password; none (or an empty one) removes it, for a mail server that asks for none.
    pub password: Option<String>,
}

/// Ask whether the password of the mail server is entered.
pub struct MailServerQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// Whether a password is entered. The password itself is never given (SEC-16): the page of the mail
/// server shows only that it is *set*.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct MailServerStatus {
    /// A password is stored.
    pub credential_set: bool,
    /// The server's own judgement of the mail server as entered: the page shows it and judges
    /// nothing itself.
    pub readiness: MailServerReadiness,
}

impl Masked for MailServerStatus {}

/// Send an e-mail to the person who asks, through the mail server as it is entered, to try the
/// settings. It goes even while e-mail is still off.
pub struct SendTestEmail {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// The record of the mail server in the events: one fixed record.
const MAIL_RECORD: &str = "mail";

/// Enters, changes or removes the password of the mail server. It is stored encrypted like other
/// secrets (SEC-16), the event says only that it changed, and the security log keeps the fact.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*, `request.invalid` for a password that
/// is too long.
pub async fn set_mail_password<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SetMailPassword,
) -> Result<Authorized<Done>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let password = input.password.filter(|password| !password.is_empty());
    if password
        .as_ref()
        .is_some_and(|password| password.len() > PASSWORD_MAX)
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let failed = |error| internal::<S>(&error);
    match &password {
        Some(password) => ctx
            .store()
            .put_mail_password(password.as_bytes(), ctx.now())
            .await
            .map_err(failed)?,
        None => ctx.store().delete_mail_password().await.map_err(failed)?,
    }
    let event = ctx.event(
        by(actor.person.id),
        fixed_entity(RecordKind::Setting, MAIL_RECORD)?,
        vec![changed("mail.password")],
    )?;
    // The event is the change itself: the secret was written above, and its value is in no event.
    ctx.store()
        .record_change(&event, |_| Box::pin(async { Ok(()) }))
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(
        SecurityAction::MailPasswordChanged,
        Some(actor.person.id),
        Some(entity(RecordKind::Setting, event.entity.id)),
    )
    .await;
    Ok(access.authorize(Done))
}

/// Tells whether the password of the mail server is entered.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*.
pub async fn mail_server_status<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MailServerQuery,
) -> Result<Authorized<MailServerStatus>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let credential_set = ctx
        .store()
        .mail_password()
        .await
        .map_err(|error| internal::<S>(&error))?
        .is_some();
    let layers = ctx.setting_layers_of(None).await?;
    let readiness = match ctx.mail_server(&layers).await {
        Ok(Some(_)) => MailServerReadiness::Ready,
        Ok(None) => MailServerReadiness::Incomplete,
        Err(AppError::Refused(ErrorCode::MailInsecure)) => MailServerReadiness::Insecure,
        Err(other) => return Err(other),
    };
    Ok(access.authorize(MailServerStatus {
        credential_set,
        readiness,
    }))
}

/// Sends a test e-mail to the person who asks.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*, `mail.off` while the host or the
/// sender is not entered, `mail.insecure` when the settings would send a user name or a password
/// without encryption to another machine, `rate_limit.exceeded` (with the seconds to wait) when
/// the person sent one a moment ago, `mail.rejected` when the mail server turns the e-mail away
/// for good, `mail.unavailable` when it cannot be reached.
pub async fn send_test_email<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SendTestEmail,
) -> Result<Authorized<Done>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let layers = ctx.setting_layers_of(None).await?;
    let Some(server) = ctx.mail_server(&layers).await? else {
        return Err(AppError::Refused(ErrorCode::MailOff));
    };
    ctx.reserve_test_email(actor.person.id)?;
    let message = MailMessage {
        to: actor.person.email.clone(),
        subject: Message::from(message::MAIL_TEST_SUBJECT).render(),
        text: Message::from(message::MAIL_TEST_TEXT).render(),
    };
    let sent = ctx.services.mailer.send(&server, &message).await;
    // A try is an event whether or not it worked; the answer follows it.
    let event = ctx.event(
        by(actor.person.id),
        fixed_entity(RecordKind::Setting, MAIL_RECORD)?,
        vec![changed(if sent.is_ok() {
            "mail.test-sent"
        } else {
            "mail.test-failed"
        })],
    )?;
    ctx.store()
        .record_change(&event, |_| Box::pin(async { Ok(()) }))
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    sent.map_err(send_failure)?;
    Ok(access.authorize(Done))
}
