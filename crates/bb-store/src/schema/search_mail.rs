//! The table of e-mailed searches (QRY-07, ADR-0033, amendment 2026-11-24).

use super::{Iden, StoredTable};

/// The e-mailed searches: one row for each saved search a person gets by e-mail. The saved search
/// has a reference, so a row never outlives its search; the person has none, like the maker of a
/// saved search (a removed person leaves a tombstone, PRIV-04), and their rows go with them.
#[derive(Clone, Copy, Iden)]
pub(crate) enum SearchMailRow {
    #[iden = "search_mail_schedule"]
    Table,
    Id,
    Holder,
    Search,
    Days,
    Hour,
    OnlyWhenChanged,
    Paused,
    NextAt,
    LastSentAt,
    CreatedAt,
    UpdatedAt,
}

stored_tables! {
    SearchMailRow => bb_core::search_mail::SearchMail {
        Id => id,
        Holder => holder,
        Search => search,
        Days => days,
        Hour => hour,
        OnlyWhenChanged => only_when_changed,
        Paused => paused,
        NextAt => next_at,
        LastSentAt => last_sent_at,
        CreatedAt => created_at,
        UpdatedAt => updated_at,
    },
}
