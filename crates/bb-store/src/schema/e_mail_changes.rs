//! The table of the changes of e-mail addresses that wait for their link (PRIV-06, ADR-0044).

use super::{Iden, StoredTable};

/// A change of a person's address that waits for its link: one per person. The person has no
/// reference: a removed person leaves a tombstone (PRIV-04), and the daily job deletes the row once
/// its link has run out.
#[derive(Clone, Copy, Iden)]
pub(crate) enum EmailChangeRow {
    #[iden = "e_mail_change"]
    Table,
    Person,
    TokenDigest,
    NewEmail,
    AskedAt,
    ExpiresAt,
}

stored_tables! {
    EmailChangeRow => bb_core::e_mail_change::EmailChange {
        Person => person,
        TokenDigest => token_digest,
        NewEmail => new_email,
        AskedAt => asked_at,
        ExpiresAt => expires_at,
    },
}
