//! Who may add a file and who may take one away (ATT-01, ATT-03): the one place that says it, for the
//! upload, the rules a screen asks for, the button and the change.
//!
//! A file of the issue itself follows the right to change the issue (a Guest only on the issues they
//! reported); a file of a comment follows the right to comment, and is added by the person who wrote
//! the comment. A file is put into the trash, or taken out again, by whoever may change issues and
//! by the person who added it while they may still add it.

use bb_core::files::Attachment;
use bb_core::id::UserId;
use bb_core::issue::Issue;
use bb_policy::{Evaluation, Permission};

/// Whether `person` may add a file to the issue itself (`comment_author` is none), or to a comment
/// whose author is `comment_author`.
pub(super) fn may_add(
    held: &Evaluation,
    issue: &Issue,
    comment_author: Option<UserId>,
    person: UserId,
) -> bool {
    match comment_author {
        Some(author) => author == person && held.allows(Permission::Comment),
        None => held.may_change_issue(issue.reported_by),
    }
}

/// Whether `person` may put the attachment into the trash or take it out again: whoever may change
/// issues, and the person who added it while they may still add it. Whoever added a file to a
/// comment wrote the comment.
pub(super) fn may_remove(
    held: &Evaluation,
    issue: &Issue,
    attachment: &Attachment,
    person: UserId,
) -> bool {
    held.allows(Permission::ChangeIssues)
        || attachment.added_by == person
            && may_add(held, issue, attachment.comment.map(|_| person), person)
}
