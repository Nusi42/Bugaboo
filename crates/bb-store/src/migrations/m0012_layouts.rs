//! Migration 12: layouts of the issue page and the new-issue form (CFG-01, ISS-02, ADR-0027).
//!
//! A layout row names its place: a project (none for the whole organization), an issue type (none
//! for all types) and what it is for. Its sections, the fields in them and the roles each field is
//! shown to are rows of their own, replaced together when the layout is saved. A field is in a
//! layout once. Nothing refers to a role: a role that is taken away leaves a row that no person
//! holds, which no form reads.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{
    IssueTypeRow, LayoutFieldRoleRow, LayoutFieldRow, LayoutRow, LayoutSectionRow, ProjectRow,
};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = layouts();
    steps.extend(contents());
    steps
}

fn layouts() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(LayoutRow::Table)
                .col(
                    ColumnDef::new(LayoutRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(LayoutRow::Project).blob())
                .col(ColumnDef::new(LayoutRow::IssueType).blob())
                .col(ColumnDef::new(LayoutRow::Kind).text().not_null())
                .col(ColumnDef::new(LayoutRow::Version).big_integer().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .from(LayoutRow::Table, LayoutRow::Project)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(LayoutRow::Table, LayoutRow::IssueType)
                        .to(IssueTypeRow::Table, IssueTypeRow::Id),
                )
                .to_owned(),
        ),
        // The layouts of one kind are read together, and the nearest place is picked in the program.
        Step::Index(
            Index::create()
                .name("layout_by_kind")
                .table(LayoutRow::Table)
                .col(LayoutRow::Kind)
                .to_owned(),
        ),
    ]
}

fn contents() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(LayoutSectionRow::Table)
                .col(id_column(LayoutSectionRow::Layout))
                .col(
                    ColumnDef::new(LayoutSectionRow::Position)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(LayoutSectionRow::Tab).text().not_null())
                .col(ColumnDef::new(LayoutSectionRow::Title).text().not_null())
                .primary_key(
                    Index::create()
                        .col(LayoutSectionRow::Layout)
                        .col(LayoutSectionRow::Position),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(LayoutSectionRow::Table, LayoutSectionRow::Layout)
                        .to(LayoutRow::Table, LayoutRow::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(LayoutFieldRow::Table)
                .col(id_column(LayoutFieldRow::Layout))
                .col(ColumnDef::new(LayoutFieldRow::Field).text().not_null())
                .col(
                    ColumnDef::new(LayoutFieldRow::Section)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(LayoutFieldRow::Position)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(LayoutFieldRow::Layout)
                        .col(LayoutFieldRow::Field),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(LayoutFieldRow::Table, LayoutFieldRow::Layout)
                        .to(LayoutRow::Table, LayoutRow::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(LayoutFieldRoleRow::Table)
                .col(id_column(LayoutFieldRoleRow::Layout))
                .col(ColumnDef::new(LayoutFieldRoleRow::Field).text().not_null())
                .col(id_column(LayoutFieldRoleRow::Role))
                .primary_key(
                    Index::create()
                        .col(LayoutFieldRoleRow::Layout)
                        .col(LayoutFieldRoleRow::Field)
                        .col(LayoutFieldRoleRow::Role),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(LayoutFieldRoleRow::Table, LayoutFieldRoleRow::Layout)
                        .to(LayoutRow::Table, LayoutRow::Id),
                )
                .to_owned(),
        ),
    ]
}
