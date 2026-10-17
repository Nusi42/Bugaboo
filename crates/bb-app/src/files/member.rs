//! The names of the members of an attachment in its events, defined once.

/// The issue an attachment belongs to.
pub(super) const ISSUE: &str = "issue";
/// The comment an attachment was added to.
pub(super) const COMMENT: &str = "comment";
/// The name of the file.
pub(super) const NAME: &str = "name";
/// The hash of the file, in hexadecimal.
pub(super) const FILE: &str = "file";
/// The real type of the file.
pub(super) const MEDIA_TYPE: &str = "media_type";
/// When the attachment went into the trash.
pub(super) const DELETED_AT: &str = crate::issues::member::DELETED_AT;
