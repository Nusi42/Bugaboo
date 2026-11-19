//! The adapter of the port of requests for agents (`bb_app::agents::AgentRequestStore`, ADR-0029
//! amendment M9).

use bb_app::agents::AgentRequestStore;
use bb_core::agent_request::{AgentRequest, AskedAgent, Asker, RequestState};
use bb_core::id::{AccessKeyId, AgentRequestId, GrantId, UserId};
use jiff::Timestamp;
use sea_query::{Cond, Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::AuthStorage;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::keeping::delete_limited;
use crate::schema::AgentRequestRow as Columns;

fn columns() -> [Columns; 11] {
    [
        Columns::Id,
        Columns::Person,
        Columns::AskedByKey,
        Columns::AskedByGrant,
        Columns::AskedByName,
        Columns::AskedFrom,
        Columns::Reason,
        Columns::Agents,
        Columns::State,
        Columns::AskedAt,
        Columns::AnsweredAt,
    ]
}

fn agents_text(agents: &[AskedAgent]) -> Result<String, StoreError> {
    serde_json::to_string(agents).map_err(|_| StoreError::Corrupt("the agents of a request"))
}

fn read(row: &Row) -> Result<AgentRequest, StoreError> {
    Ok(AgentRequest {
        id: row.id::<AgentRequestId>(Columns::Id)?,
        person: row.id::<UserId>(Columns::Person)?,
        asked_by_key: row.optional_id::<AccessKeyId>(Columns::AskedByKey)?,
        asked_by_grant: row.optional_id::<GrantId>(Columns::AskedByGrant)?,
        asked_by_name: row.get(Columns::AskedByName)?,
        asked_from: row.get(Columns::AskedFrom)?,
        reason: row.get(Columns::Reason)?,
        agents: serde_json::from_str(&row.get::<String>(Columns::Agents)?)
            .map_err(|_| StoreError::Corrupt("the agents of a request"))?,
        state: RequestState::from_code(&row.get::<String>(Columns::State)?)
            .ok_or(StoreError::Corrupt("the state of a request for agents"))?,
        asked_at: row.time(Columns::AskedAt)?,
        answered_at: row.optional_time(Columns::AnsweredAt)?,
    })
}

fn select() -> SelectStatement {
    let mut select = Query::select();
    select.columns(columns()).from(Columns::Table);
    select
}

impl AgentRequestStore for AuthStorage {
    async fn agent_request_in(
        transaction: &mut Transaction,
        id: AgentRequestId,
    ) -> Result<Option<AgentRequest>, StoreError> {
        let mut select = select();
        select.and_where(Expr::col(Columns::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read)
            .transpose()
    }

    async fn waiting_requests_of_in(
        transaction: &mut Transaction,
        asker: Asker,
        since: Timestamp,
    ) -> Result<i64, StoreError> {
        let by = match asker {
            Asker::Key(key) => Expr::col(Columns::AskedByKey).eq(key.stored()),
            Asker::Grant(grant) => Expr::col(Columns::AskedByGrant).eq(grant.stored()),
        };
        let mut select = Query::select();
        select
            .column(Columns::Id)
            .from(Columns::Table)
            .and_where(by)
            .and_where(Expr::col(Columns::State).eq(RequestState::Waiting.code()))
            .and_where(Expr::col(Columns::AskedAt).gt(to_micros(since)));
        let rows = transaction.fetch_all(&select).await?;
        Ok(i64::try_from(rows.len()).unwrap_or(i64::MAX))
    }

    async fn agent_requests_of(
        &self,
        person: UserId,
        since: Timestamp,
        limit: u64,
    ) -> Result<Vec<AgentRequest>, StoreError> {
        let mut select = select();
        select
            .and_where(Expr::col(Columns::Person).eq(person.stored()))
            .and_where(Expr::col(Columns::AskedAt).gt(to_micros(since)))
            .order_by(Columns::AskedAt, Order::Desc)
            .order_by(Columns::Id, Order::Desc)
            .limit(limit);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read)
            .collect()
    }

    async fn insert_agent_request(
        transaction: &mut Transaction,
        request: &AgentRequest,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(Columns::Table)
            .columns(columns())
            .values([
                Expr::val(request.id.stored()),
                Expr::val(request.person.stored()),
                Expr::val(request.asked_by_key.map(StoredId::stored)),
                Expr::val(request.asked_by_grant.map(StoredId::stored)),
                Expr::val(request.asked_by_name.as_str()),
                Expr::val(request.asked_from.clone()),
                Expr::val(request.reason.as_str()),
                Expr::val(agents_text(&request.agents)?),
                Expr::val(request.state.code()),
                Expr::val(to_micros(request.asked_at)),
                Expr::val(request.answered_at.map(to_micros)),
            ])
            .map_err(|_| StoreError::Corrupt("a request for agents"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn answer_agent_request(
        transaction: &mut Transaction,
        id: AgentRequestId,
        from: RequestState,
        to: RequestState,
        at: Timestamp,
        agents: &[AskedAgent],
    ) -> Result<bool, StoreError> {
        let answered = if from == RequestState::Waiting {
            Expr::val(to_micros(at))
        } else {
            Expr::col(Columns::AnsweredAt)
        };
        let mut update = Query::update();
        update
            .table(Columns::Table)
            .value(Columns::State, to.code())
            .value(Columns::Agents, agents_text(agents)?)
            .value(Columns::AnsweredAt, answered)
            .and_where(Expr::col(Columns::Id).eq(id.stored()))
            .and_where(Expr::col(Columns::State).eq(from.code()));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn delete_old_agent_requests(
        transaction: &mut Transaction,
        before: Timestamp,
        limit: u64,
    ) -> Result<u64, StoreError> {
        let delete = delete_limited(
            Columns::Table,
            &[Columns::Id],
            Cond::all().add(Expr::col(Columns::AskedAt).lt(to_micros(before))),
            limit,
        );
        transaction.execute(&delete).await
    }
}

#[cfg(test)]
#[path = "tests/agent_requests.rs"]
mod tests;
