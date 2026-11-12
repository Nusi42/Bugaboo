//! The table of own themes (THM-04, ADR-0026 update M8).

use super::{Iden, StoredTable};

/// Own themes. The maker has no reference: a removed person leaves a tombstone (PRIV-04), and the
/// personal themes of that person go with them.
#[derive(Clone, Copy, Iden)]
pub(crate) enum OwnThemeRow {
    #[iden = "own_theme"]
    Table,
    Id,
    Kind,
    Maker,
    Name,
    Extends,
    Values,
    Draft,
    DraftVersion,
    SavedAt,
    CreatedAt,
    UpdatedAt,
    Version,
}

stored_tables! {
    OwnThemeRow => bb_core::theme::OwnTheme {
        Id => id,
        Kind => kind,
        Maker => maker,
        Name => name,
        Extends => extends,
        Values => values,
        Draft => draft,
        DraftVersion => draft_version,
        SavedAt => saved_at,
        CreatedAt => created_at,
        UpdatedAt => updated_at,
        Version => version,
    },
}
