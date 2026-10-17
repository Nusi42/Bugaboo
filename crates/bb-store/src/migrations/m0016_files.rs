//! Migration 16: files on issues and comments (ATT-01, ATT-02, ADR-0032).
//!
//! A file is stored once, by the SHA-256 of its content, in `stored_file`; the bytes are in the
//! `files/` folder of the data folder, never in the database. An attachment points to the file and
//! to the issue, and to the comment when it was added to one. The attachment keeps no reference to
//! the person: a removed person leaves a tombstone (PRIV-04). An upload is a file on its way; it is
//! deleted a day after its last part.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{AttachmentRow, CommentRow, IssueRow, StoredFileRow, UploadRow};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = stored_files();
    steps.extend(attachments());
    steps.extend(uploads());
    steps
}

fn stored_files() -> Vec<Step> {
    vec![Step::Table(
        Table::create()
            .table(StoredFileRow::Table)
            .col(
                ColumnDef::new(StoredFileRow::Hash)
                    .blob()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(StoredFileRow::Size).big_integer().not_null())
            .col(ColumnDef::new(StoredFileRow::MediaType).text().not_null())
            .col(ColumnDef::new(StoredFileRow::Preview).text().not_null())
            .col(
                ColumnDef::new(StoredFileRow::CreatedAt)
                    .big_integer()
                    .not_null(),
            )
            .to_owned(),
    )]
}

fn attachments() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(AttachmentRow::Table)
                .col(id_column(AttachmentRow::Id).primary_key())
                .col(id_column(AttachmentRow::Issue))
                .col(ColumnDef::new(AttachmentRow::Comment).blob())
                .col(ColumnDef::new(AttachmentRow::File).blob().not_null())
                .col(ColumnDef::new(AttachmentRow::Name).text().not_null())
                // No reference to the person: a removed person leaves a tombstone (PRIV-04).
                .col(id_column(AttachmentRow::AddedBy))
                .col(
                    ColumnDef::new(AttachmentRow::AddedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(AttachmentRow::DeletedAt).big_integer())
                // Where the preview of this attachment is, its own state and not the file's: a
                // file that is stored already still gets a job of its own (ADR-0032 §4).
                .col(ColumnDef::new(AttachmentRow::Preview).text().not_null())
                .col(
                    ColumnDef::new(AttachmentRow::Version)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(AttachmentRow::Table, AttachmentRow::Issue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(AttachmentRow::Table, AttachmentRow::Comment)
                        .to(CommentRow::Table, CommentRow::Id),
                )
                // A file with an attachment is never deleted, whatever the order of two jobs.
                .foreign_key(
                    ForeignKey::create()
                        .from(AttachmentRow::Table, AttachmentRow::File)
                        .to(StoredFileRow::Table, StoredFileRow::Hash),
                )
                .to_owned(),
        ),
        // The files of an issue are listed without reading the others.
        Step::Index(
            Index::create()
                .name("attachment_by_issue")
                .table(AttachmentRow::Table)
                .col(AttachmentRow::Issue)
                .col(AttachmentRow::Id)
                .to_owned(),
        ),
        // The daily job asks whether anything points to a file.
        Step::Index(
            Index::create()
                .name("attachment_by_file")
                .table(AttachmentRow::Table)
                .col(AttachmentRow::File)
                .to_owned(),
        ),
    ]
}

fn uploads() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(UploadRow::Table)
                .col(id_column(UploadRow::Id).primary_key())
                .col(id_column(UploadRow::Issue))
                .col(ColumnDef::new(UploadRow::Comment).blob())
                .col(id_column(UploadRow::Person))
                .col(ColumnDef::new(UploadRow::Name).text().not_null())
                .col(ColumnDef::new(UploadRow::Size).big_integer().not_null())
                .col(
                    ColumnDef::new(UploadRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(UploadRow::UpdatedAt)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(UploadRow::Table, UploadRow::Issue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .to_owned(),
        ),
        // The unfinished uploads of a person are counted, and the old ones are found by the day.
        Step::Index(
            Index::create()
                .name("upload_by_person")
                .table(UploadRow::Table)
                .col(UploadRow::Person)
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("upload_by_update")
                .table(UploadRow::Table)
                .col(UploadRow::UpdatedAt)
                .to_owned(),
        ),
    ]
}
