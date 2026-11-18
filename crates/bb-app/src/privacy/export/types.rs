//! What the use cases of *Download my data* take and answer.

use bb_core::data_export::DataExport;
use bb_core::id::{DataExportId, UserId};
use bb_policy::{Masked, PersonRef};

/// Ask for the file of one's own data.
pub struct AskMyData {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// Ask, as an administrator with *Download someone's data*, for the file of somebody else's data.
pub struct AskSomeonesData {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Whose data.
    pub person: UserId,
    /// Why, in the administrator's own words; the person is told.
    pub reason: String,
}

/// Ask where the newest file of one's own data stands.
pub struct MyDataExportQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// Ask for a page of every export, newest first (*Download someone's data*).
pub struct DataExportsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// How many to skip.
    pub skip: u64,
    /// How many at most.
    pub limit: u64,
}

/// Ask for the signed link of the download of an export.
pub struct ExportLinkQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The export; none for an address that names no export, which is refused like an export that
    /// is not there, but only once the session is known, so an access key hears that it may not
    /// come here at all.
    pub export: Option<DataExportId>,
}

/// Download the file of an export through its signed link.
pub struct DownloadExportQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The signed link.
    pub link: String,
}

/// An export, as the person who asked for it sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportView {
    /// The stored export; the reason only for an administrator's export.
    pub export: DataExport,
    /// Whose data it is, as the viewer sees them.
    pub person: PersonRef,
    /// Who asked for it, as the viewer sees them.
    pub asked_by: PersonRef,
}

impl Masked for ExportView {}

/// A page of exports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportPage {
    /// The exports, newest first.
    pub exports: Vec<ExportView>,
    /// Whether more follow.
    pub more: bool,
}

impl Masked for ExportPage {}

/// The signed link of a download, good for a few minutes: the API puts it into the path of the
/// download.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportLinkView {
    /// The signed token.
    pub token: String,
}

impl Masked for ExportLinkView {}
