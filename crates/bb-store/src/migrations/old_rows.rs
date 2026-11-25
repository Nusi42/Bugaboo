//! Rows as a database of an older migration holds them, for the tests of the migrations that
//! change what is stored: written with the column names of `crate::schema`, never through the
//! program's use cases, so a test starts from what an old server left.

use bb_app::layouts::LayoutStore as _;
use bb_core::fields::FieldKey;
use bb_core::id::{LayoutId, RoleId, SavedSearchId, UserId};
use bb_testing::{Failure, TestResult};
use sea_query::{DynIden, Expr, IntoIden as _, Query};

use super::Database;
use crate::auth::AuthStorage;
use crate::db::StoredId as _;
use crate::schema::{
    LayoutFieldRoleRow, LayoutFieldRow, LayoutOffFieldRow, LayoutRow, LayoutSectionRow, RoleRow,
    SavedSearchRow,
};

/// Inserts one row of `values` into `table`.
pub(super) async fn insert<const N: usize>(
    database: &Database,
    table: impl sea_query::IntoIden,
    columns: [DynIden; N],
    values: [Expr; N],
) -> TestResult {
    let mut row = Query::insert();
    row.into_table(table).columns(columns).values(values)?;
    database.execute(&row).await?;
    Ok(())
}

/// A saved list *Open bugs* of `maker`, with the columns every version since migration 30 has (no
/// spacing, no options of a view).
pub(super) async fn old_saved_list(
    database: &Database,
    search: SavedSearchId,
    maker: UserId,
) -> TestResult {
    insert(
        database,
        SavedSearchRow::Table,
        [
            SavedSearchRow::Id.into_iden(),
            SavedSearchRow::Name.into_iden(),
            SavedSearchRow::Text.into_iden(),
            SavedSearchRow::Maker.into_iden(),
            SavedSearchRow::View.into_iden(),
            SavedSearchRow::CreatedAt.into_iden(),
            SavedSearchRow::UpdatedAt.into_iden(),
            SavedSearchRow::Version.into_iden(),
        ],
        [
            Expr::val(search.stored()),
            Expr::val("Open bugs"),
            Expr::val("status is open"),
            Expr::val(maker.stored()),
            Expr::val("list"),
            Expr::val(1_i64),
            Expr::val(1_i64),
            Expr::val(1_i64),
        ],
    )
    .await
}

/// A role at this place among the roles.
pub(super) async fn old_role(database: &Database, role: RoleId, position: i64) -> TestResult {
    insert(
        database,
        RoleRow::Table,
        [
            RoleRow::Id.into_iden(),
            RoleRow::Name.into_iden(),
            RoleRow::Position.into_iden(),
            RoleRow::TwoStep.into_iden(),
            RoleRow::Version.into_iden(),
        ],
        [
            Expr::val(role.stored()),
            Expr::val(format!("Role {position}")),
            Expr::val(position),
            Expr::val(false),
            Expr::val(1_i64),
        ],
    )
    .await
}

/// A role a field of a layout is shown to.
async fn old_role_of(
    database: &Database,
    layout: LayoutId,
    field: &str,
    role: RoleId,
) -> TestResult {
    insert(
        database,
        LayoutFieldRoleRow::Table,
        [
            LayoutFieldRoleRow::Layout.into_iden(),
            LayoutFieldRoleRow::Field.into_iden(),
            LayoutFieldRoleRow::Role.into_iden(),
        ],
        [
            Expr::val(layout.stored()),
            Expr::val(field.to_owned()),
            Expr::val(role.stored()),
        ],
    )
    .await
}

/// A layout of this kind, with these sections of fields (a section has the fields it is given, in
/// order) and these fields taken off its form. Every field is shown to every one of `roles`.
pub(super) async fn old_layout(
    database: &Database,
    layout: LayoutId,
    kind: &str,
    sections: &[&[&str]],
    off: &[&str],
    roles: &[RoleId],
) -> TestResult {
    insert(
        database,
        LayoutRow::Table,
        [
            LayoutRow::Id.into_iden(),
            LayoutRow::Kind.into_iden(),
            LayoutRow::Version.into_iden(),
        ],
        [
            Expr::val(layout.stored()),
            Expr::val(kind.to_owned()),
            Expr::val(1_i64),
        ],
    )
    .await?;
    for (section, fields) in (0_i64..).zip(sections) {
        insert(
            database,
            LayoutSectionRow::Table,
            [
                LayoutSectionRow::Layout.into_iden(),
                LayoutSectionRow::Position.into_iden(),
                LayoutSectionRow::Tab.into_iden(),
                LayoutSectionRow::Title.into_iden(),
            ],
            [
                Expr::val(layout.stored()),
                Expr::val(section),
                Expr::val(""),
                Expr::val(""),
            ],
        )
        .await?;
        for (position, field) in (0_i64..).zip(*fields) {
            insert(
                database,
                LayoutFieldRow::Table,
                [
                    LayoutFieldRow::Layout.into_iden(),
                    LayoutFieldRow::Field.into_iden(),
                    LayoutFieldRow::Section.into_iden(),
                    LayoutFieldRow::Position.into_iden(),
                ],
                [
                    Expr::val(layout.stored()),
                    Expr::val((*field).to_owned()),
                    Expr::val(section),
                    Expr::val(position),
                ],
            )
            .await?;
            for role in roles {
                old_role_of(database, layout, field, *role).await?;
            }
        }
    }
    for field in off {
        insert(
            database,
            LayoutOffFieldRow::Table,
            [
                LayoutOffFieldRow::Layout.into_iden(),
                LayoutOffFieldRow::Field.into_iden(),
            ],
            [Expr::val(layout.stored()), Expr::val((*field).to_owned())],
        )
        .await?;
        for role in roles {
            old_role_of(database, layout, field, *role).await?;
        }
    }
    Ok(())
}

/// A row of the history as it was before the moment it is shown at was stored (migration 44): of
/// the kind of record `record_kind`, written at `at` microseconds, with these `changes` (JSON).
pub(super) async fn old_history_row(
    database: &Database,
    event: u8,
    record_kind: &str,
    at: i64,
    changes: &str,
) -> TestResult {
    use crate::schema::HistoryEntryRow as Row;
    insert(
        database,
        Row::Table,
        [
            Row::Event.into_iden(),
            Row::Issue.into_iden(),
            Row::Position.into_iden(),
            Row::At.into_iden(),
            Row::Kind.into_iden(),
            Row::Record.into_iden(),
            Row::RecordKind.into_iden(),
            Row::Changes.into_iden(),
        ],
        [
            Expr::val(vec![event; 16]),
            Expr::val(vec![9_u8; 16]),
            Expr::val(i64::from(event)),
            Expr::val(at),
            Expr::val("change"),
            Expr::val(vec![8_u8; 16]),
            Expr::val(record_kind),
            Expr::val(changes),
        ],
    )
    .await
}

/// A field of a stored layout with the roles it is shown to, in order of role.
pub(super) type Placed = (FieldKey, Vec<RoleId>);

/// What a stored layout holds: its version and the fields of each section.
pub(super) async fn read_layout(
    database: &Database,
    layout: LayoutId,
) -> Result<(i64, Vec<Vec<Placed>>), Failure> {
    let mut read = database.begin_read().await?;
    let stored = AuthStorage::stored_layout_in(&mut read, layout)
        .await?
        .ok_or("the layout is gone")?;
    let sections = stored
        .content
        .sections
        .iter()
        .map(|section| {
            section
                .fields
                .iter()
                .map(|placed| {
                    let mut roles = placed.roles.clone();
                    roles.sort();
                    (placed.field, roles)
                })
                .collect()
        })
        .collect();
    Ok((stored.layout.version, sections))
}

/// These fields, each shown to these roles.
pub(super) fn shown(fields: &[FieldKey], roles: &[RoleId]) -> Vec<Placed> {
    let mut roles = roles.to_vec();
    roles.sort();
    fields.iter().map(|field| (*field, roles.clone())).collect()
}
