//! The port of sending e-mail (ADR-0033 §3): what the use cases need from a mail server. The
//! adapter (`bb-notify`) speaks SMTP; the tests use [`FakeMailer`], which keeps what it would have
//! sent.
//!
//! An e-mail address and a password are personal data and a secret, so none of the types here
//! writes them with `Debug`, and a failure names a class of problem and never the text of the
//! mail server's answer, which can repeat the address (DATA-LOGS).

use std::fmt;

use bb_core::mail::MailSecurity;
use zeroize::Zeroizing;

use crate::ports::BoxFuture;

/// The mail server to send through, with the password in the form it is used in. The server is
/// read from the settings and the stored secret again for every sending, so a change of either
/// counts from the next e-mail.
#[derive(Clone)]
pub struct MailServer {
    /// The host name.
    pub host: String,
    /// The port.
    pub port: u16,
    /// How the connection is protected.
    pub security: MailSecurity,
    /// The user name; empty for a server that asks for none.
    pub username: String,
    /// The password; empty for a server that asks for none. Wiped from memory when dropped.
    pub password: Zeroizing<String>,
    /// The address e-mails come from.
    pub from: String,
}

impl fmt::Debug for MailServer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MailServer")
            .field("port", &self.port)
            .field("security", &self.security)
            .finish_non_exhaustive()
    }
}

/// One e-mail: plain text only, so there is nothing that loads from outside or runs in a mail
/// program.
#[derive(Clone, PartialEq, Eq)]
pub struct MailMessage {
    /// The address it goes to.
    pub to: String,
    /// The subject, on one line.
    pub subject: String,
    /// The text.
    pub text: String,
}

impl fmt::Debug for MailMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MailMessage")
            .field("text_length", &self.text.len())
            .finish_non_exhaustive()
    }
}

/// Why an e-mail did not go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailFailure {
    /// The mail server refused it for good, or an address is not one: trying again cannot help.
    Rejected,
    /// The mail server cannot be reached or said to try later: another go may work.
    Unavailable,
}

/// Sends e-mail through a mail server.
pub trait Mailer: Send + Sync {
    /// Sends `message` through `server`.
    ///
    /// # Errors
    /// [`MailFailure::Rejected`] when no later go can work, [`MailFailure::Unavailable`] when one
    /// may.
    fn send<'a>(
        &'a self,
        server: &'a MailServer,
        message: &'a MailMessage,
    ) -> BoxFuture<'a, Result<(), MailFailure>>;
}

/// The mailer of a server that has none wired in: every e-mail is turned away as unavailable, so
/// the job waits and the person who switched e-mail on is told by the test e-mail.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoMailer;

impl Mailer for NoMailer {
    fn send<'a>(
        &'a self,
        _server: &'a MailServer,
        _message: &'a MailMessage,
    ) -> BoxFuture<'a, Result<(), MailFailure>> {
        Box::pin(async { Err(MailFailure::Unavailable) })
    }
}

#[cfg(any(test, feature = "testing"))]
pub use fake::FakeMailer;

#[cfg(any(test, feature = "testing"))]
mod fake {
    use std::sync::{Arc, Mutex};

    use super::{BoxFuture, MailFailure, MailMessage, MailServer, Mailer};
    use crate::auth::locked;

    /// A mailer that keeps what it would have sent, for tests. Clones share what was sent, so a
    /// test keeps one and hands the other to the services.
    #[derive(Clone, Debug, Default)]
    pub struct FakeMailer {
        sent: Arc<Mutex<Vec<(MailServer, MailMessage)>>>,
        failures: Arc<Mutex<Vec<MailFailure>>>,
    }

    impl FakeMailer {
        /// Every e-mail sent so far, oldest first, with the server it went through.
        #[must_use]
        pub fn sent(&self) -> Vec<(MailServer, MailMessage)> {
            locked(&self.sent).clone()
        }

        /// The e-mails sent so far, without the servers.
        #[must_use]
        pub fn messages(&self) -> Vec<MailMessage> {
            self.sent()
                .into_iter()
                .map(|(_, message)| message)
                .collect()
        }

        /// The next send fails with `failure` (and the one after it with the next failure given).
        pub fn fail_next(&self, failure: MailFailure) {
            locked(&self.failures).push(failure);
        }
    }

    impl Mailer for FakeMailer {
        fn send<'a>(
            &'a self,
            server: &'a MailServer,
            message: &'a MailMessage,
        ) -> BoxFuture<'a, Result<(), MailFailure>> {
            Box::pin(async move {
                let mut failures = locked(&self.failures);
                if !failures.is_empty() {
                    return Err(failures.remove(0));
                }
                drop(failures);
                locked(&self.sent).push((server.clone(), message.clone()));
                Ok(())
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server() -> MailServer {
        MailServer {
            host: "mail.example.test".to_owned(),
            port: 587,
            security: MailSecurity::StartTls,
            username: "bugaboo".to_owned(),
            password: Zeroizing::new("a-made-up-password".to_owned()),
            from: "bugaboo@example.test".to_owned(),
        }
    }

    #[test]
    fn sec_16_the_password_and_the_address_never_show_in_debug_output() {
        let shown = format!(
            "{:?} {:?}",
            server(),
            MailMessage {
                to: "ana@example.test".to_owned(),
                subject: "Subject".to_owned(),
                text: "Text".to_owned()
            }
        );
        assert!(!shown.contains("a-made-up-password"));
        assert!(!shown.contains("example.test"));
        assert!(!shown.contains("bugaboo"));
    }

    #[tokio::test]
    async fn the_fake_mailer_keeps_what_it_would_have_sent_and_fails_when_told_to() {
        let mailer = FakeMailer::default();
        let message = MailMessage {
            to: "ana@example.test".to_owned(),
            subject: "Hello".to_owned(),
            text: "Text".to_owned(),
        };
        mailer.fail_next(MailFailure::Unavailable);
        assert_eq!(
            mailer.send(&server(), &message).await,
            Err(MailFailure::Unavailable)
        );
        assert!(mailer.sent().is_empty());
        assert_eq!(mailer.send(&server(), &message).await, Ok(()));
        assert_eq!(mailer.messages(), [message]);
    }

    #[tokio::test]
    async fn a_server_without_a_mailer_turns_every_e_mail_away_as_unavailable() {
        let message = MailMessage {
            to: "ana@example.test".to_owned(),
            subject: "Hello".to_owned(),
            text: "Text".to_owned(),
        };
        assert_eq!(
            NoMailer.send(&server(), &message).await,
            Err(MailFailure::Unavailable)
        );
    }
}
