//! How much work an agent has: the numbers its limits are judged against.

use super::*;

/// The number a counting statement answers (the column `Computed::Total`).
async fn count_of(
    transaction: &mut Transaction,
    select: &SelectStatement,
) -> Result<u64, StoreError> {
    let count: i64 = match transaction.fetch_optional(select).await? {
        Some(row) => row.get(Computed::Total)?,
        None => 0,
    };
    Ok(u64::try_from(count).unwrap_or(0))
}

/// How many issues the agent has an attempt running on now.
pub(super) async fn working_issues(
    transaction: &mut Transaction,
    agent: AgentId,
) -> Result<u64, StoreError> {
    let mut select = Query::select();
    select
        .expr_as(Expr::col(IssueWorkRow::Id).count(), Computed::Total)
        .from(IssueWorkRow::Table)
        .and_where(Expr::col(IssueWorkRow::Agent).eq(agent.stored()))
        .and_where(
            Expr::col(IssueWorkRow::State)
                .is_in(AgentWorkState::IN_ATTEMPT.map(AgentWorkState::code)),
        );
    only_open_work(&mut select);
    count_of(transaction, &select).await
}

/// How many pieces of work the agent has open at `now`.
pub(super) async fn open_work(
    transaction: &mut Transaction,
    agent: AgentId,
    now: Timestamp,
) -> Result<u64, StoreError> {
    let mut select = open_work_of_agent(agent, now);
    select.expr_as(
        Expr::col((IssueWorkRow::Table, IssueWorkRow::Id)).count(),
        Computed::Total,
    );
    count_of(transaction, &select).await
}
