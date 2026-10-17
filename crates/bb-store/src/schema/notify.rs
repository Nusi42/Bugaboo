//! The tables of notifications, handles and the confirmation of addresses (M4 WP-06).

use super::Iden;

/// What each person is told about, built from the events by an outbox consumer (NTF-01).
#[derive(Clone, Copy, Iden)]
pub(crate) enum NotificationRow {
    #[iden = "notification"]
    Table,
    Id,
    Recipient,
    Kind,
    Issue,
    Event,
    Once,
    Record,
    By,
    At,
    Changes,
    InApp,
    Mail,
    ReadAt,
}

/// The handle of each person, for mentions (M3-F70).
#[derive(Clone, Copy, Iden)]
pub(crate) enum PersonHandleRow {
    #[iden = "person_handle"]
    Table,
    Person,
    Handle,
    ChangedAt,
    Current,
}

/// The last e-mail that went to an address for a sign-up (M1-F38).
#[derive(Clone, Copy, Iden)]
pub(crate) enum SignUpMailRow {
    #[iden = "sign_up_mail"]
    Table,
    Address,
    Invitation,
    SentAt,
}

/// The link a sign-up on one's own makes (M1-F38): an invitation with nobody who invited.
#[derive(Clone, Copy, Iden)]
pub(crate) enum SignUpInvitationRow {
    #[iden = "sign_up_invitation"]
    Table,
    Id,
    TokenHash,
    Email,
    CreatedAt,
    ExpiresAt,
    UsedAt,
}
