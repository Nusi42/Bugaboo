//! The table of agents (AI-30, ADR-0029).

use super::{Iden, StoredTable};

/// The agents of people.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AgentRow {
    #[iden = "agent"]
    Table,
    Id,
    Person,
    Name,
    State,
    PausedFor,
    ChangesAMinute,
    IssuesAtOnce,
    AttemptsOnOneIssue,
    OfferedTo,
    MadeBy,
    MadeAt,
    Version,
}

stored_tables! {
    AgentRow => bb_core::agent::Agent {
        Id => id,
        Person => person,
        Name => name,
        State => state,
        PausedFor => paused_for,
        ChangesAMinute => changes_a_minute,
        IssuesAtOnce => issues_at_once,
        AttemptsOnOneIssue => attempts_on_one_issue,
        OfferedTo => offered_to,
        MadeBy => made_by,
        MadeAt => made_at,
        Version => version,
    },
}
