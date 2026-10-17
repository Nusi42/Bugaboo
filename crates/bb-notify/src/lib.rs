//! Delivery of e-mail through the organization's mail server (NTF-02, ADR-0033 §3): the adapter of
//! the port `Mailer` of `bb-app`, built on `lettre`.
//!
//! Every e-mail is plain text and nothing else: no HTML, no picture, no attachment, so nothing in
//! it loads from outside or runs in a mail program. A subject is one line, whatever the text it
//! was made of. What this crate logs is a class of failure and never the answer of the mail
//! server, which can repeat an address (DATA-LOGS).

#![forbid(unsafe_code)]

mod smtp;

pub use smtp::{SERVER_TIMEOUT, SmtpMailer};
