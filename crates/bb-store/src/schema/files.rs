//! The tables of files, attachments and uploads (ATT-01, ATT-02, ADR-0032).

use super::{Iden, StoredTable};

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
    Preview,
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

stored_tables! {
    StoredFileRow => bb_core::files::StoredFile {
        Hash => hash,
        Size => size,
        MediaType => media_type,
        Preview => preview,
        CreatedAt => created_at,
    },
    AttachmentRow => bb_core::files::Attachment {
        Id => id,
        Issue => issue,
        Comment => comment,
        File => file,
        Name => name,
        AddedBy => added_by,
        AddedAt => added_at,
        DeletedAt => deleted_at,
        Preview => preview,
        Version => version,
    },
    UploadRow => bb_core::files::Upload {
        Id => id,
        Issue => issue,
        Comment => comment,
        Person => person,
        Name => name,
        Size => size,
        CreatedAt => created_at,
        UpdatedAt => updated_at,
    },
}
