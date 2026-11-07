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
use bb_core::id::{AgentId, EntityId, EventId, RunId};
use bb_core::outbox::OutboxEntry;
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};
use std::ops::ControlFlow;

use crate::db::{Database, Row, StoredId, Transaction, from_micros, rows_per_insert, to_micros};
use crate::error::StoreError;
use crate::schema::{Event, Outbox, OutboxPosition};

/// The name of the counter row in the consumer positions that hands out outbox positions.
pub(crate) const OUTBOX_HEAD: &str = "head";

/// How many events one page of the search for a record's newest event of a kind reads.
const EVENT_PAGE: u64 = 50;

const ACTOR_PERSON: &str = "person";
/// The kind of an event by an agent is `agent:` and the ID of the agent; `actor_id` holds the person
/// responsible for it. Two IDs in one row without a column of their own, so that an event table
/// that older databases hold stays as it is (M7-F37).
const ACTOR_AGENT: &str = "agent:";
const ACTOR_SYSTEM: &str = "system";

/// A query for events in the order they were committed (the outbox position), not by their IDs:
/// an ID begins with its time and ends in random bits, so two events of one millisecond have no
/// order there. The caller adds what it looks for.
fn committed_events(order: Order) -> sea_query::SelectStatement {
    committed_events_of(order, event_columns().len())
}

/// The same with the first `columns` of [`event_columns`]: the readers that go through the events
/// for what they hold (the history, the notifications, the text index) do not read the run, which
/// also lets the rebuild of the text index in an old migration run before the column exists.
fn committed_events_of(order: Order, columns: usize) -> sea_query::SelectStatement {
    let mut select = Query::select();
    select
        .columns(
            event_columns()
                .into_iter()
                .take(columns)
                .map(|column| (Event::Table, column)),
        )
        .from(Outbox::Table)
        .inner_join(
            Event::Table,
            Expr::col((Outbox::Table, Outbox::EventId)).equals((Event::Table, Event::Id)),
        )
        .order_by((Outbox::Table, Outbox::Position), order);
    select
}

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
        self.record_changes(std::slice::from_ref(event), change)
            .await
    }

    /// The same for any number of events of its own, also none.
    ///
    /// # Errors
    /// Returns the error of `change`, or a database error.
    pub async fn record_changes<T>(
        &self,
        events: &[DomainEvent],
        change: impl AsyncFnOnce(&mut Transaction) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        let mut transaction = self.begin().await?;
        let result = change(&mut transaction).await?;
        append_events(&mut transaction, events).await?;
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
        let mut select = committed_events(Order::Asc);
        select
            .and_where(Expr::col((Event::Table, Event::EntityKind)).eq(entity.kind.as_str()))
            .and_where(Expr::col((Event::Table, Event::EntityId)).eq(entity.id.stored()))
            .limit(limit);
        self.fetch_all(&select)
            .await?
            .iter()
            .map(read_event)
            .collect()
    }

    /// The events made inside a run, in the order they were committed, after the event `after`
    /// (a place in that order; none for the start), at most `limit`.
    ///
    /// # Errors
    /// Fails when the database fails or a stored event is unreadable.
    pub async fn run_events(
        &self,
        run: RunId,
        after: Option<EventId>,
        limit: u64,
    ) -> Result<Vec<(i64, DomainEvent)>, StoreError> {
        let mut select = committed_events(Order::Asc);
        select
            .column((Outbox::Table, Outbox::Position))
            .and_where(Expr::col((Event::Table, Event::Run)).eq(run.stored()))
            .limit(limit);
        if let Some(after) = after {
            let place = self.outbox_place(after).await?;
            select.and_where(Expr::col((Outbox::Table, Outbox::Position)).gt(place));
        }
        self.fetch_all(&select)
            .await?
            .iter()
            .map(|row| Ok((row.get(Outbox::Position)?, read_event(row)?)))
            .collect()
    }

    /// The events about these records of one kind that were committed after the event `since` and
    /// were not made inside `run`: what changed the records behind the back of the run.
    ///
    /// # Errors
    /// Fails when the database fails or a stored event is unreadable.
    pub async fn events_outside_run_after(
        &self,
        run: RunId,
        since: EventId,
        kind: &EntityKind,
        ids: &[EntityId],
    ) -> Result<Vec<(i64, DomainEvent)>, StoreError> {
        let place = self.outbox_place(since).await?;
        let mut found: Vec<(i64, DomainEvent)> = Vec::new();
        for chunk in ids.chunks(500) {
            let mut select = committed_events(Order::Asc);
            select
                .column((Outbox::Table, Outbox::Position))
                .and_where(Expr::col((Event::Table, Event::EntityKind)).eq(kind.as_str()))
                .and_where(
                    Expr::col((Event::Table, Event::EntityId))
                        .is_in(chunk.iter().map(|id| id.stored())),
                )
                .and_where(Expr::col((Outbox::Table, Outbox::Position)).gt(place))
                .and_where(
                    Expr::col((Event::Table, Event::Run))
                        .is_null()
                        .or(Expr::col((Event::Table, Event::Run)).ne(run.stored())),
                );
            for row in self.fetch_all(&select).await? {
                found.push((row.get(Outbox::Position)?, read_event(&row)?));
            }
        }
        Ok(found)
    }

    /// Where an event stands in the order they were committed.
    async fn outbox_place(&self, event: EventId) -> Result<i64, StoreError> {
        let mut select = Query::select();
        select
            .column(Outbox::Position)
            .from(Outbox::Table)
            .and_where(Expr::col(Outbox::EventId).eq(event.stored()));
        self.fetch_optional(&select)
            .await?
            .ok_or(StoreError::Corrupt("the place of an event"))?
            .get(Outbox::Position)
    }

    /// The events about records of one kind, oldest first: what the check after a run reads to
    /// count the comments of an issue (TEST-CONCURRENT).
    ///
    /// # Errors
    /// Fails when the database fails or a stored event is unreadable.
    #[cfg(any(test, feature = "testing"))]
    pub async fn events_of_kind(
        &self,
        kind: &EntityKind,
        limit: u64,
    ) -> Result<Vec<DomainEvent>, StoreError> {
        let mut select = committed_events(Order::Asc);
        select
            .and_where(Expr::col((Event::Table, Event::EntityKind)).eq(kind.as_str()))
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
        let (found, _) = self
            .scan_events_about(entity, |event| {
                if wanted(event) {
                    ControlFlow::Break(Some(event.clone()))
                } else {
                    ControlFlow::Continue(())
                }
            })
            .await?;
        Ok(found)
    }

    /// Whether an event about one record after `version` changed `field`. It reads the events from
    /// the newest on and stops at the first one that is not after `version`: events are in version
    /// order (the order they were committed), so the older ones cannot answer.
    ///
    /// # Errors
    /// Fails when the database fails or a stored event is unreadable.
    pub async fn field_changed_after(
        &self,
        entity: &EntityRef,
        field: &str,
        version: i64,
    ) -> Result<bool, StoreError> {
        Ok(self.search_field_change(entity, field, version).await?.0)
    }

    /// `field_changed_after`, and how many events it read.
    pub(crate) async fn search_field_change(
        &self,
        entity: &EntityRef,
        field: &str,
        version: i64,
    ) -> Result<(bool, usize), StoreError> {
        let (found, read) = self
            .scan_events_about(entity, |event| {
                if event.changes_field_after(field, version) {
                    ControlFlow::Break(Some(()))
                } else if event.version_after().is_some_and(|after| after <= version) {
                    ControlFlow::Break(None)
                } else {
                    ControlFlow::Continue(())
                }
            })
            .await?;
        Ok((found.is_some(), read))
    }

    /// Shows the events about one record to `visit`, the newest first, a page at a time, until it
    /// breaks with the answer (`None` when the search ends without one) or the events end. Gives
    /// back the answer and how many events `visit` saw.
    async fn scan_events_about<T>(
        &self,
        entity: &EntityRef,
        mut visit: impl FnMut(&DomainEvent) -> ControlFlow<Option<T>>,
    ) -> Result<(Option<T>, usize), StoreError> {
        let mut before = i64::MAX;
        let mut seen = 0;
        loop {
            let mut select = committed_events(Order::Desc);
            select
                .column((Outbox::Table, Outbox::Position))
                .and_where(Expr::col((Event::Table, Event::EntityKind)).eq(entity.kind.as_str()))
                .and_where(Expr::col((Event::Table, Event::EntityId)).eq(entity.id.stored()))
                .and_where(Expr::col((Outbox::Table, Outbox::Position)).lt(before))
                .limit(EVENT_PAGE);
            let rows = self.fetch_all(&select).await?;
            for row in &rows {
                let event = read_event(row)?;
                seen += 1;
                if let ControlFlow::Break(answer) = visit(&event) {
                    return Ok((answer, seen));
                }
            }
            match rows.last() {
                Some(last) if rows.len() as u64 == EVENT_PAGE => {
                    before = last.get(Outbox::Position)?;
                }
                _ => return Ok((None, seen)),
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
        let mut select = committed_events_of(Order::Asc, WITHOUT_RUN);
        select
            .column((Outbox::Table, Outbox::Position))
            .column((Outbox::Table, Outbox::CreatedAt))
            .and_where(Expr::col((Outbox::Table, Outbox::Position)).gt(done))
            .limit(limit);
        self.fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                let event = read_event_without_run(row)?;
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
        let mut transaction = self.begin().await?;
        outbox_move(&mut transaction, consumer, position, now).await?;
        transaction.commit().await
    }

    /// The next events the reader named `consumer` has not finished, with their places in the
    /// outbox: what the history, the notifications and the text index each read.
    ///
    /// # Errors
    /// Fails when the database fails or a stored event is unreadable.
    pub async fn outbox_events(
        &self,
        consumer: &'static str,
        limit: u64,
    ) -> Result<Vec<(i64, DomainEvent)>, StoreError> {
        Ok(self
            .outbox_after(&consumer_name(consumer)?, limit)
            .await?
            .into_iter()
            .map(|(entry, event)| (entry.position, event))
            .collect())
    }

    /// Records that the reader named `consumer` has finished everything up to `position`
    /// ([`Database::outbox_done`] for a reader that is named by its constant).
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn outbox_set(
        &self,
        consumer: &'static str,
        position: i64,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        self.outbox_done(&consumer_name(consumer)?, position, now)
            .await
    }
}

/// The store is the adapter of the port the use cases write through (ARCH-EVENTS).
impl bb_app::ports::ChangeStore for Database {
    type Transaction = Transaction;
    type Error = StoreError;

    fn record_changes<'a, T: Send + 'a>(
        &'a self,
        events: &'a [DomainEvent],
        change: impl for<'t> FnOnce(
            &'t mut Transaction,
        ) -> bb_app::ports::BoxFuture<'t, Result<T, StoreError>>
        + Send
        + 'a,
    ) -> impl Future<Output = Result<T, StoreError>> + Send + 'a {
        Self::record_changes(self, events, async move |transaction| {
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

/// The name of an outbox consumer (ADR-0007) as the outbox keeps it: the one place a reader makes it.
pub(crate) fn consumer_name(name: &'static str) -> Result<TechnicalName, StoreError> {
    TechnicalName::new(name).map_err(|_| StoreError::Corrupt("the name of an outbox reader"))
}

/// Records, inside the transaction of the reader that gives its place, that `consumer` has
/// finished everything up to and including `position`: the place moves in the same transaction as
/// what the reader made, so a reader that stops reads again and never twice (ADR-0007).
///
/// # Errors
/// Fails with [`StoreError::ReservedName`] for the name of the outbox's own counter, and when the
/// database fails.
pub(crate) async fn outbox_move(
    transaction: &mut Transaction,
    consumer: &TechnicalName,
    position: i64,
    now: Timestamp,
) -> Result<(), StoreError> {
    if consumer.as_str() == OUTBOX_HEAD {
        return Err(StoreError::ReservedName);
    }
    let mut update = Query::update();
    update
        .table(OutboxPosition::Table)
        .value(OutboxPosition::Position, position)
        .value(OutboxPosition::UpdatedAt, to_micros(now))
        .and_where(Expr::col(OutboxPosition::Consumer).eq(consumer.as_str()))
        .and_where(Expr::col(OutboxPosition::Position).lt(position));
    let changed = transaction.execute(&update).await?;
    if changed == 0 && transaction_has_no_row(transaction, consumer).await? {
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
    Ok(())
}

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
    for chunk in events.chunks(rows_per_insert(event_columns().len())) {
        // The column of the run is written only for events of a run, so an event outside a run is
        // written the way it always was (the tests of the older migrations write events with this
        // code on a database that does not have the column yet).
        let with_run = chunk.iter().any(|event| event.run.is_some());
        let mut insert = Query::insert();
        insert
            .into_table(Event::Table)
            .columns(
                event_columns()
                    .into_iter()
                    .take(if with_run { 10 } else { 9 }),
            );
        let mut outbox = Query::insert();
        outbox.into_table(Outbox::Table).columns([
            Outbox::Position,
            Outbox::EventId,
            Outbox::CreatedAt,
        ]);
        for event in chunk {
            let (actor_kind, actor_id) = actor_columns(event.actor);
            let mut values = vec![
                Expr::val(event.id.stored()),
                Expr::val(to_micros(event.time)),
                Expr::val(actor_kind),
                Expr::val(actor_id),
                Expr::val(event.channel.code()),
                Expr::val(event.entity.kind.as_str()),
                Expr::val(event.entity.id.stored()),
                Expr::val(serde_json::to_string(&event.changes)?),
                Expr::val(event.reason.clone()),
            ];
            if with_run {
                values.push(Expr::val(event.run.map(StoredId::stored)));
            }
            insert
                .values(values)
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
    crate::text_marks::mark_texts(transaction, events).await
}

pub(crate) fn event_columns() -> [Event; 10] {
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
        Event::Run,
    ]
}

/// The two columns that hold the actor of an event: the kind and the ID. The one function that
/// writes them, with [`actor_of`], which reads them: a person is `person` with their ID, the
/// system `system` with no ID, and an agent `agent:` and the agent's ID, with the person
/// responsible for it in the ID column, so what is kept and erased about a person finds the changes
/// of their agents too (documented with the events in architecture §4, M7-F37).
fn actor_columns(actor: Actor) -> (String, Option<Vec<u8>>) {
    match actor {
        Actor::Person(person) => (ACTOR_PERSON.to_owned(), Some(person.stored())),
        Actor::Agent { agent, person } => (format!("{ACTOR_AGENT}{agent}"), Some(person.stored())),
        Actor::System => (ACTOR_SYSTEM.to_owned(), None),
    }
}

/// The actor of an event from the two columns that [`actor_columns`] writes.
fn actor_of(row: &Row) -> Result<Actor, StoreError> {
    let kind: String = row.get(Event::ActorKind)?;
    let person = || {
        row.optional_id(Event::ActorId)?
            .ok_or(StoreError::Corrupt("an actor"))
    };
    match kind.as_str() {
        ACTOR_PERSON => Ok(Actor::Person(person()?)),
        ACTOR_SYSTEM => Ok(Actor::System),
        agent if agent.starts_with(ACTOR_AGENT) => Ok(Actor::Agent {
            agent: agent[ACTOR_AGENT.len()..]
                .parse::<AgentId>()
                .map_err(|_| StoreError::Corrupt("an actor"))?,
            person: person()?,
        }),
        _ => Err(StoreError::Corrupt("an actor")),
    }
}

/// How many of [`event_columns`] come before the run: what an event had before runs existed.
const WITHOUT_RUN: usize = 9;

/// An event read without its run (`run` is empty), from the first [`WITHOUT_RUN`] columns.
pub(crate) fn read_event_without_run(row: &Row) -> Result<DomainEvent, StoreError> {
    let actor = actor_of(row)?;
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
        run: None,
    })
}

/// An event read with its run, from all of [`event_columns`].
pub(crate) fn read_event(row: &Row) -> Result<DomainEvent, StoreError> {
    Ok(DomainEvent {
        run: row.optional_id(Event::Run)?,
        ..read_event_without_run(row)?
    })
}

#[cfg(test)]
#[path = "tests/events.rs"]
mod tests;
