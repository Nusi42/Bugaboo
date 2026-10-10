//! Migration 7: issues, issue types and issue IDs (ISS-01, ISS-01a, ISS-02, ADR-0021).
//!
//! An issue is found by its internal ID or by the readable ID of its project: the code (unique in
//! the project together with the old codes of `issue_alias`, which the use case checks inside the
//! change). The only counter in the product is `issue_counter`, the last counting code a project
//! gave out (ADR-0021, `tools/checks/counter-exemptions.toml`).

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::Step;
use crate::schema::{
    IssueAliasRow, IssueAssigneeRow, IssueCounterRow, IssueRow, IssueTypeRow, ProjectRow,
};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = types();
    steps.extend(issues());
    steps.extend(assignees_and_codes());
    steps
}

fn types() -> Vec<Step> {
    vec![Step::Table(
        Table::create()
            .table(IssueTypeRow::Table)
            .col(
                ColumnDef::new(IssueTypeRow::Id)
                    .blob()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(IssueTypeRow::Name).text().not_null())
            .col(ColumnDef::new(IssueTypeRow::Icon).text().not_null())
            .col(
                ColumnDef::new(IssueTypeRow::Position)
                    .big_integer()
                    .not_null(),
            )
            .col(ColumnDef::new(IssueTypeRow::RetiredAt).big_integer())
            .col(
                ColumnDef::new(IssueTypeRow::Version)
                    .big_integer()
                    .not_null(),
            )
            .to_owned(),
    )]
}

fn issues() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(IssueRow::Table)
                .col(ColumnDef::new(IssueRow::Id).blob().not_null().primary_key())
                .col(ColumnDef::new(IssueRow::Project).blob().not_null())
                .col(ColumnDef::new(IssueRow::IssueType).blob().not_null())
                .col(ColumnDef::new(IssueRow::Code).text().not_null())
                .col(ColumnDef::new(IssueRow::Title).text().not_null())
                .col(ColumnDef::new(IssueRow::Description).text().not_null())
                .col(ColumnDef::new(IssueRow::Status).text().not_null())
                .col(ColumnDef::new(IssueRow::Priority).text().not_null())
                // No reference to the person: a removed person leaves a tombstone, not a hole
                // (PRIV-04), and the issue keeps who reported it.
                .col(ColumnDef::new(IssueRow::ReportedBy).blob().not_null())
                .col(
                    ColumnDef::new(IssueRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(IssueRow::UpdatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(IssueRow::ClosedAt).big_integer())
                .col(ColumnDef::new(IssueRow::DeletedAt).big_integer())
                .col(ColumnDef::new(IssueRow::Version).big_integer().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .from(IssueRow::Table, IssueRow::Project)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(IssueRow::Table, IssueRow::IssueType)
                        .to(IssueTypeRow::Table, IssueTypeRow::Id),
                )
                .to_owned(),
        ),
        // A code is given out once in a project: two writers that draw the same one cannot both
        // succeed.
        Step::Index(
            Index::create()
                .name("issue_by_code")
                .table(IssueRow::Table)
                .col(IssueRow::Project)
                .col(IssueRow::Code)
                .unique()
                .to_owned(),
        ),
        // The pages of the list of a project, newest first, and the pages of the issues one
        // person reported.
        Step::Index(
            Index::create()
                .name("issue_by_project")
                .table(IssueRow::Table)
                .col(IssueRow::Project)
                .col(IssueRow::Id)
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("issue_by_reporter")
                .table(IssueRow::Table)
                .col(IssueRow::Project)
                .col(IssueRow::ReportedBy)
                .col(IssueRow::Id)
                .to_owned(),
        ),
    ]
}

fn assignees_and_codes() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(IssueAssigneeRow::Table)
                .col(ColumnDef::new(IssueAssigneeRow::Issue).blob().not_null())
                .col(ColumnDef::new(IssueAssigneeRow::Person).blob().not_null())
                .col(
                    ColumnDef::new(IssueAssigneeRow::AddedAt)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(IssueAssigneeRow::Issue)
                        .col(IssueAssigneeRow::Person),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(IssueAssigneeRow::Table, IssueAssigneeRow::Issue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(IssueAliasRow::Table)
                .col(ColumnDef::new(IssueAliasRow::Project).blob().not_null())
                .col(ColumnDef::new(IssueAliasRow::Code).text().not_null())
                .col(ColumnDef::new(IssueAliasRow::Issue).blob().not_null())
                .col(
                    ColumnDef::new(IssueAliasRow::RetiredAt)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(IssueAliasRow::Project)
                        .col(IssueAliasRow::Code),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(IssueAliasRow::Table, IssueAliasRow::Project)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(IssueAliasRow::Table, IssueAliasRow::Issue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("issue_alias_by_issue")
                .table(IssueAliasRow::Table)
                .col(IssueAliasRow::Issue)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(IssueCounterRow::Table)
                .col(
                    ColumnDef::new(IssueCounterRow::Project)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(IssueCounterRow::Last)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(IssueCounterRow::Table, IssueCounterRow::Project)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .to_owned(),
        ),
    ]
}
