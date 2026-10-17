//! The port of the file use cases: what they need from the database beyond the comments port.
//!
//! The bytes of a file are not here: they sit behind [`FileStore`](super::FileStore). The database
//! holds only the rows that say what a file is, who added it and where.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::files::{Attachment, PreviewStatus, StoredFile, Upload};
use bb_core::id::{AttachmentId, IssueId, UploadId};
use bb_core::security_log::Digest;
use jiff::Timestamp;

use crate::comments::CommentStore;
use crate::store_macros::{queries, reads, writes};

/// What the file use cases need from the database.
pub trait AttachmentStore: CommentStore + 'static {
    queries! {
        /// The attachment with this ID, also one in the trash.
        fn attachment(id: AttachmentId) -> Option<Attachment> = attachment_in;
        /// The upload with this ID.
        fn upload(id: UploadId) -> Option<Upload> = upload_in;
        /// The file with this hash.
        fn stored_file(hash: Digest) -> Option<StoredFile> = stored_file_in;
    }

    reads! {
        /// The attachments of an issue that are not in the trash, oldest first. Whether the
        /// comment of one is in the trash is for the use case to ask.
        fn attachments_of_issue(issue: IssueId) -> Vec<Attachment>;
        /// How many attachments an issue has, the ones in the trash too: they come back with
        /// *Undo* until the daily clean-up removes them, so they count toward the limit.
        fn attachment_count_of_issue(issue: IssueId) -> u64;
        /// The attachments with these IDs that exist, the ones in the trash too: read once for a
        /// page of the history.
        fn attachments_by_ids(ids: &[AttachmentId]) -> Vec<Attachment>;
        /// The files with these hashes that exist.
        fn stored_files(hashes: &[Digest]) -> Vec<StoredFile>;
        /// Starts an upload, unless the person has `open_max` open uploads already or the sizes
        /// they declared, with this one, would be more than `bytes_max`; says whether it was
        /// started. The count, the sum and the insert are one step, so two uploads started at the
        /// same moment cannot both pass. An upload is a file on its way, no record of the issue
        /// yet, so starting one is no change with an event.
        fn try_insert_upload(upload: &Upload, open_max: u64, bytes_max: u64) -> bool;
        /// The uploads whose last part (or start) was before `before`, at most `limit`.
        fn uploads_untouched_since(before: Timestamp, limit: u64) -> Vec<Upload>;
        /// Notes that a part of an upload came, so the day it is kept for starts again.
        fn touch_upload(id: UploadId, at: Timestamp) -> ();
        /// Deletes the row of an upload that was cancelled, went stale or was refused; `false`
        /// when it was not there.
        fn delete_upload_row(id: UploadId) -> bool;
        /// Says where the preview of a file is; `false` when the file is not there.
        fn set_preview_status(hash: Digest, state: PreviewStatus) -> bool;
        /// Says where the preview of an attachment is (its own state, not the file's); `false`
        /// when the attachment is not there.
        fn set_attachment_preview(id: AttachmentId, state: PreviewStatus) -> bool;
        /// Deletes the attachments that went into the trash before `before` (they cannot be taken
        /// out of it any more), at most `limit`, and says how many.
        fn purge_trashed_attachments(before: Timestamp, limit: u64) -> u64;
        /// The files that no attachment points to, at most `limit`.
        fn unreferenced_files(limit: u64) -> Vec<StoredFile>;
        /// Deletes the row of a file that no attachment points to (decided in the statement
        /// itself, so an attachment added at the same moment keeps it); `false` when it is
        /// referenced or not there.
        fn delete_unreferenced_file(hash: Digest) -> bool;
    }

    writes! {
        /// Puts the finishes of the files of one issue one after the other: takes the lock of the
        /// issue, which is held until the end of the change, so the count that follows is the
        /// count of whoever got the lock first and two finishes cannot both take the last place.
        fn lock_issue_files(issue: IssueId) -> ();
        /// How many attachments an issue has as this change sees them, the ones in the trash
        /// included.
        fn attachment_count_in_change(issue: IssueId) -> u64;
        /// Ends an upload inside the change that makes its attachment; `false` when it was not
        /// there any more.
        fn delete_upload(id: UploadId) -> bool;
        /// Stores the row of a file unless the file is there already, and says whether it is new.
        fn insert_stored_file(file: &StoredFile) -> bool;
        /// Adds an attachment.
        fn insert_attachment(attachment: &Attachment) -> ();
        /// Puts an attachment into the trash (a date) or takes it out (`None`), counts its
        /// version up, and says whether the attachment was where the change expects it.
        fn set_attachment_deleted(id: AttachmentId, deleted_at: Option<Timestamp>) -> bool;
    }
}
