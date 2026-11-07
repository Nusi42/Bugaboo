//! The table of agents (AI-30, ADR-0029).

use super::Iden;

/// The agents of people.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AgentRow {
    #[iden = "agent"]
    Table,
    Id,
    Person,
    Name,
    State,
    ChangesAMinute,
    IssuesAtOnce,
    AttemptsOnOneIssue,
    OfferedTo,
    MadeBy,
    MadeAt,
    Version,
}
