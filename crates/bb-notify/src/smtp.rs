//! Sending one e-mail over SMTP.

use std::time::Duration;

use bb_app::mailer::{MailFailure, MailMessage, MailServer, Mailer};
use bb_app::ports::BoxFuture;
use bb_core::mail::MailSecurity;
use bb_core::text::{MAIL_SUBJECT_MAX, plain_line};
use lettre::message::Mailbox;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

/// How long the mail server may take to answer one command before the try ends.
pub const SERVER_TIMEOUT: Duration = Duration::from_secs(30);

/// Sends e-mail through the mail server it is given with each e-mail.
#[derive(Clone, Copy, Debug, Default)]
pub struct SmtpMailer;

impl Mailer for SmtpMailer {
    fn send<'a>(
        &'a self,
        server: &'a MailServer,
        message: &'a MailMessage,
    ) -> BoxFuture<'a, Result<(), MailFailure>> {
        Box::pin(async move {
            let email = email_of(server, message)?;
            let transport = transport_of(server)?;
            transport.send(email).await.map(|_| ()).map_err(|error| {
                // Only the class: the text of the answer can hold the address.
                let failure = if error.is_permanent() {
                    MailFailure::Rejected
                } else {
                    MailFailure::Unavailable
                };
                tracing::warn!(
                    permanent = error.is_permanent(),
                    timed_out = error.is_timeout(),
                    tls = error.is_tls(),
                    "the mail server did not take an e-mail"
                );
                failure
            })
        })
    }
}

/// Whatever went wrong while the e-mail is put together (an address that is none), trying again
/// cannot help.
fn rejected<E>(_: E) -> MailFailure {
    MailFailure::Rejected
}

fn email_of(server: &MailServer, message: &MailMessage) -> Result<Message, MailFailure> {
    let from: Mailbox = server.from.parse().map_err(rejected)?;
    let to: Mailbox = message.to.parse().map_err(rejected)?;
    Message::builder()
        .from(from)
        .to(to)
        .subject(plain_line(&message.subject, MAIL_SUBJECT_MAX))
        .header(ContentType::TEXT_PLAIN)
        .body(message.text.clone())
        .map_err(rejected)
}

fn transport_of(server: &MailServer) -> Result<AsyncSmtpTransport<Tokio1Executor>, MailFailure> {
    let host = server.host.as_str();
    // A mail server that cannot be set up from these settings is a setting to fix, not a
    // problem that goes away by waiting.
    let builder = match server.security {
        MailSecurity::StartTls => {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host).map_err(rejected)?
        }
        MailSecurity::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(host).map_err(rejected)?,
        MailSecurity::Off => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host),
    };
    let builder = builder.port(server.port).timeout(Some(SERVER_TIMEOUT));
    Ok(if server.username.is_empty() {
        builder
    } else {
        builder.credentials(Credentials::new(
            server.username.clone(),
            server.password.to_string(),
        ))
    }
    .build())
}

#[cfg(test)]
mod tests;
