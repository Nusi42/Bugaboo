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
}

/// An account that waits for its address to be confirmed (M1-F38).
#[derive(Clone, Copy, Iden)]
pub(crate) enum AddressConfirmationRow {
    #[iden = "address_confirmation"]
    Table,
    Person,
    TokenHash,
    CreatedAt,
    ExpiresAt,
}
