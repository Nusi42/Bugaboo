//! The adapter of the history port (`bb_app::history::HistoryStore`): the rows of the history that
//! an outbox consumer builds, and the place the consumer has come to (ISS-06, ADR-0007).
//!
//! A row is stored once for each event and issue (`ON CONFLICT DO NOTHING`), and the place of the
//! consumer moves on in the same transaction as the rows (M5-F101: one wait for the write lock of
//! the database, not two, for a reader that catches the history up while people write). A consumer
//! that stops reads the same events again and finds its rows made, so nothing is doubled and nothing
//! is lost.

use bb_app::history::HistoryStore;
use bb_core::event::{DomainEvent, FieldChange};
use bb_core::history::{CONSUMER, HistoryEntry, HistoryKind};
use bb_core::id::{AgentId, EntityId, EventId, IssueId, RunId, UserId};
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, OnConflict, Order, Query};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, to_micros};
use crate::error::StoreError;
use crate::events::{consumer_name, outbox_move};
use crate::schema::{Event, HistoryEntryRow, ManyChangeRunRow};

fn read_entry(row: &Row) -> Result<HistoryEntry, StoreError> {
    let changes: Vec<FieldChange> =
        serde_json::from_str(&row.get::<String>(HistoryEntryRow::Changes)?)
            .map_err(|_| StoreError::Corrupt("the changes of a history row"))?;
    Ok(HistoryEntry {
        event: row.id::<EventId>(HistoryEntryRow::Event)?,
        issue: row.id::<IssueId>(HistoryEntryRow::Issue)?,
        position: row.get(HistoryEntryRow::Position)?,
        at: row.time(HistoryEntryRow::At)?,
        by: row.optional_id::<UserId>(HistoryEntryRow::By)?,
        agent: row.optional_id::<AgentId>(HistoryEntryRow::Agent)?,
        kind: HistoryKind::from_code(&row.get::<String>(HistoryEntryRow::Kind)?)
            .ok_or(StoreError::Corrupt("a kind of history row"))?,
        record: row.id::<EntityId>(HistoryEntryRow::Record)?,
        record_kind: TechnicalName::new(&row.get::<String>(HistoryEntryRow::RecordKind)?)
            .map_err(|_| StoreError::Corrupt("a kind of record"))?,
        run: row.optional_id::<RunId>(HistoryEntryRow::Run)?,
        undid_run: row.optional_id::<RunId>(HistoryEntryRow::UndidRun)?,
        changes,
    })
}

impl HistoryStore for AuthStorage {
    async fn history_events(&self, limit: u64) -> Result<Vec<(i64, DomainEvent)>, StoreError> {
        self.database()
            .outbox_events_with_run(CONSUMER, limit)
            .await
    }

    async fn store_history(
        &self,
        entries: &[HistoryEntry],
        position: i64,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        let mut transaction = self.database().begin().await?;
        for entry in entries {
            let mut insert = Query::insert();
            insert
                .into_table(HistoryEntryRow::Table)
                .columns([
                    HistoryEntryRow::Event,
                    HistoryEntryRow::Issue,
                    HistoryEntryRow::Position,
                    HistoryEntryRow::At,
                    HistoryEntryRow::By,
                    HistoryEntryRow::Agent,
                    HistoryEntryRow::Kind,
                    HistoryEntryRow::Record,
                    HistoryEntryRow::RecordKind,
                    HistoryEntryRow::Run,
                    HistoryEntryRow::UndidRun,
                    HistoryEntryRow::Changes,
                ])
                .values([
                    Expr::val(entry.event.stored()),
                    Expr::val(entry.issue.stored()),
                    Expr::val(entry.position),
                    Expr::val(to_micros(entry.at)),
                    Expr::val(entry.by.map(StoredId::stored)),
                    Expr::val(entry.agent.map(StoredId::stored)),
                    Expr::val(entry.kind.code()),
                    Expr::val(entry.record.stored()),
                    Expr::val(entry.record_kind.as_str()),
                    Expr::val(entry.run.map(StoredId::stored)),
                    Expr::val(entry.undid_run.map(StoredId::stored)),
                    Expr::val(serde_json::to_string(&entry.changes)?),
                ])
                .map_err(|_| StoreError::Corrupt("a history row"))?
                .on_conflict(
                    OnConflict::columns([HistoryEntryRow::Event, HistoryEntryRow::Issue])
                        .do_nothing()
                        .to_owned(),
                );
            transaction.execute(&insert).await?;
        }
        outbox_move(&mut transaction, &consumer_name(CONSUMER)?, position, now).await?;
        transaction.commit().await
    }

    async fn apps_of_events(
        &self,
        events: &[EventId],
    ) -> Result<Vec<(EventId, String)>, StoreError> {
        if events.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = Query::select();
        select
            .column((Event::Table, Event::Id))
            .column((ManyChangeRunRow::Table, ManyChangeRunRow::App))
            .from(Event::Table)
            .inner_join(
                ManyChangeRunRow::Table,
                Expr::col((Event::Table, Event::Run))
                    .equals((ManyChangeRunRow::Table, ManyChangeRunRow::Id)),
            )
            .and_where(
                Expr::col((Event::Table, Event::Id)).is_in(
                    events
                        .iter()
                        .map(|event| event.stored())
                        .collect::<Vec<_>>(),
                ),
            )
            .and_where(Expr::col((ManyChangeRunRow::Table, ManyChangeRunRow::App)).is_not_null());
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                Ok((
                    row.id::<EventId>(Event::Id)?,
                    row.get(ManyChangeRunRow::App)?,
                ))
            })
            .collect()
    }

    async fn history_page(
        &self,
        issue: IssueId,
        kind: Option<HistoryKind>,
        before: Option<i64>,
        skip: u64,
        limit: u64,
    ) -> Result<Vec<HistoryEntry>, StoreError> {
        let mut select = Query::select();
        select
            .columns([
                HistoryEntryRow::Event,
                HistoryEntryRow::Issue,
                HistoryEntryRow::Position,
                HistoryEntryRow::At,
                HistoryEntryRow::By,
                HistoryEntryRow::Agent,
                HistoryEntryRow::Kind,
                HistoryEntryRow::Record,
                HistoryEntryRow::RecordKind,
                HistoryEntryRow::Run,
                HistoryEntryRow::UndidRun,
                HistoryEntryRow::Changes,
            ])
            .from(HistoryEntryRow::Table)
            .and_where(Expr::col(HistoryEntryRow::Issue).eq(issue.stored()))
            .order_by(HistoryEntryRow::Position, Order::Desc)
            .order_by(HistoryEntryRow::Event, Order::Desc)
            .limit(limit)
            .offset(skip);
        if let Some(kind) = kind {
            select.and_where(Expr::col(HistoryEntryRow::Kind).eq(kind.code()));
        }
        if let Some(before) = before {
            select.and_where(Expr::col(HistoryEntryRow::Position).lt(before));
        }
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_entry)
            .collect()
    }
}

#[cfg(test)]
#[path = "tests/history_rows.rs"]
mod tests;
