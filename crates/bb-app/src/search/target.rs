//! What a word of a search stands for: the one list the checker and the compiler both start from.

use bb_core::id::FieldId;
use bb_core::own_fields::FieldType;
use bb_searchlang::Field as SearchField;

/// The part of an issue (or the own field) that a field of the search language reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldTarget {
    /// *Title*.
    Title,
    /// *Description*.
    Description,
    /// *Status*: the key of the status.
    Status,
    /// *Priority*.
    Priority,
    /// *Assigned to*.
    AssignedTo,
    /// *Reported by*.
    ReportedBy,
    /// *Labels*.
    Labels,
    /// *Tags*.
    Tags,
    /// The project.
    Project,
    /// The issue type, by its name.
    Type,
    /// The status group of the status.
    StatusGroup,
    /// The issue the issue sits under.
    Parent,
    /// When the issue was made.
    Created,
    /// When the issue last changed.
    Changed,
    /// When the issue was closed.
    Closed,
    /// An own field.
    Own {
        /// The field.
        field: FieldId,
        /// What kind of information it holds, which says which column holds its values.
        kind: FieldType,
    },
}

/// One field a person may search by: its word and kind for the checker, and what it stands for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Searchable {
    /// The field as the checker knows it.
    pub field: SearchField,
    /// What it reads.
    pub target: FieldTarget,
}

/// The target of the field with this word.
#[must_use]
pub fn target_of(fields: &[Searchable], word: &str) -> Option<FieldTarget> {
    fields
        .iter()
        .find(|searchable| searchable.field.word == word)
        .map(|searchable| searchable.target)
}
