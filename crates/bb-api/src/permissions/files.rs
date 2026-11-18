//! What the routes of files need (ADR-0032): uploads, the files of an issue and their signed
//! downloads. The rows of the table of permissions for this area, kept beside it so the table does
//! not grow.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of files, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("startUpload", Declared::ByRecord, Answer::Ordinary),
    route("getUpload", Declared::ByRecord, Answer::NeverStored),
    route("sendUploadPart", Declared::ByRecord, Answer::Ordinary),
    route("cancelUpload", Declared::ByRecord, Answer::Ordinary),
    route("finishUpload", Declared::ByRecord, Answer::Ordinary),
    route(
        "listIssueFiles",
        Declared::Permission(Permission::SeeIssues),
        Answer::NeverStored,
    ),
    route(
        "getFileRules",
        Declared::Permission(Permission::SeeIssues),
        Answer::NeverStored,
    ),
    route("deleteAttachment", Declared::ByRecord, Answer::Ordinary),
    route("restoreAttachment", Declared::ByRecord, Answer::Ordinary),
    route(
        "downloadFile",
        Declared::Permission(Permission::SeeIssues),
        Answer::NeverStored,
    ),
];
