//! The tables of the releases of a project (ISS-08, ADR-0049).

use super::{Iden, StoredTable};

/// The releases of the projects. The people who made and changed one have no reference: a removed
/// person leaves a tombstone (PRIV-04).
#[derive(Clone, Copy, Iden)]
pub(crate) enum ReleaseRow {
    #[iden = "release"]
    Table,
    Id,
    Project,
    Name,
    ReleaseDate,
    State,
    ReleasedOn,
    MadeBy,
    MadeAt,
    ChangedBy,
    ChangedAt,
    DeletedAt,
    Version,
}

/// The releases an issue names in *Found in release*, in the order given.
#[derive(Clone, Copy, Iden)]
pub(crate) enum IssueFoundInRow {
    #[iden = "issue_found_in_release"]
    Table,
    Issue,
    Release,
    Position,
}

stored_tables! {
    ReleaseRow => bb_core::project_release::Release {
        Id => id,
        Project => project,
        Name => name,
        ReleaseDate => release_date,
        State => state,
        ReleasedOn => released_on,
        MadeBy => made_by,
        MadeAt => made_at,
        ChangedBy => changed_by,
        ChangedAt => changed_at,
        DeletedAt => deleted_at,
        Version => version,
    },
    IssueFoundInRow => bb_core::project_release::IssueFoundIn {
        Issue => issue,
        Release => release,
        Position => position,
    },
}
