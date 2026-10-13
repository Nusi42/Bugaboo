//! Migration 9: own fields (CF-01, CF-03, CF-04, CF-06, ADR-0006, ADR-0027): the definition of a
//! field, where it is used, its choices, the values people fill in, and the levels of fields per
//! issue type.
//!
//! Values are in one table with one typed column per kind and one row per value; indexes per
//! column keep a search over one field fast. There is no index on the text column: a long text
//! can be larger than an index entry of PostgreSQL, and searching text comes with search (M5).
//! A value names its item by kind and ID, so the one table serves issues now and time entries,
//! documents and releases later.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{
    FieldChoiceRow, FieldIssueTypeRow, FieldProjectRow, FieldRow, FieldValueRow, IssueTypeRow,
    ProjectRow, ProjectTypeFieldLevelRow, RoleRow, RoleTypeFieldLevelRow,
};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = definitions();
    steps.extend(choices());
    steps.extend(values());
    steps.extend(levels_per_type());
    steps.extend(project_levels_per_type());
    steps
}

fn definitions() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(FieldRow::Table)
                .col(id_column(FieldRow::Id).primary_key())
                .col(ColumnDef::new(FieldRow::Name).text().not_null())
                .col(ColumnDef::new(FieldRow::Help).text().not_null())
                .col(ColumnDef::new(FieldRow::Kind).text().not_null())
                .col(ColumnDef::new(FieldRow::AppliesTo).text().not_null())
                // The rules as JSON text (`bb_core::own_fields::Rules`).
                .col(ColumnDef::new(FieldRow::Rules).text().not_null())
                // The default value as JSON text; empty for none.
                .col(ColumnDef::new(FieldRow::DefaultValue).text().not_null())
                .col(ColumnDef::new(FieldRow::Look).text().not_null())
                .col(ColumnDef::new(FieldRow::NeverToAssistant).boolean().not_null())
                .col(ColumnDef::new(FieldRow::Position).big_integer().not_null())
                .col(ColumnDef::new(FieldRow::RetiredAt).big_integer())
                .col(ColumnDef::new(FieldRow::Version).big_integer().not_null())
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(FieldProjectRow::Table)
                .col(id_column(FieldProjectRow::Field))
                .col(id_column(FieldProjectRow::Project))
                .primary_key(
                    Index::create()
                        .col(FieldProjectRow::Field)
                        .col(FieldProjectRow::Project),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(FieldProjectRow::Table, FieldProjectRow::Field)
                        .to(FieldRow::Table, FieldRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(FieldProjectRow::Table, FieldProjectRow::Project)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .to_owned(),
        ),
        // "Used in 4 projects" and the fields of a project are found by project.
        Step::Index(
            Index::create()
                .name("field_project_by_project")
                .table(FieldProjectRow::Table)
                .col(FieldProjectRow::Project)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(FieldIssueTypeRow::Table)
                .col(id_column(FieldIssueTypeRow::Field))
                .col(id_column(FieldIssueTypeRow::IssueType))
                .primary_key(
                    Index::create()
                        .col(FieldIssueTypeRow::Field)
                        .col(FieldIssueTypeRow::IssueType),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(FieldIssueTypeRow::Table, FieldIssueTypeRow::Field)
                        .to(FieldRow::Table, FieldRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(FieldIssueTypeRow::Table, FieldIssueTypeRow::IssueType)
                        .to(IssueTypeRow::Table, IssueTypeRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("field_issue_type_by_type")
                .table(FieldIssueTypeRow::Table)
                .col(FieldIssueTypeRow::IssueType)
                .to_owned(),
        ),
    ]
}

fn choices() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(FieldChoiceRow::Table)
                .col(id_column(FieldChoiceRow::Id).primary_key())
                .col(id_column(FieldChoiceRow::Field))
                .col(ColumnDef::new(FieldChoiceRow::Name).text().not_null())
                .col(ColumnDef::new(FieldChoiceRow::Color).text().not_null())
                .col(ColumnDef::new(FieldChoiceRow::Icon).text())
                .col(
                    ColumnDef::new(FieldChoiceRow::Position)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(FieldChoiceRow::RetiredAt).big_integer())
                .foreign_key(
                    ForeignKey::create()
                        .from(FieldChoiceRow::Table, FieldChoiceRow::Field)
                        .to(FieldRow::Table, FieldRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("field_option_by_field")
                .table(FieldChoiceRow::Table)
                .col(FieldChoiceRow::Field)
                .col(FieldChoiceRow::Position)
                .to_owned(),
        ),
    ]
}

fn values() -> Vec<Step> {
    let mut steps = vec![Step::Table(
        Table::create()
            .table(FieldValueRow::Table)
            .col(ColumnDef::new(FieldValueRow::EntityKind).text().not_null())
            .col(id_column(FieldValueRow::EntityId))
            .col(id_column(FieldValueRow::Field))
            .col(ColumnDef::new(FieldValueRow::ValueText).text())
            .col(ColumnDef::new(FieldValueRow::ValueNumber).big_integer())
            .col(ColumnDef::new(FieldValueRow::ValueDate).big_integer())
            .col(ColumnDef::new(FieldValueRow::ValueRef).blob())
            .col(
                ColumnDef::new(FieldValueRow::Position)
                    .big_integer()
                    .not_null(),
            )
            .primary_key(
                Index::create()
                    .col(FieldValueRow::EntityKind)
                    .col(FieldValueRow::EntityId)
                    .col(FieldValueRow::Field)
                    .col(FieldValueRow::Position),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(FieldValueRow::Table, FieldValueRow::Field)
                    .to(FieldRow::Table, FieldRow::Id),
            )
            .to_owned(),
    )];
    for (name, column) in [
        ("field_value_by_number", FieldValueRow::ValueNumber),
        ("field_value_by_date", FieldValueRow::ValueDate),
        ("field_value_by_ref", FieldValueRow::ValueRef),
    ] {
        steps.push(Step::Index(
            Index::create()
                .name(name)
                .table(FieldValueRow::Table)
                .col(FieldValueRow::Field)
                .col(column)
                .to_owned(),
        ));
    }
    steps
}

fn levels_per_type() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(RoleTypeFieldLevelRow::Table)
                .col(id_column(RoleTypeFieldLevelRow::Role))
                .col(id_column(RoleTypeFieldLevelRow::IssueType))
                .col(
                    ColumnDef::new(RoleTypeFieldLevelRow::Field)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(RoleTypeFieldLevelRow::Level)
                        .text()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(RoleTypeFieldLevelRow::Role)
                        .col(RoleTypeFieldLevelRow::IssueType)
                        .col(RoleTypeFieldLevelRow::Field),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(RoleTypeFieldLevelRow::Table, RoleTypeFieldLevelRow::Role)
                        .to(RoleRow::Table, RoleRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(
                            RoleTypeFieldLevelRow::Table,
                            RoleTypeFieldLevelRow::IssueType,
                        )
                        .to(IssueTypeRow::Table, IssueTypeRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("role_type_field_level_by_type")
                .table(RoleTypeFieldLevelRow::Table)
                .col(RoleTypeFieldLevelRow::IssueType)
                .to_owned(),
        ),
    ]
}

fn project_levels_per_type() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(ProjectTypeFieldLevelRow::Table)
                .col(id_column(ProjectTypeFieldLevelRow::Project))
                .col(id_column(ProjectTypeFieldLevelRow::Role))
                .col(id_column(ProjectTypeFieldLevelRow::IssueType))
                .col(
                    ColumnDef::new(ProjectTypeFieldLevelRow::Field)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ProjectTypeFieldLevelRow::Level)
                        .text()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(ProjectTypeFieldLevelRow::Project)
                        .col(ProjectTypeFieldLevelRow::Role)
                        .col(ProjectTypeFieldLevelRow::IssueType)
                        .col(ProjectTypeFieldLevelRow::Field),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(
                            ProjectTypeFieldLevelRow::Table,
                            ProjectTypeFieldLevelRow::Project,
                        )
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(
                            ProjectTypeFieldLevelRow::Table,
                            ProjectTypeFieldLevelRow::Role,
                        )
                        .to(RoleRow::Table, RoleRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(
                            ProjectTypeFieldLevelRow::Table,
                            ProjectTypeFieldLevelRow::IssueType,
                        )
                        .to(IssueTypeRow::Table, IssueTypeRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("project_type_field_level_by_type")
                .table(ProjectTypeFieldLevelRow::Table)
                .col(ProjectTypeFieldLevelRow::IssueType)
                .to_owned(),
        ),
    ]
}
