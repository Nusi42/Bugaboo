//! The table of the requests for agents of programs (ADR-0029, amendment M9).

use super::{Iden, StoredTable};

/// A request for agents. The person, the key and the grant have no reference: a removed person
/// leaves a tombstone (PRIV-04), a key or grant taken back leaves a request nobody can collect, and
/// the daily job deletes the row after its keep period.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AgentRequestRow {
    #[iden = "agent_request"]
    Table,
    Id,
    Person,
    AskedByKey,
    AskedByGrant,
    AskedByName,
    AskedFrom,
    Reason,
    Agents,
    State,
    AskedAt,
    AnsweredAt,
}

stored_tables! {
    AgentRequestRow => bb_core::agent_request::AgentRequest {
        Id => id,
        Person => person,
        AskedByKey => asked_by_key,
        AskedByGrant => asked_by_grant,
        AskedByName => asked_by_name,
        AskedFrom => asked_from,
        Reason => reason,
        Agents => agents,
        State => state,
        AskedAt => asked_at,
        AnsweredAt => answered_at,
    },
}
