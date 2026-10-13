//! The event log and the outbox (ADR-0007).
//!
//! [`Database::record_change`] is the only way to write domain data: it runs the change, appends
//! the domain event and the outbox entry in the same transaction, and commits all three or none.
//! Consumers (history, notifications, webhooks, the security log, the search index, live
//! updates) read the outbox with [`Database::outbox_after`], remember how far they came with
//! [`Database::outbox_done`], and are idempotent.
//!
//! The outbox positions are handed out one by one from a counter row (`head`) that the change
//! transaction locks until it commits. Two changes therefore never commit out of order, so a
//! consumer that has seen position 7 can never miss a position 6 that commits later.

use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef, FieldChange};
use bb_core::id::EventId;
use bb_core::outbox::OutboxEntry;
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use crate::db::{Database, Row, StoredId, Transaction, from_micros, to_micros};
use crate::error::StoreError;
use crate::schema::{Event, Outbox, OutboxPosition};

/// The name of the counter row in the consumer positions that hands out outbox positions.
pub(crate) const OUTBOX_HEAD: &str = "head";

/// How many events one page of the search for a record's newest event of a kind reads.
const EVENT_PAGE: u64 = 50;

const ACTOR_PERSON: &str = "person";
const ACTOR_SYSTEM: &str = "system";

impl Database {
    /// Runs `change` and appends `event` and its outbox entry, all in one transaction. If
    /// `change` fails, nothing is written.
    ///
    /// # Errors
    /// Returns the error of `change`, or a database error.
    pub async fn record_change<T>(
        &self,
        event: &DomainEvent,
        change: impl AsyncFnOnce(&mut Transaction) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        let mut transaction = self.begin().await?;
        let result = change(&mut transaction).await?;
        append_events(&mut transaction, std::slice::from_ref(event)).await?;
        // The events the change added besides its own come last, so the lock on the outbox is
        // the last lock of the transaction and no writer waits on it while holding a row.
        let follow_ups = std::mem::take(&mut transaction.follow_ups);
        append_events(&mut transaction, &follow_ups).await?;
        transaction.commit().await?;
        Ok(result)
    }

    /// The events about one record, oldest first.
    ///
    /// # Errors
    /// Fails when the database fails or a stored event is unreadable.
    pub async fn events_about(
        &self,
        entity: &EntityRef,
        limit: u64,
    ) -> Result<Vec<DomainEvent>, StoreError> {
        let mut select = Query::select();
        select
            .columns(event_columns())
            .from(Event::Table)
            .and_where(Expr::col(Event::EntityKind).eq(entity.kind.as_str()))
            .and_where(Expr::col(Event::EntityId).eq(entity.id.stored()))
            .order_by(Event::Id, Order::Asc)
            .limit(limit);
        self.fetch_all(&select)
            .await?
            .iter()
            .map(read_event)
            .collect()
    }

    /// Whether an event with this reason was written at or after `since`. Event IDs begin with
    /// their time, so the search reads only the range of the IDs since then.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn event_with_reason_since(
        &self,
        reason: &str,
        since: Timestamp,
    ) -> Result<bool, StoreError> {
        let first = EventId::from_parts(since, [0; 10])
            .map_err(|_| StoreError::Corrupt("a moment of an event"))?;
        let mut select = Query::select();
        select
            .column(Event::Id)
            .from(Event::Table)
            .and_where(Expr::col(Event::Id).gte(first.stored()))
            .and_where(Expr::col(Event::Reason).eq(reason))
            .limit(1);
        Ok(self.fetch_optional(&select).await?.is_some())
    }

    /// The newest event about one record for which `wanted` is true, looked for through all its
    /// events from the newest on, a page at a time, so no window cuts the search short.
    ///
    /// # Errors
    /// Fails when the database fails or a stored event is unreadable.
    pub async fn latest_event_about_where(
        &self,
        entity: &EntityRef,
        wanted: impl Fn(&DomainEvent) -> bool + Send,
    ) -> Result<Option<DomainEvent>, StoreError> {
        // The order they were committed in (the outbox position), not the time in their IDs: two
        // events of the same millisecond have no order there.
        let mut before = i64::MAX;
        loop {
            let mut select = Query::select();
            select
                .columns(event_columns().map(|column| (Event::Table, column)))
                .column((Outbox::Table, Outbox::Position))
                .from(Outbox::Table)
                .inner_join(
                    Event::Table,
                    Expr::col((Outbox::Table, Outbox::EventId)).equals((Event::Table, Event::Id)),
                )
                .and_where(Expr::col((Event::Table, Event::EntityKind)).eq(entity.kind.as_str()))
                .and_where(Expr::col((Event::Table, Event::EntityId)).eq(entity.id.stored()))
                .and_where(Expr::col((Outbox::Table, Outbox::Position)).lt(before))
                .order_by((Outbox::Table, Outbox::Position), Order::Desc)
                .limit(EVENT_PAGE);
            let rows = self.fetch_all(&select).await?;
            for row in &rows {
                let event = read_event(row)?;
                if wanted(&event) {
                    return Ok(Some(event));
                }
            }
            match rows.last() {
                Some(last) if rows.len() as u64 == EVENT_PAGE => {
                    before = last.get(Outbox::Position)?;
                }
                _ => return Ok(None),
            }
        }
    }

    /// The next events a consumer has not finished, with their outbox entries, in the order they
    /// were committed.
    ///
    /// # Errors
    /// Fails when the database fails or a stored event is unreadable.
    pub async fn outbox_after(
        &self,
        consumer: &TechnicalName,
        limit: u64,
    ) -> Result<Vec<(OutboxEntry, DomainEvent)>, StoreError> {
        let done = self.outbox_position(consumer).await?;
        let mut select = Query::select();
        select
            .columns(event_columns().map(|column| (Event::Table, column)))
            .column((Outbox::Table, Outbox::Position))
            .column((Outbox::Table, Outbox::CreatedAt))
            .from(Outbox::Table)
            .inner_join(
                Event::Table,
                Expr::col((Outbox::Table, Outbox::EventId)).equals((Event::Table, Event::Id)),
            )
            .and_where(Expr::col((Outbox::Table, Outbox::Position)).gt(done))
            .order_by((Outbox::Table, Outbox::Position), Order::Asc)
            .limit(limit);
        self.fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                let event = read_event(row)?;
                let entry = OutboxEntry {
                    position: row.get(Outbox::Position)?,
                    event_id: event.id,
                    created_at: row.time(Outbox::CreatedAt)?,
                };
                Ok((entry, event))
            })
            .collect()
    }

    /// How far `consumer` has come; 0 for a consumer that has not read anything yet.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn outbox_position(&self, consumer: &TechnicalName) -> Result<i64, StoreError> {
        let mut select = Query::select();
        select
            .column(OutboxPosition::Position)
            .from(OutboxPosition::Table)
            .and_where(Expr::col(OutboxPosition::Consumer).eq(consumer.as_str()));
        match self.fetch_optional(&select).await? {
            Some(row) => row.get(OutboxPosition::Position),
            None => Ok(0),
        }
    }

    /// Records that `consumer` has finished everything up to and including `position`. A smaller
    /// position than the recorded one changes nothing.
    ///
    /// # Errors
    /// Fails with [`StoreError::ReservedName`] for the name of the outbox's own counter, and when
    /// the database fails.
    pub async fn outbox_done(
        &self,
        consumer: &TechnicalName,
        position: i64,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        if consumer.as_str() == OUTBOX_HEAD {
            return Err(StoreError::ReservedName);
        }
        let mut transaction = self.begin().await?;
        let mut update = Query::update();
        update
            .table(OutboxPosition::Table)
            .value(OutboxPosition::Position, position)
            .value(OutboxPosition::UpdatedAt, to_micros(now))
            .and_where(Expr::col(OutboxPosition::Consumer).eq(consumer.as_str()))
            .and_where(Expr::col(OutboxPosition::Position).lt(position));
        let changed = transaction.execute(&update).await?;
        if changed == 0 && transaction_has_no_row(&mut transaction, consumer).await? {
            let mut insert = Query::insert();
            insert
                .into_table(OutboxPosition::Table)
                .columns([
                    OutboxPosition::Consumer,
                    OutboxPosition::Position,
                    OutboxPosition::UpdatedAt,
                ])
                .values([
                    Expr::val(consumer.as_str()),
                    Expr::val(position),
                    Expr::val(to_micros(now)),
                ])
                .map_err(|_| StoreError::Corrupt("an outbox position"))?;
            transaction.execute(&insert).await?;
        }
        transaction.commit().await
    }
}

/// The store is the adapter of the port the use cases write through (ARCH-EVENTS).
impl bb_app::ports::ChangeStore for Database {
    type Transaction = Transaction;
    type Error = StoreError;

    fn record_change<'a, T: Send + 'a>(
        &'a self,
        event: &'a DomainEvent,
        change: impl for<'t> FnOnce(
            &'t mut Transaction,
        ) -> bb_app::ports::BoxFuture<'t, Result<T, StoreError>>
        + Send
        + 'a,
    ) -> impl Future<Output = Result<T, StoreError>> + Send + 'a {
        Self::record_change(self, event, async move |transaction| {
            change(transaction).await
        })
    }

    async fn read_snapshot<'a, T: Send + 'a>(
        &'a self,
        read: impl for<'t> FnOnce(
            &'t mut Transaction,
        ) -> bb_app::ports::BoxFuture<'t, Result<T, StoreError>>
        + Send
        + 'a,
    ) -> Result<T, StoreError> {
        let mut transaction = self.begin_read().await?;
        read(&mut transaction).await
    }
}

async fn transaction_has_no_row(
    transaction: &mut Transaction,
    consumer: &TechnicalName,
) -> Result<bool, StoreError> {
    let mut select = Query::select();
    select
        .column(OutboxPosition::Position)
        .from(OutboxPosition::Table)
        .and_where(Expr::col(OutboxPosition::Consumer).eq(consumer.as_str()));
    Ok(transaction.fetch_optional(&select).await?.is_none())
}

/// How many events one statement inserts: the bind variables of a statement are limited (nine
/// for each event here), and SQLite's older limit of 999 is the smallest.
const EVENTS_PER_STATEMENT: usize = 100;

/// Appends the events and their outbox entries, in the order given, inside the transaction of the
/// change. The positions are taken with one statement for all of them and the rows are written
/// many to a statement, so a change that writes an event for each of thousands of issues is a
/// handful of statements and not thousands.
pub(crate) async fn append_events(
    transaction: &mut Transaction,
    events: &[DomainEvent],
) -> Result<(), StoreError> {
    if events.is_empty() {
        return Ok(());
    }
    let count = i64::try_from(events.len()).map_err(|_| StoreError::Corrupt("the events"))?;
    // First the counter: it takes the lock that puts the commits in order.
    let mut next = Query::update();
    next.table(OutboxPosition::Table)
        .value(
            OutboxPosition::Position,
            Expr::col(OutboxPosition::Position).add(count),
        )
        .and_where(Expr::col(OutboxPosition::Consumer).eq(OUTBOX_HEAD))
        .returning_col(OutboxPosition::Position);
    let last: i64 = transaction
        .fetch_optional(&next)
        .await?
        .ok_or(StoreError::Corrupt("the outbox counter"))?
        .get(OutboxPosition::Position)?;
    let first = last - (count - 1);

    let mut position = first;
    for chunk in events.chunks(EVENTS_PER_STATEMENT) {
        let mut insert = Query::insert();
        insert.into_table(Event::Table).columns(event_columns());
        let mut outbox = Query::insert();
        outbox.into_table(Outbox::Table).columns([
            Outbox::Position,
            Outbox::EventId,
            Outbox::CreatedAt,
        ]);
        for event in chunk {
            let (actor_kind, actor_id) = match event.actor {
                Actor::Person(person) => (ACTOR_PERSON, Some(person.stored())),
                Actor::System => (ACTOR_SYSTEM, None),
            };
            insert
                .values([
                    Expr::val(event.id.stored()),
                    Expr::val(to_micros(event.time)),
                    Expr::val(actor_kind),
                    Expr::val(actor_id),
                    Expr::val(event.channel.code()),
                    Expr::val(event.entity.kind.as_str()),
                    Expr::val(event.entity.id.stored()),
                    Expr::val(serde_json::to_string(&event.changes)?),
                    Expr::val(event.reason.clone()),
                ])
                .map_err(|_| StoreError::Corrupt("an event"))?;
            outbox
                .values([
                    Expr::val(position),
                    Expr::val(event.id.stored()),
                    Expr::val(to_micros(event.time)),
                ])
                .map_err(|_| StoreError::Corrupt("an outbox entry"))?;
            position += 1;
        }
        transaction.execute(&insert).await?;
        transaction.execute(&outbox).await?;
    }
    Ok(())
}

fn event_columns() -> [Event; 9] {
    [
        Event::Id,
        Event::Time,
        Event::ActorKind,
        Event::ActorId,
        Event::Channel,
        Event::EntityKind,
        Event::EntityId,
        Event::Changes,
        Event::Reason,
    ]
}

fn read_event(row: &Row) -> Result<DomainEvent, StoreError> {
    let actor = match row.get::<String>(Event::ActorKind)?.as_str() {
        ACTOR_PERSON => Actor::Person(
            row.optional_id(Event::ActorId)?
                .ok_or(StoreError::Corrupt("an actor"))?,
        ),
        ACTOR_SYSTEM => Actor::System,
        _ => return Err(StoreError::Corrupt("an actor")),
    };
    let changes: Vec<FieldChange> = serde_json::from_str(&row.get::<String>(Event::Changes)?)?;
    Ok(DomainEvent {
        id: row.id(Event::Id)?,
        time: from_micros(row.get(Event::Time)?)?,
        actor,
        channel: Channel::from_code(&row.get::<String>(Event::Channel)?)
            .ok_or(StoreError::Corrupt("a channel"))?,
        entity: EntityRef {
            kind: EntityKind::new(&row.get::<String>(Event::EntityKind)?)
                .map_err(|_| StoreError::Corrupt("a kind of record"))?,
            id: row.id(Event::EntityId)?,
        },
        changes,
        reason: row.get(Event::Reason)?,
    })
}

#[cfg(test)]
mod tests {
    use bb_core::id::IdSource as _;
    use bb_testing::{TestResult, each_database};

    use super::*;
    use crate::test_support::{at, event, ids, open};

    fn name(text: &str) -> TechnicalName {
        TechnicalName::new(text).unwrap()
    }

    #[tokio::test]
    async fn adr_0007_a_change_writes_its_event_and_outbox_entry_together() -> TestResult {
        each_database("adr_0007_change_event_outbox", async |db| {
            let database = open(db).await?;
            let mut ids = ids();
            let event = event(&mut ids, 5);
            let answer = database
                .record_change(&event, async |_transaction| Ok(42))
                .await?;
            assert_eq!(answer, 42);
            assert_eq!(
                database.events_about(&event.entity, 10).await?,
                std::slice::from_ref(&event)
            );
            let waiting = database.outbox_after(&name("history"), 10).await?;
            assert_eq!(waiting.len(), 1);
            assert_eq!(waiting[0].0.position, 1);
            assert_eq!(waiting[0].0.event_id, event.id);
            assert_eq!(waiting[0].1, event);
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn adr_0007_many_follow_up_events_get_one_position_each_in_order() -> TestResult {
        each_database("adr_0007_follow_ups_in_bulk", async |db| {
            let database = open(db).await?;
            let mut ids = ids();
            let event = event(&mut ids, 5);
            // More than one statement holds, so the chunks must join up.
            let follow_ups: Vec<DomainEvent> = (0..250)
                .map(|at| crate::test_support::event(&mut ids, 6 + at))
                .collect();
            let queued = follow_ups.clone();
            database
                .record_change(&event, async move |transaction| {
                    transaction.follow_ups.extend(queued);
                    Ok(())
                })
                .await?;
            let waiting = database.outbox_after(&name("history"), 1000).await?;
            let mut expected = vec![event];
            expected.extend(follow_ups);
            let seen: Vec<(i64, DomainEvent)> = waiting
                .into_iter()
                .map(|(entry, event)| (entry.position, event))
                .collect();
            assert_eq!(
                seen.iter()
                    .map(|(position, _)| *position)
                    .collect::<Vec<_>>(),
                (1..=251).collect::<Vec<_>>()
            );
            assert_eq!(
                seen.into_iter().map(|(_, event)| event).collect::<Vec<_>>(),
                expected
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn bad_example_a_failed_change_leaves_no_event() -> TestResult {
        each_database("adr_0007_failed_change", async |db| {
            let database = open(db).await?;
            let mut ids = ids();
            let event = event(&mut ids, 5);
            let failed: Result<(), _> = database
                .record_change(&event, async |_transaction| Err(StoreError::Contended))
                .await;
            assert!(matches!(failed, Err(StoreError::Contended)));
            assert!(database.events_about(&event.entity, 10).await?.is_empty());
            assert!(
                database
                    .outbox_after(&name("history"), 10)
                    .await?
                    .is_empty()
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn bad_example_the_same_event_twice_is_refused_and_takes_no_position() -> TestResult {
        each_database("adr_0007_duplicate_event", async |db| {
            let database = open(db).await?;
            let mut ids = ids();
            let first = event(&mut ids, 5);
            database.record_change(&first, async |_| Ok(())).await?;
            let again = database.record_change(&first, async |_| Ok(())).await;
            assert!(again.unwrap_err().is_duplicate());
            let second = event(&mut ids, 6);
            database.record_change(&second, async |_| Ok(())).await?;
            let positions: Vec<i64> = database
                .outbox_after(&name("history"), 10)
                .await?
                .iter()
                .map(|(entry, _)| entry.position)
                .collect();
            assert_eq!(positions, [1, 2], "no gap after the refused event");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn consumers_remember_their_position_and_never_go_back() -> TestResult {
        each_database("adr_0007_consumer_positions", async |db| {
            let database = open(db).await?;
            let mut ids = ids();
            for seconds in 1..=3 {
                let event = event(&mut ids, seconds);
                database.record_change(&event, async |_| Ok(())).await?;
            }
            let notifications = name("notifications");
            let history = name("history");
            assert_eq!(database.outbox_after(&notifications, 10).await?.len(), 3);
            database.outbox_done(&notifications, 2, at(10)).await?;
            let rest = database.outbox_after(&notifications, 10).await?;
            assert_eq!(rest.len(), 1);
            assert_eq!(rest[0].0.position, 3);
            database.outbox_done(&notifications, 1, at(11)).await?;
            assert_eq!(database.outbox_position(&notifications).await?, 2);
            assert_eq!(
                database.outbox_after(&history, 10).await?.len(),
                3,
                "each consumer has its own place"
            );
            let reserved = database.outbox_done(&name("head"), 9, at(12)).await;
            assert!(matches!(reserved, Err(StoreError::ReservedName)));
            Ok(())
        })
        .await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn many_changes_at_once_get_every_position_exactly_once() -> TestResult {
        each_database("adr_0007_concurrent_changes", async |db| {
            let database = open(db).await?;
            let mut source = ids();
            let mut tasks = Vec::new();
            for seconds in 0..12 {
                let database = database.clone();
                let event = event(&mut source, seconds);
                tasks.push(tokio::spawn(async move {
                    database.record_change(&event, async |_| Ok(())).await
                }));
            }
            for task in tasks {
                task.await??;
            }
            let positions: Vec<i64> = database
                .outbox_after(&name("history"), 100)
                .await?
                .iter()
                .map(|(entry, _)| entry.position)
                .collect();
            assert_eq!(positions, (1..=12).collect::<Vec<i64>>());
            let _ = source.now();
            Ok(())
        })
        .await
    }

    /// What a use case sees: only the port, not the database.
    async fn through_the_port(
        store: &impl bb_app::ports::ChangeStore<Error = StoreError>,
        event: &DomainEvent,
    ) -> Result<(), StoreError> {
        store
            .record_change(event, |_transaction| Box::pin(async { Ok(()) }))
            .await
    }

    #[tokio::test]
    async fn the_store_is_the_adapter_of_the_change_port() -> TestResult {
        each_database("adr_0007_change_port", async |db| {
            let database = open(db).await?;
            let event = event(&mut ids(), 5);
            through_the_port(&database, &event).await?;
            assert_eq!(database.events_about(&event.entity, 10).await?.len(), 1);
            Ok(())
        })
        .await
    }
}
