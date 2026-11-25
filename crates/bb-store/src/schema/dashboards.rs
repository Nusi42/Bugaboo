//! The tables of dashboards (CFG-03, ADR-0050).

use super::{Iden, StoredTable};

/// Dashboards. The maker has no reference: a removed person leaves a tombstone (PRIV-04).
#[derive(Clone, Copy, Iden)]
pub(crate) enum DashboardRow {
    #[iden = "dashboard"]
    Table,
    Id,
    Name,
    Maker,
    Home,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Version,
}

/// The boxes of a dashboard, in order. A box reads a search text or a saved search: the saved
/// search has no reference, because a box whose saved search is gone is left out, never refused.
#[derive(Clone, Copy, Iden)]
pub(crate) enum DashboardBoxRow {
    #[iden = "dashboard_box"]
    Table,
    Dashboard,
    Id,
    Position,
    Kind,
    GridColumn,
    GridRow,
    Width,
    Height,
    Title,
    SearchText,
    SavedSearch,
    ChartBy,
    ChartShape,
    TimeSpan,
}

/// The groups and projects a dashboard is shared with. The target has **no reference**, as for
/// saved searches: `kind` tells a group from a project.
#[derive(Clone, Copy, Iden)]
pub(crate) enum DashboardShareRow {
    #[iden = "dashboard_share"]
    Table,
    Dashboard,
    Kind,
    Target,
    SharedAt,
}

stored_tables! {
    DashboardRow => bb_core::dashboard::Dashboard {
        Id => id,
        Name => name,
        Maker => maker,
        Home => home,
        CreatedAt => created_at,
        UpdatedAt => updated_at,
        DeletedAt => deleted_at,
        Version => version,
    },
    DashboardBoxRow => bb_core::dashboard::DashboardBox {
        Dashboard => dashboard,
        Id => id,
        Position => position,
        Kind => spec,
        GridColumn => spec,
        GridRow => spec,
        Width => spec,
        Height => spec,
        Title => spec,
        SearchText => spec,
        SavedSearch => spec,
        ChartBy => spec,
        ChartShape => spec,
        TimeSpan => spec,
    },
    DashboardShareRow => bb_core::dashboard::DashboardShare {
        Dashboard => dashboard,
        Kind => target,
        Target => target,
        SharedAt => shared_at,
    },
}
