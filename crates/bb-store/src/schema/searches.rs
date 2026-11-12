//! The tables of saved searches (QRY-06, ADR-0034).

use super::{Iden, StoredTable};

/// Saved searches. The maker has no reference: a removed person leaves a tombstone (PRIV-04).
#[derive(Clone, Copy, Iden)]
pub(crate) enum SavedSearchRow {
    #[iden = "saved_search"]
    Table,
    Id,
    Name,
    Text,
    Maker,
    View,
    GroupBy,
    TimelineStart,
    TimelineEnd,
    PlanByEffort,
    Zoom,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Version,
}

/// The columns of a saved search, in order.
#[derive(Clone, Copy, Iden)]
pub(crate) enum SearchColumnRow {
    #[iden = "saved_search_column"]
    Table,
    Search,
    Position,
    Field,
    Width,
}

/// The groups and projects a saved search is shared with. The target has **no reference**: it holds
/// the ID of a group or of a project, which `kind` tells apart, and one column cannot refer to
/// two tables. A group that is deleted takes its rows with it in the same change.
#[derive(Clone, Copy, Iden)]
pub(crate) enum SearchShareRow {
    #[iden = "saved_search_share"]
    Table,
    Search,
    Kind,
    Target,
    SharedAt,
}

/// The saved search a project shows as its list.
#[derive(Clone, Copy, Iden)]
pub(crate) enum ProjectDefaultListRow {
    #[iden = "project_default_list"]
    Table,
    Project,
    Search,
}

stored_tables! {
    SavedSearchRow => bb_core::saved_search::SavedSearch {
        Id => id,
        Name => name,
        Text => text,
        Maker => maker,
        View => view,
        GroupBy => group_by,
        TimelineStart => timeline_start,
        TimelineEnd => timeline_end,
        PlanByEffort => plan_by_effort,
        Zoom => zoom,
        CreatedAt => created_at,
        UpdatedAt => updated_at,
        DeletedAt => deleted_at,
        Version => version,
    },
    SearchColumnRow => bb_core::saved_search::SearchColumn {
        Search => search,
        Position => position,
        Field => field,
        Width => width,
    },
    SearchShareRow => bb_core::saved_search::SearchShare {
        Search => search,
        Kind => target,
        Target => target,
        SharedAt => shared_at,
    },
    ProjectDefaultListRow => bb_core::saved_search::ProjectDefaultList {
        Project => project,
        Search => search,
    },
}
