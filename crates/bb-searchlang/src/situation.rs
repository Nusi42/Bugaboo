//! What the grammar wants at a place of the text, and why a text is not a search. The parser makes
//! these; the mistakes in plain words (`mistake.rs`) and the suggestions (`suggest.rs`) are both
//! made from them, so what the person is told and what is offered cannot disagree.

use crate::span::Span;

/// What the grammar wants at a place of the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Situation {
    /// The start of the text: a filter, or `sort by`.
    Start,
    /// A filter: a field, `not`, a bracket or a helper.
    Filter,
    /// The operator of a field.
    Operator { field: String },
    /// A value for a field, after `is not`, a comparison or inside `in (…)`.
    Value { field: String },
    /// After `is`: `not`, `empty` or a value.
    IsRest { field: String },
    /// After `is not`: `empty` or a value.
    IsNotRest { field: String },
    /// After `in`: `last` or an opening bracket.
    InRest,
    /// After a value in `in (…)`: a comma or the closing bracket.
    ListNext,
    /// A whole number of at least one.
    Count,
    /// `days`, `weeks` or `months`.
    Unit,
    /// `step` or `steps`.
    StepsWord,
    /// The text of `contains`.
    Text,
    /// The name of the group of `member of`.
    Group,
    /// After `member`: `of`.
    MemberOf,
    /// The short name of a project.
    ProjectRef,
    /// An Issue ID.
    IssueRef,
    /// After `blocked`: `by`.
    BlockedBy,
    /// After `connected`: `to`.
    ConnectedTo,
    /// After a whole filter: `and`, `or`, `sort by`, a closing bracket, or the end.
    Continue { closing: bool },
    /// After `sort`: `by`.
    SortBy,
    /// A field to sort by.
    SortField,
    /// After the field of a sort key: a direction, a comma, or the end.
    SortAfterField,
    /// After a whole sort key: a comma or the end.
    SortAfterKey,
    /// A direction, of which the person has typed the first `typed` words.
    Direction { typed: Vec<String> },
}

/// Why a text is not a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reason {
    /// The grammar wants something else here.
    Expected(Situation),
    /// A word of the language where a value is needed: it has to be quoted.
    ReservedWord(String),
    /// A quoted text that does not end.
    UnclosedQuote,
    /// Something that looks like a day but is not one.
    NotADate(String),
    /// A piece the language does not know.
    Unreadable(String),
    /// Brackets or `not` nested deeper than the limit.
    TooDeep,
    /// A text longer than the limit; nothing is read of it.
    TooLong,
}

/// A failed parse: why, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParseError {
    pub(crate) reason: Reason,
    pub(crate) span: Span,
    /// Whether the text ended before the error: nothing was found where something is needed.
    pub(crate) at_end: bool,
}
