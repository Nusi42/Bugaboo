//! Migration 8: editing without losing text (ISS-17, ISS-11, ADR-0019): the lease on a thing that
//! is being edited, the private draft of a person, and the notes for the person who was editing.
//!
//! A lock row lives while its lease runs and is deleted when it ends or is let go (no keep
//! period). A draft is kept until it is saved or thrown away, at most 30 days after its last change.
//! Locks and drafts name any record by its kind and ID, so they need no table of the record.

use bb_core::version::FIRST_VERSION;
use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::Step;
use crate::schema::{
    EditDraftRow, EditLockRow, EditNoticeRow, GroupRow, PersonRow, PolicyVersionRow, RoleRow,
};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = locks();
    steps.extend(drafts());
    steps.extend(notices());
    steps.extend(versions());
    steps
}

/// Groups and roles get the version that every change counts up and every change is made from
/// (ADR-0019): the ones that are there start at the first.
fn versions() -> Vec<Step> {
    vec![
        Step::Alter(
            Table::alter()
                .table(GroupRow::Table)
                .add_column(
                    ColumnDef::new(GroupRow::Version)
                        .big_integer()
                        .not_null()
                        .default(FIRST_VERSION),
                )
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(RoleRow::Table)
                .add_column(
                    ColumnDef::new(RoleRow::Version)
                        .big_integer()
                        .not_null()
                        .default(FIRST_VERSION),
                )
                .to_owned(),
        ),
        // The sign-up policy is two settings with one version, kept in a row of its own that no
        // change or reset of a setting touches; a policy without a row is at version 0.
        Step::Table(
            Table::create()
                .table(PolicyVersionRow::Table)
                .col(ColumnDef::new(PolicyVersionRow::Policy).text().not_null())
                .col(
                    ColumnDef::new(PolicyVersionRow::Version)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(Index::create().col(PolicyVersionRow::Policy))
                .to_owned(),
        ),
    ]
}

fn locks() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(EditLockRow::Table)
                .col(ColumnDef::new(EditLockRow::RecordKind).text().not_null())
                .col(ColumnDef::new(EditLockRow::RecordId).blob().not_null())
                .col(ColumnDef::new(EditLockRow::Part).text().not_null())
                .col(ColumnDef::new(EditLockRow::Holder).blob().not_null())
                .col(ColumnDef::new(EditLockRow::Editor).blob().not_null())
                .col(ColumnDef::new(EditLockRow::Token).blob().not_null())
                .col(
                    ColumnDef::new(EditLockRow::TakenAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(EditLockRow::ExpiresAt)
                        .big_integer()
                        .not_null(),
                )
                // One lock for each thing being edited.
                .primary_key(
                    Index::create()
                        .col(EditLockRow::RecordKind)
                        .col(EditLockRow::RecordId)
                        .col(EditLockRow::Part),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(EditLockRow::Table, EditLockRow::Holder)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("edit_lock_by_expiry")
                .table(EditLockRow::Table)
                .col(EditLockRow::ExpiresAt)
                .to_owned(),
        ),
    ]
}

fn drafts() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(EditDraftRow::Table)
                .col(ColumnDef::new(EditDraftRow::Author).blob().not_null())
                // What the draft is for, as `DraftTarget::key` writes it.
                .col(ColumnDef::new(EditDraftRow::Target).text().not_null())
                .col(ColumnDef::new(EditDraftRow::Text).text().not_null())
                .col(ColumnDef::new(EditDraftRow::BaseVersion).big_integer())
                .col(
                    ColumnDef::new(EditDraftRow::SavedAt)
                        .big_integer()
                        .not_null(),
                )
                // One draft for each person and target.
                .primary_key(
                    Index::create()
                        .col(EditDraftRow::Author)
                        .col(EditDraftRow::Target),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(EditDraftRow::Table, EditDraftRow::Author)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        // The job that deletes old drafts finds them by this.
        Step::Index(
            Index::create()
                .name("edit_draft_by_age")
                .table(EditDraftRow::Table)
                .col(EditDraftRow::SavedAt)
                .to_owned(),
        ),
    ]
}

fn notices() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(EditNoticeRow::Table)
                .col(ColumnDef::new(EditNoticeRow::Recipient).blob().not_null())
                .col(ColumnDef::new(EditNoticeRow::RecordKind).text().not_null())
                .col(ColumnDef::new(EditNoticeRow::RecordId).blob().not_null())
                .col(ColumnDef::new(EditNoticeRow::Part).text().not_null())
                .col(ColumnDef::new(EditNoticeRow::Kind).text().not_null())
                .col(ColumnDef::new(EditNoticeRow::FromPerson).blob().not_null())
                .col(ColumnDef::new(EditNoticeRow::At).big_integer().not_null())
                // A newer note of the same kind for the same thing replaces the older one.
                .primary_key(
                    Index::create()
                        .col(EditNoticeRow::Recipient)
                        .col(EditNoticeRow::RecordKind)
                        .col(EditNoticeRow::RecordId)
                        .col(EditNoticeRow::Part)
                        .col(EditNoticeRow::Kind),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(EditNoticeRow::Table, EditNoticeRow::Recipient)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(EditNoticeRow::Table, EditNoticeRow::FromPerson)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("edit_notice_by_age")
                .table(EditNoticeRow::Table)
                .col(EditNoticeRow::At)
                .to_owned(),
        ),
    ]
}
