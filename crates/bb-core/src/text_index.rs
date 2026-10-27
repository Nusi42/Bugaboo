//! The index of the full-text search (ADR-0016, amendment M5): one row for each text of an issue
//! that a search may look into, with the prepared words of the text (`text_words`). Like the
//! history, it is a read model that an outbox consumer keeps; what a reader may see of it is
//! decided by the search, in the same statement as the permissions, never here.

use crate::id::{EntityId, IssueId};
use crate::personal::Entity;

/// The name of the outbox consumer that keeps the index (ADR-0007): its place in the outbox is
/// kept under this name.
pub const CONSUMER: &str = "text-index";

/// The number of the newest migration that changed the rule that prepares a text, so an index made
/// before it has to be made again (migration 25, M5-F53). A tool that keeps an index of its own
/// (the benchmark's dataset) compares its marker with this. It is also the number the list of
/// migrations gives migration 25 (one definition); when the rule changes again, give migration 25
/// its literal 25 back and set this to the new number.
pub const RULE_MIGRATION: i64 = 25;

/// The place the consumer has when the whole index has to be made again from the stored texts (the
/// rule that prepares a text changed, migration 25): the largest place there is, so a read of the
/// events after it finds none and the consumer waits. The first thing a start does about it is to
/// take the head of the outbox, make the rows in pieces, and put the place at that head.
pub const REBUILD_WANTED: i64 = i64::MAX;

words! {
    /// Where a text of an index row comes from. A search leaves out every source the reader may
    /// not see.
    pub enum TextSource {
        /// The title of the issue.
        Title => "title",
        /// The description of the issue.
        Description => "description",
        /// A comment of the issue.
        Comment => "comment",
        /// The name of a file attached to the issue or to one of its comments.
        File => "file",
        /// An own long-text field of the issue.
        Field => "field",
    }
}

/// One text of an issue, as the index keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct IndexedText {
    /// The number of the row.
    #[not_personal]
    pub id: i64,
    /// The issue the text belongs to.
    #[not_personal]
    pub issue: IssueId,
    /// Where the text comes from.
    #[not_personal]
    pub source: TextSource,
    /// The record the text comes from: the comment, the file or the own field; the issue itself for
    /// its title and description.
    #[not_personal]
    pub source_ref: EntityId,
    /// The prepared words of the text, one space between. It can hold anything a person typed, so it
    /// has the legal basis of the texts it is made from.
    #[personal(category = "content", basis = "contract")]
    pub words: String,
}
