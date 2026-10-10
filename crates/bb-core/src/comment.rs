//! Comments and mentions (ISS-05, M3-Q7, ADR-0022): what people write under an issue, the earlier
//! versions that are kept, and who is mentioned.
//!
//! A mention is `@` and a handle. The handle of a person, group or role is made from its name by
//! [`handle_of`], so a name with spaces is written as `@ana.silva`. A mention that names more
//! than one thing, or nothing the writer may name, stays plain text.

use jiff::Timestamp;

use crate::id::{CommentId, IssueId, UserId};
use crate::personal::Entity;

/// The most people one comment can put into somebody's *Mentions*: a group of ten thousand
/// people is not told one by one.
pub const MENTION_RECIPIENTS_MAX: usize = 500;

/// The longest handle, in characters: the same as the longest name after `@` that Markdown finds
/// (`bb_markdown::MAX_MENTION_CHARS`).
pub const HANDLE_MAX: usize = 64;

/// The part of a comment that is edited under a lock: the name of the part in the lock.
pub const TEXT_PART: &str = "text";

words! {
    /// What a mention names.
    pub enum MentionTarget {
        /// A person, by name.
        Person => "person",
        /// A group: its members are mentioned.
        Group => "group",
        /// A role: the people who have it where the issue is are mentioned.
        Role => "role",
    }
}

words_in_api!(
    MentionTarget,
    "What a mention names: a person, a group or a role."
);

/// What the handle of a name is: lower case, words joined by dots, only letters, digits and
/// `_`, `.`, `-`, never starting or ending with a dot or a dash. `None` for a name that has no
/// letter or digit.
#[must_use]
pub fn handle_of(name: &str) -> Option<String> {
    let mut handle = String::new();
    let mut gap = false;
    for character in name.chars() {
        if character.is_alphanumeric() || matches!(character, '_' | '-') {
            if gap && !handle.is_empty() {
                handle.push('.');
            }
            gap = false;
            handle.extend(character.to_lowercase());
        } else if character.is_whitespace() || character == '.' {
            gap = true;
        }
    }
    let trimmed = handle.trim_matches(['.', '-']);
    let shortened: String = trimmed.chars().take(HANDLE_MAX).collect();
    let shortened = shortened.trim_end_matches(['.', '-']).to_owned();
    (!shortened.is_empty()).then_some(shortened)
}

/// Whether the text after `@` (without the `@`) names the thing called `name`.
#[must_use]
pub fn names_the_handle(written: &str, name: &str) -> bool {
    handle_of(name).is_some_and(|handle| handle == written.to_lowercase())
}

/// A comment under an issue (ISS-05).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Comment {
    /// The ID of the comment.
    #[not_personal]
    pub id: CommentId,
    /// The issue it is under.
    #[not_personal]
    pub issue: IssueId,
    /// Who wrote it.
    #[personal(category = "activity", basis = "contract")]
    pub author: UserId,
    /// The text, in Markdown. Free text that could name a person.
    #[personal(category = "content", basis = "contract")]
    pub text: String,
    /// When it was written.
    #[not_personal]
    pub created_at: Timestamp,
    /// When its text was last changed, if it was.
    #[not_personal]
    pub edited_at: Option<Timestamp>,
    /// When it was deleted into the trash, while it is there.
    #[not_personal]
    pub deleted_at: Option<Timestamp>,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// An earlier text of a comment, kept when the comment was edited (ISS-05). Shown to the people
/// who may see the comment.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct CommentVersion {
    /// The comment.
    #[not_personal]
    pub comment: CommentId,
    /// The version of the comment this text belonged to.
    #[not_personal]
    pub version: i64,
    /// The text it had. Free text that could name a person.
    #[personal(category = "content", basis = "contract")]
    pub text: String,
    /// When this text was saved.
    #[not_personal]
    pub saved_at: Timestamp,
}

/// A comment that puts someone into a person's *Mentions* (M3-Q7): the person was named, or a
/// group they are in, or a role they have where the issue is. One row for each comment and person.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Mention {
    /// The comment.
    #[not_personal]
    pub comment: CommentId,
    /// Who is mentioned.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub recipient: UserId,
    /// What the comment named to reach the person.
    #[not_personal]
    pub via: MentionTarget,
    /// When the person looked at it under *Mentions*; unread while empty.
    #[not_personal]
    pub read_at: Option<Timestamp>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iss_05_a_handle_is_the_name_in_lower_case_with_dots() {
        assert_eq!(handle_of("Ana Silva").as_deref(), Some("ana.silva"));
        assert_eq!(
            handle_of("  Project   lead ").as_deref(),
            Some("project.lead")
        );
        assert_eq!(handle_of("Zoë O'Neil").as_deref(), Some("zoë.oneil"));
        assert_eq!(handle_of("---").as_deref(), None);
        assert_eq!(handle_of("   ").as_deref(), None);
        assert!(names_the_handle("Ana.Silva", "Ana Silva"));
        assert!(!names_the_handle("ana", "Ana Silva"));
    }

    #[test]
    fn iss_05_a_handle_is_short_and_never_ends_in_a_dot() {
        let long = "ab ".repeat(100);
        let handle = handle_of(&long).unwrap();
        assert!(handle.chars().count() <= HANDLE_MAX);
        assert!(!handle.ends_with('.') && !handle.ends_with('-'));
    }
}
