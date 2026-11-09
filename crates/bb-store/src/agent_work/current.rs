//! The current work of the agents of a person, the list of *My agents*.

use super::{
    AgentId, AgentWorkState, Expr, IssueWork, IssueWorkRow, Order, SelectStatement, StoreError,
    Timestamp, UserId, all, only_open_work_at, read_work, work_select,
};
use crate::db::{StoredId as _, Transaction};
use sea_query::ExprTrait as _;

pub(super) async fn current_work_of_agents(
    transaction: &mut Transaction,
    agents: Vec<AgentId>,
    person: UserId,
    now: Timestamp,
    limit: u64,
) -> Result<Vec<IssueWork>, StoreError> {
    // Open work is what `only_open_work_at` says, of these agents; work accepted by itself is
    // closed and stays in the list of the person it was given for until they looked at it,
    // whoever the agent belongs to now. Three statements that each follow an index, newest
    // first: what waits for a person (so that a cut never takes it), the rest of the open work,
    // and what was accepted by itself.
    let of_agents = || {
        let mut select = work_select();
        select.and_where(
            Expr::col((IssueWorkRow::Table, IssueWorkRow::Agent)).is_in(
                agents
                    .iter()
                    .map(|agent| agent.stored())
                    .collect::<Vec<_>>(),
            ),
        );
        only_open_work_at(&mut select, now);
        select
    };
    let waiting_states = AgentWorkState::WAITING_FOR_A_PERSON.map(AgentWorkState::code);
    let state = || Expr::col((IssueWorkRow::Table, IssueWorkRow::State));
    let newest = |mut select: SelectStatement, limit: u64| {
        select
            .order_by((IssueWorkRow::Table, IssueWorkRow::Id), Order::Desc)
            .limit(limit);
        select
    };
    let mut found = all(
        transaction,
        &newest(
            {
                let mut waiting = of_agents();
                waiting.and_where(state().is_in(waiting_states));
                waiting
            },
            limit,
        ),
        read_work,
    )
    .await?;
    let room = |found: &Vec<IssueWork>| limit.saturating_sub(found.len() as u64);
    if room(&found) == 0 {
        return Ok(found);
    }
    let mut rest = of_agents();
    rest.and_where(state().is_not_in(waiting_states));
    let mut also = all(transaction, &newest(rest, room(&found)), read_work).await?;
    let mut accepted = work_select();
    accepted
        .and_where(Expr::col((IssueWorkRow::Table, IssueWorkRow::AcceptedByItself)).eq(true))
        .and_where(Expr::col((IssueWorkRow::Table, IssueWorkRow::Person)).eq(person.stored()));
    also.extend(all(transaction, &newest(accepted, room(&found)), read_work).await?);
    also.sort_by_key(|work| std::cmp::Reverse(work.id));
    found.extend(
        also.into_iter()
            .take(usize::try_from(room(&found)).unwrap_or(usize::MAX)),
    );
    Ok(found)
}
