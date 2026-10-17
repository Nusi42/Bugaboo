//! The organization's mail server (NTF-02, PRIV-08, ADR-0033 §3): the words of its settings.
//!
//! The settings themselves (`mail.*`) are in the registry; the password is a stored secret
//! (SEC-16) and is no setting.

/// The name the mail password is stored under (SEC-16).
pub const PASSWORD_SECRET: &str = "mail.password";

/// The longest a mail password may be, in bytes: a limit of storage, not of security.
pub const PASSWORD_MAX: usize = 1024;

/// How many seconds one person waits between two test e-mails, so the button cannot be used to send
/// a stream of e-mail through the mail server.
pub const TEST_EMAIL_EVERY_SECONDS: i64 = 10;

/// Whether a host name is this machine: `localhost`, an address of `127.0.0.0/8`, or `::1`.
#[must_use]
pub fn is_loopback(host: &str) -> bool {
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost")
        || host == "::1"
        || host
            .parse::<std::net::Ipv4Addr>()
            .is_ok_and(|address| address.is_loopback())
}

/// Whether the settings would send a user name or a password over a connection that is not
/// encrypted, to a mail server on another machine. Bugaboo refuses to: the secret would cross the
/// network in the clear.
#[must_use]
pub fn would_send_secrets_in_the_clear(
    security: MailSecurity,
    host: &str,
    has_credentials: bool,
) -> bool {
    security == MailSecurity::Off && has_credentials && !is_loopback(host)
}

words! {
    /// How the connection to the mail server is protected.
    pub enum MailSecurity {
        /// The connection starts in the clear and switches to encryption before anything private
        /// is sent. The usual way (port 587).
        StartTls => "starttls",
        /// The connection is encrypted from the first byte (port 465).
        Tls => "tls",
        /// No encryption. Only for a mail server on the same machine or a trusted network.
        Off => "off",
    }
}

words_in_api!(
    MailSecurity,
    "How the connection to the mail server is protected: it switches to encryption, it is encrypted from the start, or it is not encrypted."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_never_sent_in_the_clear_to_another_machine() {
        let sends = would_send_secrets_in_the_clear;
        assert!(sends(MailSecurity::Off, "mail.example.test", true));
        assert!(
            !sends(MailSecurity::Off, "mail.example.test", false),
            "nothing to hide"
        );
        assert!(!sends(MailSecurity::StartTls, "mail.example.test", true));
        assert!(!sends(MailSecurity::Tls, "mail.example.test", true));
        for here in [
            "localhost",
            "LocalHost",
            "127.0.0.1",
            "127.1.2.3",
            "::1",
            "[::1]",
        ] {
            assert!(
                !sends(MailSecurity::Off, here, true),
                "{here} is this machine"
            );
        }
        // A name that only starts like a loopback address is another machine.
        assert!(sends(MailSecurity::Off, "127.0.0.1.example.test", true));
        assert!(sends(MailSecurity::Off, "10.0.0.5", true));
    }

    #[test]
    fn the_default_of_the_security_setting_is_one_of_the_words() {
        let default = crate::settings::MAIL_SECURITY.default_value().unwrap();
        assert!(MailSecurity::from_code(&default).is_some());
    }
}
