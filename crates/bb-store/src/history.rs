//! The adapter of the history port (`bb_app::history::HistoryStore`): the rows of the history that
//! an outbox consumer builds, and the place the consumer has come to (ISS-06, ADR-0007).
//!
//! A row is stored once for each event and issue (`ON CONFLICT DO NOTHING`), and the place of the
//! consumer moves on after the rows are committed. A consumer that stops between the two reads the
//! same events again and finds its rows made, so nothing is doubled and nothing is lost.

use bb_app::history::HistoryStore;
use bb_core::event::{DomainEvent, FieldChange};
use bb_core::history::{CONSUMER, HistoryEntry, HistoryKind};
use bb_core::id::{EntityId, EventId, IssueId, UserId};
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, OnConflict, Order, Query};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, to_micros};
use crate::error::StoreError;
use crate::schema::HistoryEntryRow;

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
        kind: HistoryKind::from_code(&row.get::<String>(HistoryEntryRow::Kind)?)
            .ok_or(StoreError::Corrupt("a kind of history row"))?,
        record: row.id::<EntityId>(HistoryEntryRow::Record)?,
        record_kind: TechnicalName::new(&row.get::<String>(HistoryEntryRow::RecordKind)?)
            .map_err(|_| StoreError::Corrupt("a kind of record"))?,
        changes,
    })
}

impl HistoryStore for AuthStorage {
    async fn history_events(&self, limit: u64) -> Result<Vec<(i64, DomainEvent)>, StoreError> {
        self.database().outbox_events(CONSUMER, limit).await
    }

    async fn store_history(
        &self,
        entries: &[HistoryEntry],
        position: i64,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        let database = self.database();
        if !entries.is_empty() {
            let mut transaction = database.begin().await?;
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
                        HistoryEntryRow::Kind,
                        HistoryEntryRow::Record,
                        HistoryEntryRow::RecordKind,
                        HistoryEntryRow::Changes,
                    ])
                    .values([
                        Expr::val(entry.event.stored()),
                        Expr::val(entry.issue.stored()),
                        Expr::val(entry.position),
                        Expr::val(to_micros(entry.at)),
                        Expr::val(entry.by.map(StoredId::stored)),
                        Expr::val(entry.kind.code()),
                        Expr::val(entry.record.stored()),
                        Expr::val(entry.record_kind.as_str()),
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
            transaction.commit().await?;
        }
        database.outbox_set(CONSUMER, position, now).await
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
                HistoryEntryRow::Kind,
                HistoryEntryRow::Record,
                HistoryEntryRow::RecordKind,
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
