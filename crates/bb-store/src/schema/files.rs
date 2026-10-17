//! The tables of files, attachments and uploads (ATT-01, ATT-02, ADR-0032).

use super::Iden;

/// A file as it is stored: once, by the hash of its content (ATT-02).
#[derive(Clone, Copy, Iden)]
pub(crate) enum StoredFileRow {
    #[iden = "stored_file"]
    Table,
    Hash,
    Size,
    MediaType,
    Preview,
    CreatedAt,
}

/// A file added to an issue or a comment (ATT-01).
#[derive(Clone, Copy, Iden)]
pub(crate) enum AttachmentRow {
    #[iden = "attachment"]
    Table,
    Id,
    Issue,
    Comment,
    File,
    Name,
    AddedBy,
    AddedAt,
    DeletedAt,
    Version,
}

/// A file on its way in parts (ATT-01).
#[derive(Clone, Copy, Iden)]
pub(crate) enum UploadRow {
    #[iden = "upload"]
    Table,
    Id,
    Issue,
    Comment,
    Person,
    Name,
    Size,
    CreatedAt,
    UpdatedAt,
}
