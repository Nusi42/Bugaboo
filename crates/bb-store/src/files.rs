//! The adapter of the file port (`bb_app::files::AttachmentStore`): the rows of stored files,
//! attachments and uploads (ATT-01, ATT-02, ADR-0032). The bytes are in the `files/` folder, not
//! here.
//!
//! A file is stored once by its hash. The row of a file is deleted only by one statement that looks
//! for an attachment again, and the attachment holds a foreign key to the file, so a file that has
//! an attachment can never be deleted, whatever the order of two jobs.

use bb_app::files::AttachmentStore;
use bb_core::files::{Attachment, MediaType, PreviewStatus, StoredFile, Upload};
use bb_core::id::{AttachmentId, CommentId, IssueId, UploadId, UserId};
use bb_core::security_log::Digest;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{LockClass, Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{AttachmentRow, StoredFileRow, UploadRow};

fn digest(row: &Row, column: StoredFileRow) -> Result<Digest, StoreError> {
    Digest::from_slice(&row.get::<Vec<u8>>(column)?).ok_or(StoreError::Corrupt("a file hash"))
}

fn stored_file_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            StoredFileRow::Hash,
            StoredFileRow::Size,
            StoredFileRow::MediaType,
            StoredFileRow::Preview,
            StoredFileRow::CreatedAt,
        ])
        .from(StoredFileRow::Table);
    select
}

fn read_stored_file(row: &Row) -> Result<StoredFile, StoreError> {
    Ok(StoredFile {
        hash: digest(row, StoredFileRow::Hash)?,
        size: row.get(StoredFileRow::Size)?,
        media_type: MediaType::new(&row.get::<String>(StoredFileRow::MediaType)?)
            .ok_or(StoreError::Corrupt("a file type"))?,
        preview: PreviewStatus::from_code(&row.get::<String>(StoredFileRow::Preview)?)
            .ok_or(StoreError::Corrupt("a preview state"))?,
        created_at: row.time(StoredFileRow::CreatedAt)?,
    })
}

fn attachment_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            AttachmentRow::Id,
            AttachmentRow::Issue,
            AttachmentRow::Comment,
            AttachmentRow::File,
            AttachmentRow::Name,
            AttachmentRow::AddedBy,
            AttachmentRow::AddedAt,
            AttachmentRow::DeletedAt,
            AttachmentRow::Preview,
            AttachmentRow::Version,
        ])
        .from(AttachmentRow::Table);
    select
}

fn read_attachment(row: &Row) -> Result<Attachment, StoreError> {
    Ok(Attachment {
        id: row.id::<AttachmentId>(AttachmentRow::Id)?,
        issue: row.id::<IssueId>(AttachmentRow::Issue)?,
        comment: row.optional_id::<CommentId>(AttachmentRow::Comment)?,
        file: Digest::from_slice(&row.get::<Vec<u8>>(AttachmentRow::File)?)
            .ok_or(StoreError::Corrupt("a file hash"))?,
        name: row.get(AttachmentRow::Name)?,
        added_by: row.id::<UserId>(AttachmentRow::AddedBy)?,
        added_at: row.time(AttachmentRow::AddedAt)?,
        deleted_at: row.optional_time(AttachmentRow::DeletedAt)?,
        preview: PreviewStatus::from_code(&row.get::<String>(AttachmentRow::Preview)?)
            .ok_or(StoreError::Corrupt("a preview state"))?,
        version: row.get(AttachmentRow::Version)?,
    })
}

fn upload_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            UploadRow::Id,
            UploadRow::Issue,
            UploadRow::Comment,
            UploadRow::Person,
            UploadRow::Name,
            UploadRow::Size,
            UploadRow::CreatedAt,
            UploadRow::UpdatedAt,
        ])
        .from(UploadRow::Table);
    select
}

fn read_upload(row: &Row) -> Result<Upload, StoreError> {
    Ok(Upload {
        id: row.id::<UploadId>(UploadRow::Id)?,
        issue: row.id::<IssueId>(UploadRow::Issue)?,
        comment: row.optional_id::<CommentId>(UploadRow::Comment)?,
        person: row.id::<UserId>(UploadRow::Person)?,
        name: row.get(UploadRow::Name)?,
        size: row.get(UploadRow::Size)?,
        created_at: row.time(UploadRow::CreatedAt)?,
        updated_at: row.time(UploadRow::UpdatedAt)?,
    })
}

/// The values of an attachment in the order of the columns of the insert below.
fn bytes_of(hash: Digest) -> Vec<u8> {
    hash.as_bytes().to_vec()
}

/// The select of the hashes of the files some attachment points to.
fn pointed_to() -> SelectStatement {
    let mut select = Query::select();
    select
        .column(AttachmentRow::File)
        .from(AttachmentRow::Table);
    select
}

impl AttachmentStore for AuthStorage {
    async fn attachment_in(
        transaction: &mut Transaction,
        id: AttachmentId,
    ) -> Result<Option<Attachment>, StoreError> {
        let mut select = attachment_select();
        select.and_where(Expr::col(AttachmentRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_attachment)
            .transpose()
    }

    async fn upload_in(
        transaction: &mut Transaction,
        id: UploadId,
    ) -> Result<Option<Upload>, StoreError> {
        let mut select = upload_select();
        select.and_where(Expr::col(UploadRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_upload)
            .transpose()
    }

    async fn stored_file_in(
        transaction: &mut Transaction,
        hash: Digest,
    ) -> Result<Option<StoredFile>, StoreError> {
        let mut select = stored_file_select();
        select.and_where(Expr::col(StoredFileRow::Hash).eq(bytes_of(hash)));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_stored_file)
            .transpose()
    }

    async fn attachments_of_issue(&self, issue: IssueId) -> Result<Vec<Attachment>, StoreError> {
        let mut select = attachment_select();
        select
            .and_where(Expr::col(AttachmentRow::Issue).eq(issue.stored()))
            .and_where(Expr::col(AttachmentRow::DeletedAt).is_null())
            .order_by(AttachmentRow::Id, Order::Asc);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_attachment)
            .collect()
    }

    async fn stored_files(&self, hashes: &[Digest]) -> Result<Vec<StoredFile>, StoreError> {
        if hashes.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = stored_file_select();
        select.and_where(
            Expr::col(StoredFileRow::Hash)
                .is_in(hashes.iter().copied().map(bytes_of).collect::<Vec<_>>()),
        );
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_stored_file)
            .collect()
    }

    async fn attachment_count_of_issue(&self, issue: IssueId) -> Result<u64, StoreError> {
        let mut select = Query::select();
        select
            .expr_as(
                Expr::col(AttachmentRow::Id).count(),
                crate::schema::Computed::Total,
            )
            .from(AttachmentRow::Table)
            .and_where(Expr::col(AttachmentRow::Issue).eq(issue.stored()));
        match self.database().fetch_optional(&select).await? {
            Some(row) => u64::try_from(row.get::<i64>(crate::schema::Computed::Total)?)
                .map_err(|_| StoreError::Corrupt("a count")),
            None => Ok(0),
        }
    }

    async fn try_insert_upload(
        &self,
        upload: &Upload,
        open_max: u64,
        bytes_max: u64,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.database().begin().await?;
        // The lock is named by the person: four random bytes of their ID.
        let mut tail = [0_u8; 4];
        tail.copy_from_slice(&upload.person.to_bytes()[12..]);
        transaction
            .advisory_lock(LockClass::Uploads, i32::from_be_bytes(tail))
            .await?;
        let mut select = Query::select();
        select
            .column(UploadRow::Size)
            .from(UploadRow::Table)
            .and_where(Expr::col(UploadRow::Person).eq(upload.person.stored()));
        let open = transaction.fetch_all(&select).await?;
        let mut declared = u64::try_from(upload.size).unwrap_or(u64::MAX);
        for row in &open {
            declared = declared
                .saturating_add(u64::try_from(row.get::<i64>(UploadRow::Size)?).unwrap_or(0));
        }
        if u64::try_from(open.len()).unwrap_or(u64::MAX) >= open_max || declared > bytes_max {
            return Ok(false);
        }
        let mut insert = Query::insert();
        insert
            .into_table(UploadRow::Table)
            .columns([
                UploadRow::Id,
                UploadRow::Issue,
                UploadRow::Comment,
                UploadRow::Person,
                UploadRow::Name,
                UploadRow::Size,
                UploadRow::CreatedAt,
                UploadRow::UpdatedAt,
            ])
            .values([
                Expr::val(upload.id.stored()),
                Expr::val(upload.issue.stored()),
                Expr::val(upload.comment.map(StoredId::stored)),
                Expr::val(upload.person.stored()),
                Expr::val(upload.name.as_str()),
                Expr::val(upload.size),
                Expr::val(to_micros(upload.created_at)),
                Expr::val(to_micros(upload.updated_at)),
            ])
            .map_err(|_| StoreError::Corrupt("an upload"))?;
        transaction.execute(&insert).await?;
        transaction.commit().await?;
        Ok(true)
    }

    async fn uploads_untouched_since(
        &self,
        before: Timestamp,
        limit: u64,
    ) -> Result<Vec<Upload>, StoreError> {
        let mut select = upload_select();
        select
            .and_where(Expr::col(UploadRow::UpdatedAt).lt(to_micros(before)))
            .order_by(UploadRow::UpdatedAt, Order::Asc)
            .limit(limit);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_upload)
            .collect()
    }

    async fn touch_upload(&self, id: UploadId, at: Timestamp) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(UploadRow::Table)
            .value(UploadRow::UpdatedAt, to_micros(at))
            .and_where(Expr::col(UploadRow::Id).eq(id.stored()));
        self.database().execute(&update).await.map(|_| ())
    }

    async fn delete_upload_row(&self, id: UploadId) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(UploadRow::Table)
            .and_where(Expr::col(UploadRow::Id).eq(id.stored()));
        Ok(self.database().execute(&delete).await? > 0)
    }

    async fn set_preview_status(
        &self,
        hash: Digest,
        state: PreviewStatus,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(StoredFileRow::Table)
            .value(StoredFileRow::Preview, state.code())
            .and_where(Expr::col(StoredFileRow::Hash).eq(bytes_of(hash)));
        Ok(self.database().execute(&update).await? > 0)
    }

    async fn set_attachment_preview(
        &self,
        id: AttachmentId,
        state: PreviewStatus,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(AttachmentRow::Table)
            .value(AttachmentRow::Preview, state.code())
            .and_where(Expr::col(AttachmentRow::Id).eq(id.stored()));
        Ok(self.database().execute(&update).await? > 0)
    }

    async fn purge_trashed_attachments(
        &self,
        before: Timestamp,
        limit: u64,
    ) -> Result<u64, StoreError> {
        // The ones to delete are chosen first, so a limit holds on both databases.
        let mut select = Query::select();
        select
            .column(AttachmentRow::Id)
            .from(AttachmentRow::Table)
            .and_where(Expr::col(AttachmentRow::DeletedAt).lt(to_micros(before)))
            .order_by(AttachmentRow::Id, Order::Asc)
            .limit(limit);
        let mut delete = Query::delete();
        delete
            .from_table(AttachmentRow::Table)
            .and_where(Expr::col(AttachmentRow::Id).in_subquery(select));
        self.database().execute(&delete).await
    }

    async fn unreferenced_files(&self, limit: u64) -> Result<Vec<StoredFile>, StoreError> {
        let mut select = stored_file_select();
        select
            .and_where(Expr::col(StoredFileRow::Hash).not_in_subquery(pointed_to()))
            .order_by(StoredFileRow::CreatedAt, Order::Asc)
            .limit(limit);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_stored_file)
            .collect()
    }

    async fn delete_unreferenced_file(&self, hash: Digest) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(StoredFileRow::Table)
            .and_where(Expr::col(StoredFileRow::Hash).eq(bytes_of(hash)))
            .and_where(Expr::col(StoredFileRow::Hash).not_in_subquery(pointed_to()));
        Ok(self.database().execute(&delete).await? > 0)
    }

    async fn lock_issue_files(
        transaction: &mut Transaction,
        issue: IssueId,
    ) -> Result<(), StoreError> {
        // The lock is named by the issue: four random bytes of its ID.
        let mut tail = [0_u8; 4];
        tail.copy_from_slice(&issue.to_bytes()[12..]);
        transaction
            .advisory_lock(LockClass::IssueFiles, i32::from_be_bytes(tail))
            .await
    }

    async fn attachment_count_in_change(
        transaction: &mut Transaction,
        issue: IssueId,
    ) -> Result<u64, StoreError> {
        let mut select = Query::select();
        select
            .expr_as(
                Expr::col(AttachmentRow::Id).count(),
                crate::schema::Computed::Total,
            )
            .from(AttachmentRow::Table)
            .and_where(Expr::col(AttachmentRow::Issue).eq(issue.stored()));
        match transaction.fetch_optional(&select).await? {
            Some(row) => u64::try_from(row.get::<i64>(crate::schema::Computed::Total)?)
                .map_err(|_| StoreError::Corrupt("a count")),
            None => Ok(0),
        }
    }

    async fn delete_upload(
        transaction: &mut Transaction,
        id: UploadId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(UploadRow::Table)
            .and_where(Expr::col(UploadRow::Id).eq(id.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn insert_stored_file(
        transaction: &mut Transaction,
        file: &StoredFile,
    ) -> Result<bool, StoreError> {
        if Self::stored_file_in(transaction, file.hash)
            .await?
            .is_some()
        {
            return Ok(false);
        }
        let mut insert = Query::insert();
        insert
            .into_table(StoredFileRow::Table)
            .columns([
                StoredFileRow::Hash,
                StoredFileRow::Size,
                StoredFileRow::MediaType,
                StoredFileRow::Preview,
                StoredFileRow::CreatedAt,
            ])
            .values([
                Expr::val(bytes_of(file.hash)),
                Expr::val(file.size),
                Expr::val(file.media_type.as_str()),
                Expr::val(file.preview.code()),
                Expr::val(to_micros(file.created_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a file"))?;
        transaction.execute(&insert).await?;
        Ok(true)
    }

    async fn insert_attachment(
        transaction: &mut Transaction,
        attachment: &Attachment,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(AttachmentRow::Table)
            .columns([
                AttachmentRow::Id,
                AttachmentRow::Issue,
                AttachmentRow::Comment,
                AttachmentRow::File,
                AttachmentRow::Name,
                AttachmentRow::AddedBy,
                AttachmentRow::AddedAt,
                AttachmentRow::DeletedAt,
                AttachmentRow::Preview,
                AttachmentRow::Version,
            ])
            .values([
                Expr::val(attachment.id.stored()),
                Expr::val(attachment.issue.stored()),
                Expr::val(attachment.comment.map(StoredId::stored)),
                Expr::val(bytes_of(attachment.file)),
                Expr::val(attachment.name.as_str()),
                Expr::val(attachment.added_by.stored()),
                Expr::val(to_micros(attachment.added_at)),
                Expr::val(attachment.deleted_at.map(to_micros)),
                Expr::val(attachment.preview.code()),
                Expr::val(attachment.version),
            ])
            .map_err(|_| StoreError::Corrupt("an attachment"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn set_attachment_deleted(
        transaction: &mut Transaction,
        id: AttachmentId,
        deleted_at: Option<Timestamp>,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(AttachmentRow::Table)
            .value(AttachmentRow::DeletedAt, deleted_at.map(to_micros))
            .value(
                AttachmentRow::Version,
                Expr::col(AttachmentRow::Version).add(1),
            )
            .and_where(Expr::col(AttachmentRow::Id).eq(id.stored()));
        // Only an attachment that is where the change expects it: not in the trash to put it
        // there, in the trash to take it out.
        update.and_where(if deleted_at.is_some() {
            Expr::col(AttachmentRow::DeletedAt).is_null()
        } else {
            Expr::col(AttachmentRow::DeletedAt).is_not_null()
        });
        Ok(transaction.execute(&update).await? > 0)
    }
}

#[cfg(test)]
#[path = "tests/files.rs"]
mod tests;
