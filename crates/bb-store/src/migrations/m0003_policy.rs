//! Migration 3: projects, groups, roles, who has which role, field levels and *Who sees people*
//! (SEC-01, SEC-02, SEC-04, SEC-08, PRIV-12, ADR-0025).
//!
//! The administrator flag of M1 becomes a list (`legacy_administrator`) and the column is taken
//! away: the first start gives those people the Administrator role, in the same change that makes
//! the starting roles (ADR-0025, decision 5). The roles themselves are never made here (a change
//! of the definition in an update would otherwise change every organization's roles). The text
//! setting `security.two-step-required-roles` goes with its stored values: a switch on the role
//! replaces it (ADR-0025, decision 7).

use sea_query::{ColumnDef, Expr, ExprTrait as _, ForeignKey, Index, Query, Table};

use super::Step;
use crate::schema::{
    AssignmentRow, GroupMemberRow, GroupRow, LegacyAdministratorRow, PersonRow,
    ProjectFieldLevelRow, ProjectPeopleLevelRow, ProjectRow, Retired, RoleFieldLevelRow,
    RolePeopleLevelRow, RolePermissionRow, RoleRow, SettingRow, StartingRolesMadeRow,
};

/// The setting that the switch on the role replaces. Its stored values are deleted.
const REMOVED_SETTING: &str = "security.two-step-required-roles";

pub(super) fn steps() -> Result<Vec<Step>, sea_query::error::Error> {
    let mut steps = Vec::new();
    steps.extend(projects());
    steps.extend(groups());
    steps.extend(roles());
    steps.extend(assignments());
    steps.extend(field_levels());
    steps.extend(people_levels());
    steps.extend(starting_roles_and_old_administrators());
    steps.extend(retire_the_flag_and_the_setting()?);
    Ok(steps)
}

fn id_column(name: impl sea_query::Iden + 'static) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.blob().not_null();
    column
}

fn projects() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(ProjectRow::Table)
                .col(id_column(ProjectRow::Id).primary_key())
                .col(ColumnDef::new(ProjectRow::Parent).blob())
                .col(
                    ColumnDef::new(ProjectRow::ShortName)
                        .text()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(ProjectRow::Name).text().not_null())
                .col(ColumnDef::new(ProjectRow::Version).big_integer().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .from(ProjectRow::Table, ProjectRow::Parent)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("project_by_parent")
                .table(ProjectRow::Table)
                .col(ProjectRow::Parent)
                .to_owned(),
        ),
    ]
}

fn groups() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(GroupRow::Table)
                .col(id_column(GroupRow::Id).primary_key())
                .col(
                    ColumnDef::new(GroupRow::Name)
                        .text()
                        .not_null()
                        .unique_key(),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(GroupMemberRow::Table)
                .col(id_column(GroupMemberRow::Group))
                .col(id_column(GroupMemberRow::Person))
                .col(
                    ColumnDef::new(GroupMemberRow::AddedAt)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(GroupMemberRow::Group)
                        .col(GroupMemberRow::Person),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(GroupMemberRow::Table, GroupMemberRow::Group)
                        .to(GroupRow::Table, GroupRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(GroupMemberRow::Table, GroupMemberRow::Person)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("group_member_by_person")
                .table(GroupMemberRow::Table)
                .col(GroupMemberRow::Person)
                .to_owned(),
        ),
    ]
}

fn roles() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(RoleRow::Table)
                .col(id_column(RoleRow::Id).primary_key())
                .col(ColumnDef::new(RoleRow::Name).text().not_null().unique_key())
                .col(ColumnDef::new(RoleRow::Starting).text())
                .col(ColumnDef::new(RoleRow::Position).big_integer().not_null())
                .col(ColumnDef::new(RoleRow::TwoStep).boolean().not_null())
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(RolePermissionRow::Table)
                .col(id_column(RolePermissionRow::Role))
                .col(
                    ColumnDef::new(RolePermissionRow::Permission)
                        .text()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(RolePermissionRow::Role)
                        .col(RolePermissionRow::Permission),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(RolePermissionRow::Table, RolePermissionRow::Role)
                        .to(RoleRow::Table, RoleRow::Id),
                )
                .to_owned(),
        ),
    ]
}

fn assignments() -> Vec<Step> {
    let mut steps = vec![Step::Table(
        Table::create()
            .table(AssignmentRow::Table)
            .col(id_column(AssignmentRow::Id).primary_key())
            .col(ColumnDef::new(AssignmentRow::Person).blob())
            .col(ColumnDef::new(AssignmentRow::Group).blob())
            // No role means *No access*.
            .col(ColumnDef::new(AssignmentRow::Role).blob())
            // No project means the whole organization.
            .col(ColumnDef::new(AssignmentRow::Project).blob())
            .col(ColumnDef::new(AssignmentRow::AppliesTo).text().not_null())
            .col(
                ColumnDef::new(AssignmentRow::GivenAt)
                    .big_integer()
                    .not_null(),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(AssignmentRow::Table, AssignmentRow::Person)
                    .to(PersonRow::Table, PersonRow::Id),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(AssignmentRow::Table, AssignmentRow::Group)
                    .to(GroupRow::Table, GroupRow::Id),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(AssignmentRow::Table, AssignmentRow::Role)
                    .to(RoleRow::Table, RoleRow::Id),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(AssignmentRow::Table, AssignmentRow::Project)
                    .to(ProjectRow::Table, ProjectRow::Id),
            )
            .to_owned(),
    )];
    for (name, column) in [
        ("assignment_by_person", AssignmentRow::Person),
        ("assignment_by_group", AssignmentRow::Group),
        ("assignment_by_project", AssignmentRow::Project),
        ("assignment_by_role", AssignmentRow::Role),
    ] {
        steps.push(Step::Index(
            Index::create()
                .name(name)
                .table(AssignmentRow::Table)
                .col(column)
                .to_owned(),
        ));
    }
    steps
}

fn field_levels() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(RoleFieldLevelRow::Table)
                .col(id_column(RoleFieldLevelRow::Role))
                .col(ColumnDef::new(RoleFieldLevelRow::Field).text().not_null())
                .col(ColumnDef::new(RoleFieldLevelRow::Level).text().not_null())
                .primary_key(
                    Index::create()
                        .col(RoleFieldLevelRow::Role)
                        .col(RoleFieldLevelRow::Field),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(RoleFieldLevelRow::Table, RoleFieldLevelRow::Role)
                        .to(RoleRow::Table, RoleRow::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(ProjectFieldLevelRow::Table)
                .col(id_column(ProjectFieldLevelRow::Project))
                .col(id_column(ProjectFieldLevelRow::Role))
                .col(
                    ColumnDef::new(ProjectFieldLevelRow::Field)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ProjectFieldLevelRow::Level)
                        .text()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(ProjectFieldLevelRow::Project)
                        .col(ProjectFieldLevelRow::Role)
                        .col(ProjectFieldLevelRow::Field),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(ProjectFieldLevelRow::Table, ProjectFieldLevelRow::Project)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(ProjectFieldLevelRow::Table, ProjectFieldLevelRow::Role)
                        .to(RoleRow::Table, RoleRow::Id),
                )
                .to_owned(),
        ),
    ]
}

fn people_levels() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(RolePeopleLevelRow::Table)
                .col(id_column(RolePeopleLevelRow::Role))
                .col(ColumnDef::new(RolePeopleLevelRow::Kind).text().not_null())
                .col(ColumnDef::new(RolePeopleLevelRow::Level).text().not_null())
                .primary_key(
                    Index::create()
                        .col(RolePeopleLevelRow::Role)
                        .col(RolePeopleLevelRow::Kind),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(RolePeopleLevelRow::Table, RolePeopleLevelRow::Role)
                        .to(RoleRow::Table, RoleRow::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(ProjectPeopleLevelRow::Table)
                .col(id_column(ProjectPeopleLevelRow::Project))
                .col(id_column(ProjectPeopleLevelRow::Role))
                .col(
                    ColumnDef::new(ProjectPeopleLevelRow::Kind)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ProjectPeopleLevelRow::Level)
                        .text()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(ProjectPeopleLevelRow::Project)
                        .col(ProjectPeopleLevelRow::Role)
                        .col(ProjectPeopleLevelRow::Kind),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(ProjectPeopleLevelRow::Table, ProjectPeopleLevelRow::Project)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(ProjectPeopleLevelRow::Table, ProjectPeopleLevelRow::Role)
                        .to(RoleRow::Table, RoleRow::Id),
                )
                .to_owned(),
        ),
    ]
}

fn starting_roles_and_old_administrators() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(LegacyAdministratorRow::Table)
                .col(id_column(LegacyAdministratorRow::Person).primary_key())
                .foreign_key(
                    ForeignKey::create()
                        .from(
                            LegacyAdministratorRow::Table,
                            LegacyAdministratorRow::Person,
                        )
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(StartingRolesMadeRow::Table)
                .col(
                    ColumnDef::new(StartingRolesMadeRow::MadeAt)
                        .big_integer()
                        .not_null()
                        .primary_key(),
                )
                .to_owned(),
        ),
    ]
}

/// Keeps the list of M1 administrators, then drops the flag and the setting that the roles replace.
fn retire_the_flag_and_the_setting() -> Result<Vec<Step>, sea_query::error::Error> {
    let mut administrators = Query::select();
    administrators
        .column(PersonRow::Id)
        .from(PersonRow::Table)
        .and_where(Expr::col(Retired::Administrator).eq(true));
    let mut keep = Query::insert();
    keep.into_table(LegacyAdministratorRow::Table)
        .columns([LegacyAdministratorRow::Person])
        .select_from(administrators)?;
    let mut forget_setting = Query::delete();
    forget_setting
        .from_table(SettingRow::Table)
        .and_where(Expr::col(SettingRow::Key).eq(REMOVED_SETTING));
    Ok(vec![
        Step::Seed(keep),
        Step::Delete(forget_setting),
        Step::Alter(
            Table::alter()
                .table(PersonRow::Table)
                .drop_column(Retired::Administrator)
                .to_owned(),
        ),
    ])
}
