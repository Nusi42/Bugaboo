//! Migration 5: the old short names of projects (PRJ-01, M2-Q4).
//!
//! A project can change its short name. The old one stays with the project (`project_alias`), so
//! addresses that used it keep working and nobody else can take it. The name of the table says
//! what it is for; its key is the old short name, which is unique in the organization together
//! with the current short names of `project` (the use case checks both inside the change, after
//! the policy lock).

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::Step;
use crate::schema::{ProjectAliasRow, ProjectRow};

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(ProjectAliasRow::Table)
                .col(
                    ColumnDef::new(ProjectAliasRow::ShortName)
                        .text()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(ProjectAliasRow::Project).blob().not_null())
                .col(
                    ColumnDef::new(ProjectAliasRow::RetiredAt)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(ProjectAliasRow::Table, ProjectAliasRow::Project)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("project_alias_by_project")
                .table(ProjectAliasRow::Table)
                .col(ProjectAliasRow::Project)
                .to_owned(),
        ),
    ]
}
