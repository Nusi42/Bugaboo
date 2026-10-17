//! The port of the comment use cases: what they need from storage beyond the issues port.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::comment::{Comment, CommentVersion, Mention};
use bb_core::id::CommentId;
use jiff::Timestamp;

use crate::issues::IssueStore;
use crate::store_macros::{queries, reads, writes};

/// What the comment use cases need from storage.
pub trait CommentStore: IssueStore + 'static {
    queries! {
        /// The comment with this ID, also one in the trash.
        fn comment(id: CommentId) -> Option<Comment> = comment_in;
        /// The earlier texts of a comment, oldest first. The current text is in the comment.
        fn comment_versions(id: CommentId) -> Vec<CommentVersion> = comment_versions_in;
    }

    reads! {
        /// The comments with these IDs that exist, also those in the trash.
        fn comments_by_ids(ids: &[CommentId]) -> Vec<Comment>;
        /// The people a comment puts into *Mentions*.
        fn mentions_in_comment(comment: CommentId) -> Vec<Mention>;
    }

    writes! {
        /// Adds a comment.
        fn insert_comment(comment: &Comment) -> ();
        /// Writes a new text of a comment, counts its version up, and says whether the comment was
        /// still at `version`.
        fn update_comment_text(
            id: CommentId,
            text: &str,
            edited_at: Timestamp,
            version: i64
        ) -> bool;
        /// Keeps an earlier text of a comment.
        fn insert_comment_version(version: &CommentVersion) -> ();
        /// Puts a comment into the trash (a date) or takes it out (`None`), counts its version up,
        /// and says whether the comment was there.
        fn set_comment_deleted(id: CommentId, deleted_at: Option<Timestamp>) -> bool;
        /// Makes these the mentions of a comment: the ones that stay keep the date they were read.
        fn replace_mentions(comment: CommentId, mentions: &[Mention]) -> ();
    }
}
