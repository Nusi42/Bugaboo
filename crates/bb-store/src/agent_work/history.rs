//! The history of an agent (AI-37, ADR-0029, amendment M9): the hand-overs it made and the undos of
//! its runs, each newest first and then by ID, the order the use case puts both in.

use std::collections::BTreeMap;

use bb_app::agents::{HandOverByAgent, HistoryAfter, UndoByAgent};
use bb_core::event::{Actor, RecordKind};
use bb_core::id::{AgentId, EntityId, HandOverId, IssueId, UserId};
use sea_query::{Expr, ExprTrait as _, IntoColumnRef, Order, Query, SelectStatement};

use crate::db::{Database, StoredId as _, to_micros};
use crate::error::StoreError;
use crate::events::{actor_columns, event_columns, read_event};
use crate::paging::after_last;
use crate::schema::{AttemptRow, Event, HandOverRow, IssueWorkRow, ManyChangeRunRow};

/// The page of the history after `after`, newest first and then by ID.
fn newest_after(
    select: &mut SelectStatement,
    time: impl IntoColumnRef + Copy,
    id: impl IntoColumnRef + Copy,
    after: HistoryAfter,
    limit: u64,
) {
    select
        .order_by(time, Order::Desc)
        .order_by(id, Order::Asc)
        .limit(limit);
    if let Some((at, last)) = after {
        select.cond_where(after_last(
            || Expr::col(time),
            || Expr::val(to_micros(at)),
            &Order::Desc,
            id,
            EntityId::to_bytes(last).to_vec(),
        ));
    }
}

impl Database {
    /// The hand-overs `agent` made while it worked for `person`, with the issues of each.
    ///
    /// # Errors
    /// Fails when the database fails or a stored row is unreadable.
    pub(crate) async fn hand_overs_by_agent(
        &self,
        agent: AgentId,
        person: UserId,
        after: HistoryAfter,
        limit: u64,
    ) -> Result<Vec<HandOverByAgent>, StoreError> {
        let mut select = Query::select();
        select
            .columns([HandOverRow::Id, HandOverRow::Agent, HandOverRow::MadeAt])
            .from(HandOverRow::Table)
            .and_where(Expr::col(HandOverRow::ByAgent).eq(agent.stored()))
            .and_where(Expr::col(HandOverRow::Person).eq(person.stored()));
        newest_after(
            &mut select,
            (HandOverRow::Table, HandOverRow::MadeAt),
            (HandOverRow::Table, HandOverRow::Id),
            after,
            limit,
        );
        let mut found = Vec::new();
        for row in self.fetch_all(&select).await? {
            found.push(HandOverByAgent {
                id: row.id(HandOverRow::Id)?,
                to: row.id(HandOverRow::Agent)?,
                at: row.time(HandOverRow::MadeAt)?,
                issues: Vec::new(),
            });
        }
        if found.is_empty() {
            return Ok(found);
        }
        // The issues of the whole page in one statement.
        let mut issues = Query::select();
        issues
            .columns([IssueWorkRow::HandOver, IssueWorkRow::Issue])
            .from(IssueWorkRow::Table)
            .and_where(
                Expr::col(IssueWorkRow::HandOver).is_in(
                    found
                        .iter()
                        .map(|each| each.id.stored())
                        .collect::<Vec<_>>(),
                ),
            )
            .order_by(IssueWorkRow::Id, Order::Asc);
        let mut of: BTreeMap<HandOverId, Vec<IssueId>> = BTreeMap::new();
        for row in self.fetch_all(&issues).await? {
            of.entry(row.id(IssueWorkRow::HandOver)?)
                .or_default()
                .push(row.id(IssueWorkRow::Issue)?);
        }
        for each in &mut found {
            each.issues = of.remove(&each.id).unwrap_or_default();
        }
        Ok(found)
    }

    /// The undos `agent` made of its runs while it worked for `person`: the events an undo writes
    /// on its run, recorded as the agent's, with the issue of the attempt of that run.
    ///
    /// # Errors
    /// Fails when the database fails or a stored event is unreadable.
    pub(crate) async fn undos_by_agent(
        &self,
        agent: AgentId,
        person: UserId,
        after: HistoryAfter,
        limit: u64,
    ) -> Result<Vec<UndoByAgent>, StoreError> {
        let (actor_kind, actor_id) = actor_columns(Actor::Agent { agent, person });
        let mut select = Query::select();
        select
            .columns(
                event_columns()
                    .into_iter()
                    .map(|column| (Event::Table, column)),
            )
            .column((IssueWorkRow::Table, IssueWorkRow::Issue))
            .from(Event::Table)
            .inner_join(
                ManyChangeRunRow::Table,
                Expr::col((ManyChangeRunRow::Table, ManyChangeRunRow::Id))
                    .equals((Event::Table, Event::EntityId)),
            )
            .left_join(
                AttemptRow::Table,
                Expr::col((AttemptRow::Table, AttemptRow::Run))
                    .equals((ManyChangeRunRow::Table, ManyChangeRunRow::Id)),
            )
            .left_join(
                IssueWorkRow::Table,
                Expr::col((IssueWorkRow::Table, IssueWorkRow::Id))
                    .equals((AttemptRow::Table, AttemptRow::Work)),
            )
            .and_where(
                Expr::col((Event::Table, Event::EntityKind)).eq(RecordKind::ChangeRun.code()),
            )
            .and_where(
                Expr::col((ManyChangeRunRow::Table, ManyChangeRunRow::Agent)).eq(agent.stored()),
            )
            .and_where(Expr::col((Event::Table, Event::ActorKind)).eq(actor_kind))
            .and_where(Expr::col((Event::Table, Event::ActorId)).eq(actor_id))
            .and_where(Expr::col((Event::Table, Event::Reason)).eq(bb_core::run::RUN_UNDONE));
        newest_after(
            &mut select,
            (Event::Table, Event::Time),
            (Event::Table, Event::Id),
            after,
            limit,
        );
        self.fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                Ok(UndoByAgent {
                    event: read_event(row)?,
                    issue: row.optional_id(IssueWorkRow::Issue)?,
                })
            })
            .collect()
    }
}
