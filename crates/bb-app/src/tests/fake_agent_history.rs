//! The history of an agent in the in-memory store (AI-37): its hand-overs and its undos, newest
//! first and then by ID, as the statements of the store read them.

use std::cmp::Reverse;

use bb_core::event::{Actor, RecordKind};
use bb_core::id::{AgentId, EntityId, RunId, UserId};
use jiff::Timestamp;

use super::fake::Shared;
use crate::agents::{HandOverByAgent, HistoryAfter, UndoByAgent};

/// The entries after `after` in the order of the history, at most `limit`.
fn page<T>(
    mut found: Vec<T>,
    key: impl Fn(&T) -> (Timestamp, EntityId),
    after: HistoryAfter,
    limit: u64,
) -> Vec<T> {
    let place = |at: Timestamp, id: EntityId| (Reverse(at), id.to_bytes());
    found.sort_by_key(|each| {
        let (at, id) = key(each);
        place(at, id)
    });
    found
        .into_iter()
        .filter(|each| {
            let (at, id) = key(each);
            after.is_none_or(|(last_at, last_id)| place(at, id) > place(last_at, last_id))
        })
        .take(usize::try_from(limit).unwrap_or(usize::MAX))
        .collect()
}

pub(super) fn hand_overs_by_agent(
    shared: &mut Shared,
    agent: AgentId,
    person: UserId,
    after: HistoryAfter,
    limit: u64,
) -> Vec<HandOverByAgent> {
    shared.reads += 1;
    let work = &shared.state.work;
    let found = work
        .hand_overs
        .iter()
        .filter(|hand_over| hand_over.by_agent == Some(agent) && hand_over.person == person)
        .map(|hand_over| HandOverByAgent {
            id: hand_over.id,
            to: hand_over.agent,
            at: hand_over.made_at,
            issues: work
                .works
                .iter()
                .filter(|each| each.hand_over == hand_over.id)
                .map(|each| each.issue)
                .collect(),
        })
        .collect();
    page(found, |each| (each.at, each.id.into()), after, limit)
}

pub(super) fn undos_by_agent(
    shared: &mut Shared,
    agent: AgentId,
    person: UserId,
    after: HistoryAfter,
    limit: u64,
) -> Vec<UndoByAgent> {
    shared.reads += 1;
    let work = &shared.state.work;
    let issue_of = |run: RunId| {
        let attempt = work.attempts.iter().find(|each| each.run == Some(run))?;
        work.works
            .iter()
            .find(|each| each.id == attempt.work)
            .map(|each| each.issue)
    };
    let found = shared
        .events
        .iter()
        .filter(|event| {
            event.entity.kind.as_str() == RecordKind::ChangeRun.code()
                && event.actor == Actor::Agent { agent, person }
                && bb_core::run::is_undo_fact(event.reason.as_deref())
        })
        .map(|event| UndoByAgent {
            event: event.clone(),
            issue: RunId::from_bytes(event.entity.id.to_bytes())
                .ok()
                .and_then(issue_of),
        })
        .collect();
    page(
        found,
        |each| (each.event.time, each.event.id.into()),
        after,
        limit,
    )
}
