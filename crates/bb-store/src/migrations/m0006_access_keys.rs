//! Migration 6: access keys (AUTH-07, ADR-0025 decision 8). A key belongs to a person, keeps only
//! the hash of its secret, and says what it may do, in which projects, and when it runs out.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::Step;
use crate::schema::{
    AccessKeyPermissionRow, AccessKeyProjectRow, AccessKeyRow, PersonRow, ProjectRow,
};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = keys();
    steps.extend(scope());
    steps
}

fn keys() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(AccessKeyRow::Table)
                .col(
                    ColumnDef::new(AccessKeyRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(AccessKeyRow::Owner).blob().not_null())
                .col(ColumnDef::new(AccessKeyRow::Name).text().not_null())
                // The public part of the key. Unique, so a person can tell their keys apart and
                // the list never shows two with the same start.
                .col(
                    ColumnDef::new(AccessKeyRow::Prefix)
                        .text()
                        .not_null()
                        .unique_key(),
                )
                // The key is found by this hash, so a wrong secret and an unknown key are the
                // same lookup that finds nothing.
                .col(
                    ColumnDef::new(AccessKeyRow::SecretHash)
                        .blob()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(AccessKeyRow::Choice).text().not_null())
                .col(
                    ColumnDef::new(AccessKeyRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(AccessKeyRow::ExpiresAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(AccessKeyRow::LastUsedAt).big_integer())
                .foreign_key(
                    ForeignKey::create()
                        .from(AccessKeyRow::Table, AccessKeyRow::Owner)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("access_key_by_owner")
                .table(AccessKeyRow::Table)
                .col(AccessKeyRow::Owner)
                .col(AccessKeyRow::CreatedAt)
                .to_owned(),
        ),
    ]
}

/// What a key may do (single permissions) and where (projects).
fn scope() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(AccessKeyPermissionRow::Table)
                .col(
                    ColumnDef::new(AccessKeyPermissionRow::Key)
                        .blob()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(AccessKeyPermissionRow::Permission)
                        .text()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(AccessKeyPermissionRow::Key)
                        .col(AccessKeyPermissionRow::Permission),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(AccessKeyPermissionRow::Table, AccessKeyPermissionRow::Key)
                        .to(AccessKeyRow::Table, AccessKeyRow::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(AccessKeyProjectRow::Table)
                .col(ColumnDef::new(AccessKeyProjectRow::Key).blob().not_null())
                .col(
                    ColumnDef::new(AccessKeyProjectRow::Project)
                        .blob()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(AccessKeyProjectRow::Key)
                        .col(AccessKeyProjectRow::Project),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(AccessKeyProjectRow::Table, AccessKeyProjectRow::Key)
                        .to(AccessKeyRow::Table, AccessKeyRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(AccessKeyProjectRow::Table, AccessKeyProjectRow::Project)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("access_key_project_by_project")
                .table(AccessKeyProjectRow::Table)
                .col(AccessKeyProjectRow::Project)
                .to_owned(),
        ),
    ]
}
