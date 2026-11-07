//! The tables of AI (ADR-0013): the marks *Never sent to AI* of built-in fields and projects, and
//! the log of tool calls.

use super::Iden;

/// The mark *Never sent to AI* of a built-in field or a project: one row has a field or a project,
/// never both. An own field carries its mark in `field.never_to_assistant`.
#[derive(Clone, Copy, Iden)]
pub(crate) enum NeverSentRow {
    #[iden = "never_sent_to_ai"]
    Table,
    Field,
    Project,
    Marked,
    Version,
}

/// One call of a tool. No column holds a question, an answer or a name.
#[derive(Clone, Copy, Iden)]
pub(crate) enum ToolCallRow {
    #[iden = "tool_call"]
    Table,
    Id,
    Person,
    Caller,
    App,
    Tool,
    Effect,
    BytesIn,
    BytesOut,
    At,
    Outcome,
}
