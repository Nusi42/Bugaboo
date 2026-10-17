//! What the comment use cases take and give back (ISS-05). Inputs that change something are named
//! after what they do; inputs that only read end in `Query` (ARCH-EVENTS). Everything that is given
//! back is wrapped in `Authorized<T>` by the use case.
//!
//! The text of a comment is Markdown. Every answer that holds a text also holds `html`, made on
//! the server by `bb-markdown` from that text: issue IDs of issues the reader may see and names the
//! writer could name are chips, everything else is plain text (SEC-RICHTEXT, M3-F12).

use bb_core::comment::{Comment, CommentVersion, MentionTarget};
use bb_core::id::{CommentId, IssueId, LockToken};
use bb_policy::{Masked, PersonRef};
use jiff::Timestamp;
use serde::Serialize;

/// Write a comment under an issue.
pub struct AddComment {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// The text, in Markdown.
    pub text: String,
}

/// Save the new text of a comment under the lock the editor holds.
pub struct SaveComment {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The comment.
    pub comment: CommentId,
    /// The version of the comment the person started from.
    pub version: i64,
    /// The token the lock was taken with.
    pub lock_token: LockToken,
    /// The new text, in Markdown.
    pub text: String,
}

/// Put a comment into the trash.
pub struct DeleteComment {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The comment.
    pub comment: CommentId,
}

/// Take a comment out of the trash.
pub struct RestoreComment {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The comment.
    pub comment: CommentId,
}

/// The texts a comment had.
pub struct CommentVersionsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The comment.
    pub comment: CommentId,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many texts a page holds at most.
    pub limit: u32,
}

/// A comment as one viewer sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommentView {
    /// The ID of the comment.
    pub id: CommentId,
    /// The issue it is under.
    pub issue: IssueId,
    /// Who wrote it, as the kind *Authors* says for this viewer.
    pub author: PersonRef,
    /// The text, in Markdown.
    pub text: String,
    /// The text as safe HTML made on the server.
    pub html: String,
    /// When it was written.
    pub created_at: Timestamp,
    /// When its text was last changed, if it was.
    pub edited_at: Option<Timestamp>,
    /// Counts up with every change; a change is made on the version that was looked at.
    pub version: i64,
    /// Whether the viewer may change the text (they wrote it).
    pub may_edit: bool,
    /// Whether the viewer may put it into the trash.
    pub may_delete: bool,
}

impl Masked for CommentView {}

/// One text a comment had, with the moment it was saved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommentTextView {
    /// The version of the comment this text belonged to.
    pub version: i64,
    /// The text, in Markdown.
    pub text: String,
    /// The text as safe HTML.
    pub html: String,
    /// When it was saved.
    pub saved_at: Timestamp,
}

/// Every text of a comment, the current one last.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommentVersions {
    /// The comment.
    pub comment: CommentId,
    /// The texts, oldest first.
    pub items: Vec<CommentTextView>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for CommentVersions {}

impl CommentTextView {
    /// The earlier text `version` as the HTML `html` shows it.
    pub(crate) fn of(version: &CommentVersion, html: String) -> Self {
        Self {
            version: version.version,
            text: version.text.clone(),
            html,
            saved_at: version.saved_at,
        }
    }

    /// The current text of a comment as the HTML `html` shows it.
    pub(crate) fn current(comment: &Comment, html: String) -> Self {
        Self {
            version: comment.version,
            text: comment.text.clone(),
            html,
            saved_at: comment.edited_at.unwrap_or(comment.created_at),
        }
    }
}

/// What the writer may name after `@` under an issue.
pub struct MentionChoicesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue the comment goes under, by its readable ID or its internal ID.
    pub issue: String,
    /// The start of a name; everything the writer may name when empty.
    pub search: Option<String>,
    /// How many choices at most.
    pub limit: u32,
}

/// One thing the writer may name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MentionChoice {
    /// What it is.
    pub target: MentionTarget,
    /// What is written after `@`.
    pub handle: String,
    /// The name as people read it.
    pub name: String,
}

/// The choices of a writer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MentionChoices {
    /// The choices, people first, then groups, then roles.
    pub items: Vec<MentionChoice>,
    /// The cursor of the next page; always none, a writer reads the first few choices.
    pub next_cursor: Option<String>,
}

impl Masked for MentionChoices {}
