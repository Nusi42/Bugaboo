//! What the file use cases take and give back (ATT-01…ATT-04, ATT-07). Inputs that change a
//! domain record are named after what they do; inputs that only read, and the steps of an upload
//! that change no record yet (the upload is a file on its way, not an attachment), end in `Query`
//! (ARCH-EVENTS). Everything that is given back is wrapped in `Authorized<T>` by the use case.

use bb_core::id::{AttachmentId, CommentId, IssueId, UploadId};
use bb_policy::{Masked, PersonRef};
use jiff::Timestamp;
use serde::Serialize;

use super::file_store::FileBody;

/// Start an upload of a file to an issue, or to a comment under it.
pub struct StartUploadQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// The comment the file is for; none for a file of the issue itself.
    pub comment: Option<CommentId>,
    /// The name of the file as the person's device has it.
    pub name: String,
    /// The size in bytes.
    pub size: u64,
}

/// How far an upload is.
pub struct UploadStateQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The upload.
    pub upload: UploadId,
}

/// Send the next part of an upload.
pub struct SendPartQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The upload.
    pub upload: UploadId,
    /// Where the part starts: the number of bytes the server has.
    pub offset: u64,
    /// The bytes of the part.
    pub bytes: Vec<u8>,
}

/// Throw an unfinished upload away.
pub struct CancelUploadQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The upload.
    pub upload: UploadId,
}

/// Finish an upload: the file is checked and becomes an attachment.
pub struct FinishUpload {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The upload.
    pub upload: UploadId,
}

/// The files of an issue and of its comments.
pub struct IssueFilesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
}

/// What the person may add to an issue, and the limits that count there.
pub struct FileRulesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
}

/// Put an attachment into the trash.
pub struct DeleteAttachment {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The attachment.
    pub attachment: AttachmentId,
}

/// Take an attachment out of the trash (*Undo*).
pub struct RestoreAttachment {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The attachment.
    pub attachment: AttachmentId,
}

/// Download a file, or look at its preview, through a signed link.
pub struct DownloadQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The token of the link, as it is in the address.
    pub link: String,
}

/// An upload that is going on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UploadView {
    /// The ID of the upload.
    pub id: UploadId,
    /// The size the file will have.
    pub size: u64,
    /// Where the next part starts: the bytes the server has.
    pub offset: u64,
    /// The most bytes one part may have.
    pub part_bytes: u64,
}

impl Masked for UploadView {}

/// An attachment as one viewer sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AttachmentView {
    /// The ID of the attachment.
    pub id: AttachmentId,
    /// The issue it belongs to.
    pub issue: IssueId,
    /// The comment it was added to, if it was.
    pub comment: Option<CommentId>,
    /// The name of the file.
    pub name: String,
    /// The real type of the file.
    pub media_type: String,
    /// The size in bytes.
    pub size: u64,
    /// Who added it, as the kind *Authors* says for this viewer.
    pub added_by: PersonRef,
    /// When it was added.
    pub added_at: Timestamp,
    /// Counts up with every change.
    pub version: i64,
    /// The signed link that downloads the file (good for five minutes, for this viewer).
    pub download_link: String,
    /// The signed link of the preview, while the file has one.
    pub preview_link: Option<String>,
    /// Whether the viewer may put it into the trash.
    pub may_delete: bool,
}

impl Masked for AttachmentView {}

/// The files of an issue, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IssueFiles {
    /// The attachments the viewer may see.
    pub items: Vec<AttachmentView>,
    /// The cursor of the next page; always none, an issue holds a bounded number of files.
    pub next_cursor: Option<String>,
}

impl Masked for IssueFiles {}

/// What the viewer may add to an issue and the limits that count there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileRules {
    /// Whether the viewer may add files to the issue itself.
    pub may_add: bool,
    /// The most bytes one file may have in the project of the issue.
    pub limit_bytes: u64,
    /// The extensions the organization never accepts, in lower case.
    pub blocked_types: Vec<String>,
    /// The most bytes one part of an upload may have.
    pub part_bytes: u64,
}

impl Masked for FileRules {}

/// A file that is sent to a person: not serializable, it goes to the HTTP layer as a stream.
pub struct FileDownload {
    /// The name of the file, for the download.
    pub name: String,
    /// The type the answer is sent as: plain data for a file, whatever it is, and the PNG of a
    /// preview.
    pub media_type: String,
    /// The pieces and the size.
    pub body: FileBody,
}

impl std::fmt::Debug for FileDownload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FileDownload")
            .field("size", &self.body.size)
            .finish_non_exhaustive()
    }
}
