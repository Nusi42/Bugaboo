//! The adapter of the port of agents (`bb_app::agents::AgentStore`): the records of agents, and
//! the empty implementation of their work (AI-30, ADR-0029).

use bb_app::agents::{AgentStore, AgentWork, AgentWorkStore};
use bb_app::auth::AccessKeyRecord;
use bb_core::agent::{Agent, AgentState};
use bb_core::id::{AccessKeyId, AgentId, UserId};
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::locks::{LockClass, lock_key};
use crate::schema::{AccessKeyRow, AgentRow, Computed};

fn agent_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            AgentRow::Id,
            AgentRow::Person,
            AgentRow::Name,
            AgentRow::State,
            AgentRow::ChangesAMinute,
            AgentRow::IssuesAtOnce,
            AgentRow::AttemptsOnOneIssue,
            AgentRow::OfferedTo,
            AgentRow::MadeBy,
            AgentRow::MadeAt,
            AgentRow::Version,
        ])
        .from(AgentRow::Table);
    select
}

fn read_agent(row: &Row) -> Result<Agent, StoreError> {
    Ok(Agent {
        id: row.id::<AgentId>(AgentRow::Id)?,
        person: row.id::<UserId>(AgentRow::Person)?,
        name: row.get(AgentRow::Name)?,
        state: AgentState::from_code(&row.get::<String>(AgentRow::State)?)
            .ok_or(StoreError::Corrupt("an agent state"))?,
        changes_a_minute: row.get(AgentRow::ChangesAMinute)?,
        issues_at_once: row.get(AgentRow::IssuesAtOnce)?,
        attempts_on_one_issue: row.get(AgentRow::AttemptsOnOneIssue)?,
        offered_to: row.optional_id::<UserId>(AgentRow::OfferedTo)?,
        made_by: row.id::<UserId>(AgentRow::MadeBy)?,
        made_at: row.time(AgentRow::MadeAt)?,
        version: row.get(AgentRow::Version)?,
    })
}

/// A page of agents, by ID.
fn page(mut select: SelectStatement, after: Option<AgentId>, limit: u64) -> SelectStatement {
    if let Some(after) = after {
        select.and_where(Expr::col(AgentRow::Id).gt(after.stored()));
    }
    select.order_by(AgentRow::Id, Order::Asc).limit(limit);
    select
}

/// Writes one change to an agent that is still at `version`, counting the version up; says
/// whether the agent was at that version.
async fn update_at(
    transaction: &mut Transaction,
    id: AgentId,
    version: i64,
    set: impl FnOnce(&mut sea_query::UpdateStatement),
) -> Result<bool, StoreError> {
    let mut update = Query::update();
    update
        .table(AgentRow::Table)
        .value(AgentRow::Version, version + 1)
        .and_where(Expr::col(AgentRow::Id).eq(id.stored()))
        .and_where(Expr::col(AgentRow::Version).eq(version));
    set(&mut update);
    Ok(transaction.execute(&update).await? > 0)
}

impl AgentWorkStore for AuthStorage {
    /// Until hand-overs and standing rules exist, no agent has work: it may do nothing but read
    /// its own record.
    async fn agent_work(&self, _agent: AgentId) -> Result<AgentWork, StoreError> {
        Ok(AgentWork::default())
    }
}

impl AgentStore for AuthStorage {
    async fn agent_in(
        transaction: &mut Transaction,
        id: AgentId,
    ) -> Result<Option<Agent>, StoreError> {
        let mut select = agent_select();
        select.and_where(Expr::col(AgentRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_agent)
            .transpose()
    }

    async fn agent_count_of_in(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<i64, StoreError> {
        let mut select = Query::select();
        select
            .expr_as(Expr::col(AgentRow::Id).count(), Computed::Total)
            .from(AgentRow::Table)
            .and_where(Expr::col(AgentRow::Person).eq(person.stored()));
        match transaction.fetch_optional(&select).await? {
            Some(row) => row.get(Computed::Total),
            None => Ok(0),
        }
    }

    async fn agents_of(
        &self,
        person: UserId,
        after: Option<AgentId>,
        limit: u64,
    ) -> Result<Vec<Agent>, StoreError> {
        let mut select = agent_select();
        select.and_where(Expr::col(AgentRow::Person).eq(person.stored()));
        self.database()
            .fetch_all(&page(select, after, limit))
            .await?
            .iter()
            .map(read_agent)
            .collect()
    }

    async fn all_agents(
        &self,
        after: Option<AgentId>,
        limit: u64,
    ) -> Result<Vec<Agent>, StoreError> {
        self.database()
            .fetch_all(&page(agent_select(), after, limit))
            .await?
            .iter()
            .map(read_agent)
            .collect()
    }

    async fn keys_of_agent(&self, agent: AgentId) -> Result<Vec<AccessKeyRecord>, StoreError> {
        self.database().keys_of_agent(agent).await
    }

    async fn agents_offered_to(&self, person: UserId) -> Result<Vec<Agent>, StoreError> {
        let mut select = agent_select();
        select.and_where(Expr::col(AgentRow::OfferedTo).eq(person.stored()));
        self.database()
            .fetch_all(&page(select, None, 100))
            .await?
            .iter()
            .map(read_agent)
            .collect()
    }

    async fn lock_agents_of(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<(), StoreError> {
        transaction
            .advisory_lock(LockClass::Agents, lock_key(person.to_bytes()))
            .await
    }

    async fn insert_agent(transaction: &mut Transaction, agent: &Agent) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(AgentRow::Table)
            .columns([
                AgentRow::Id,
                AgentRow::Person,
                AgentRow::Name,
                AgentRow::State,
                AgentRow::ChangesAMinute,
                AgentRow::IssuesAtOnce,
                AgentRow::AttemptsOnOneIssue,
                AgentRow::OfferedTo,
                AgentRow::MadeBy,
                AgentRow::MadeAt,
                AgentRow::Version,
            ])
            .values([
                Expr::val(agent.id.stored()),
                Expr::val(agent.person.stored()),
                Expr::val(agent.name.as_str()),
                Expr::val(agent.state.code()),
                Expr::val(agent.changes_a_minute),
                Expr::val(agent.issues_at_once),
                Expr::val(agent.attempts_on_one_issue),
                Expr::val(agent.offered_to.map(StoredId::stored)),
                Expr::val(agent.made_by.stored()),
                Expr::val(to_micros(agent.made_at)),
                Expr::val(agent.version),
            ])
            .map_err(|_| StoreError::Corrupt("an agent"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn rename_agent(
        transaction: &mut Transaction,
        id: AgentId,
        name: &str,
        version: i64,
    ) -> Result<bool, StoreError> {
        update_at(transaction, id, version, |update| {
            update.value(AgentRow::Name, name);
        })
        .await
    }

    async fn set_agent_state(
        transaction: &mut Transaction,
        id: AgentId,
        state: AgentState,
        version: i64,
    ) -> Result<bool, StoreError> {
        update_at(transaction, id, version, |update| {
            update.value(AgentRow::State, state.code());
        })
        .await
    }

    async fn offer_agent(
        transaction: &mut Transaction,
        id: AgentId,
        to: Option<UserId>,
        version: i64,
    ) -> Result<bool, StoreError> {
        update_at(transaction, id, version, |update| {
            update.value(AgentRow::OfferedTo, to.map(StoredId::stored));
        })
        .await
    }

    async fn hand_agent_to(
        transaction: &mut Transaction,
        id: AgentId,
        person: UserId,
        version: i64,
    ) -> Result<bool, StoreError> {
        update_at(transaction, id, version, |update| {
            update
                .value(AgentRow::Person, person.stored())
                .value(AgentRow::OfferedTo, None::<Vec<u8>>);
        })
        .await
    }

    async fn stop_agents_of(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Vec<AgentId>, StoreError> {
        let mut select = Query::select();
        select
            .column(AgentRow::Id)
            .from(AgentRow::Table)
            .and_where(Expr::col(AgentRow::Person).eq(person.stored()))
            .and_where(Expr::col(AgentRow::State).ne(AgentState::Stopped.code()))
            .order_by(AgentRow::Id, Order::Asc);
        let mut stopped = Vec::new();
        for row in transaction.fetch_all(&select).await? {
            stopped.push(row.id::<AgentId>(AgentRow::Id)?);
        }
        if !stopped.is_empty() {
            let mut update = Query::update();
            update
                .table(AgentRow::Table)
                .value(AgentRow::State, AgentState::Stopped.code())
                .value(AgentRow::Version, Expr::col(AgentRow::Version).add(1))
                .and_where(
                    Expr::col(AgentRow::Id)
                        .is_in(stopped.iter().map(|id| id.stored()).collect::<Vec<_>>()),
                );
            transaction.execute(&update).await?;
        }
        Ok(stopped)
    }

    async fn delete_keys_of_agent(
        transaction: &mut Transaction,
        agent: AgentId,
    ) -> Result<Vec<AccessKeyId>, StoreError> {
        let mut held = Query::select();
        held.columns([AccessKeyRow::Id, AccessKeyRow::Owner])
            .from(AccessKeyRow::Table)
            .and_where(Expr::col(AccessKeyRow::Agent).eq(agent.stored()));
        let mut ids = Vec::new();
        for row in transaction.fetch_all(&held).await? {
            ids.push((
                row.id::<AccessKeyId>(AccessKeyRow::Id)?,
                row.id::<UserId>(AccessKeyRow::Owner)?,
            ));
        }
        for (id, owner) in &ids {
            crate::access_keys::delete_access_key(transaction, *owner, *id).await?;
        }
        Ok(ids.into_iter().map(|(id, _)| id).collect())
    }
}
