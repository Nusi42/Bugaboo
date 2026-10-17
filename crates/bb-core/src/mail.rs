//! The organization's mail server (NTF-02, PRIV-08, ADR-0033 §3): the words of its settings.
//!
//! The settings themselves (`mail.*`) are in the registry; the password is a stored secret
//! (SEC-16) and is no setting.

/// The name the mail password is stored under (SEC-16).
pub const PASSWORD_SECRET: &str = "mail.password";

/// The longest a mail password may be, in bytes: a limit of storage, not of security.
pub const PASSWORD_MAX: usize = 1024;

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
    fn the_default_of_the_security_setting_is_one_of_the_words() {
        let default = crate::settings::MAIL_SECURITY.default_value().unwrap();
        assert!(MailSecurity::from_code(&default).is_some());
    }
}
