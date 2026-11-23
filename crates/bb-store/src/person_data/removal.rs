//! The adapter of `bb_app::privacy::removal::RemovalStore` (PRIV-04, ADR-0044, decision 6): the
//! rows a removal deletes and the changes it scrubs, found by the conditions of the walk and by
//! nothing else.

use bb_app::privacy::removal::{PersonWords, RemovalStore, Scrub};
use bb_core::event::{DomainEvent, FieldChange};
use bb_core::id::{CommentId, IssueId, UserId};
use bb_core::personal::{Entity as _, EntityMarks, Link, LinkScope};
use bb_core::whose::MADE_ONLY_BY_THEIR_OWNER;
use sea_query::{Alias, Cond, Expr, ExprTrait as _, Iden, Order, Query, SelectStatement};

use super::{UNSTORED_LINK, column, linked, records_of_kinds, table_of};
use crate::AuthStorage;
use crate::db::{Dialect, StoredId as _, Transaction};
use crate::error::StoreError;
use crate::events::ACTOR_PERSON;
use crate::like;
use crate::schema::{
    Event, HistoryEntryRow, Job, NotificationRow, PersonHandleRow, PersonRow, tables,
};
use crate::text_index::{compact_text_index, forget_comments, reindex_issues};

impl RemovalStore for AuthStorage {
    async fn reindex_removed_texts(
        &self,
        issues: &[IssueId],
        comments: &[CommentId],
    ) -> Result<(), StoreError> {
        let mut transaction = self.database().begin().await?;
        forget_comments(&mut transaction, comments).await?;
        reindex_issues(&mut transaction, issues).await?;
        transaction.commit().await
    }

    async fn compact_removed_texts(&self) -> Result<(), StoreError> {
        compact_text_index(self.database()).await
    }

    async fn person_words(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Option<PersonWords>, StoreError> {
        let mut select = Query::select();
        select
            .columns([PersonRow::DisplayName, PersonRow::Email])
            .from(PersonRow::Table)
            .and_where(Expr::col(PersonRow::Id).eq(person.stored()));
        let Some(row) = transaction.fetch_optional(&select).await? else {
            return Ok(None);
        };
        let mut handles = Query::select();
        handles
            .column(PersonHandleRow::Handle)
            .from(PersonHandleRow::Table)
            .and_where(Expr::col(PersonHandleRow::Person).eq(person.stored()));
        let handles = transaction
            .fetch_all(&handles)
            .await?
            .iter()
            .map(|row| row.get(PersonHandleRow::Handle))
            .collect::<Result<_, _>>()?;
        Ok(Some(PersonWords {
            name: row.get(PersonRow::DisplayName)?,
            address: row.get(PersonRow::Email)?,
            handles,
        }))
    }

    async fn delete_linked(
        transaction: &mut Transaction,
        person: UserId,
        kind: &EntityMarks,
        links: &[Link],
    ) -> Result<u64, StoreError> {
        let all = tables();
        let Some(table) = all.iter().find(|table| table.marks.name() == kind.name()) else {
            return Ok(0);
        };
        let Some(any) = linked(table, &all, person, links, transaction.dialect())? else {
            return Ok(0);
        };
        let mut delete = Query::delete();
        delete
            .from_table(Alias::new(table.name.as_str()))
            .cond_where(any);
        transaction.execute(&delete).await
    }

    async fn delete_jobs_with_address(
        transaction: &mut Transaction,
        address: &str,
    ) -> Result<u64, StoreError> {
        if address.is_empty() {
            return Ok(0);
        }
        let mut delete = Query::delete();
        delete
            .from_table(Job::Table)
            .and_where(like::containing_any_case(
                Expr::col(Job::Payload),
                &as_in_json(address),
            ));
        transaction.execute(&delete).await
    }

    async fn scrub_changes(
        transaction: &mut Transaction,
        person: UserId,
        scrub: &Scrub,
    ) -> Result<u64, StoreError> {
        let about = events_about(person, scrub, transaction.dialect())?;
        let words = scrub.looked_for();
        let events = Scrubbed {
            table: Event::Table,
            changes: Event::Changes,
            event: Event::Id,
            key: &[Event::Id],
            reason: Some(Event::Reason),
        };
        let history = Scrubbed {
            table: HistoryEntryRow::Table,
            changes: HistoryEntryRow::Changes,
            event: HistoryEntryRow::Event,
            key: &[HistoryEntryRow::Event, HistoryEntryRow::Issue],
            reason: None,
        };
        let notifications = Scrubbed {
            table: NotificationRow::Table,
            changes: NotificationRow::Changes,
            event: NotificationRow::Event,
            key: &[NotificationRow::Id],
            reason: None,
        };
        let mut changed = scrub_table(transaction, &events, &about, &words, scrub).await?;
        changed += scrub_table(transaction, &history, &about, &words, scrub).await?;
        changed += scrub_table(transaction, &notifications, &about, &words, scrub).await?;
        Ok(changed)
    }
}

/// The IDs of the events about a record that `scrub` names: of one of its kinds and the person's,
/// as the walk finds them, or one of its records; and the events the person made themselves about
/// a record of one of its kinds that only their owner makes (`MADE_ONLY_BY_THEIR_OWNER`), which
/// can be gone by now (the name of an access key they deleted). What the person did to the records
/// of others (the roles they gave, the work times they edited) stays.
fn events_about(
    person: UserId,
    scrub: &Scrub,
    dialect: Dialect,
) -> Result<SelectStatement, StoreError> {
    let all = tables();
    let events = table_of(&all, DomainEvent::MARKS)?;
    let record = events
        .marks
        .person_links()
        .links()
        .iter()
        .find(|link| matches!(link.scope(), LinkScope::Record { kind: None }))
        .ok_or(UNSTORED_LINK)?;
    let kind_column = column(events, record.field(), false)?;
    let id_column = column(events, record.field(), true)?;
    let mut any = records_of_kinds(
        &all,
        &scrub.kinds,
        &kind_column,
        &id_column,
        (person, dialect),
    )?;
    for named in &scrub.records {
        any = any.add(
            Cond::all()
                .add(Expr::col(kind_column.clone()).eq(named.kind.as_str()))
                .add(Expr::col(id_column.clone()).eq(named.id.stored())),
        );
    }
    let own: Vec<&str> = scrub
        .kinds
        .iter()
        .filter(|kind| MADE_ONLY_BY_THEIR_OWNER.contains(kind))
        .map(|kind| kind.code())
        .collect();
    if !own.is_empty() {
        any = any.add(
            Cond::all()
                .add(Expr::col(Event::ActorKind).eq(ACTOR_PERSON))
                .add(Expr::col(Event::ActorId).eq(person.stored()))
                .add(Expr::col(kind_column).is_in(own)),
        );
    }
    let mut select = Query::select();
    select.column(Event::Id).from(Event::Table).cond_where(any);
    Ok(select)
}

/// `word` as JSON writes it inside a string, so a word with a quote is found as it is stored.
fn as_in_json(word: &str) -> String {
    let quoted = serde_json::Value::String(word.to_owned()).to_string();
    quoted
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(&quoted)
        .to_owned()
}

/// How many rows one page of a scrub reads: a scrub that cannot narrow its rows reads a whole
/// table, so it goes through it by key, a page at a time.
const SCRUB_PAGE: u64 = 500;

/// A table whose changes are scrubbed: its column of changes, the column of its event, the columns
/// of its key (IDs, 16 bytes each), and the column of the reason, if it has one.
struct Scrubbed<T: 'static> {
    table: T,
    changes: T,
    event: T,
    key: &'static [T],
    reason: Option<T>,
}

/// How the changes of a row are scrubbed.
#[derive(Clone, Copy)]
enum Scrubbing<'s> {
    /// Every value goes: the change is about a record of the person.
    Every,
    /// The values that hold one of the person's words go.
    Holding(&'s Scrub),
}

impl Scrubbing<'_> {
    fn takes_text(self, text: &str) -> bool {
        match self {
            Self::Every => true,
            Self::Holding(scrub) => scrub.holds(text),
        }
    }

    fn takes(self, value: Option<&serde_json::Value>) -> bool {
        match self {
            Self::Every => value.is_some(),
            Self::Holding(scrub) => value.is_some_and(|value| scrub.holds_value(value)),
        }
    }
}

/// The changes `stored` with the values `how` names taken out.
fn scrubbed(stored: &str, how: Scrubbing<'_>) -> Result<String, StoreError> {
    let corrupt = |_| StoreError::Corrupt("the changes of an event");
    let mut changes: Vec<FieldChange> = serde_json::from_str(stored).map_err(corrupt)?;
    for change in &mut changes {
        if how.takes(change.before.as_ref()) {
            change.before = None;
        }
        if how.takes(change.after.as_ref()) {
            change.after = None;
        }
    }
    serde_json::to_string(&changes).map_err(corrupt)
}

/// The rows of `columns` that may hold one of `words` (folded in Rust): the candidates of
/// [`like::may_hold_folded`], or every row (`None`) when a word keeps too little to narrow by.
fn candidates<T: Iden + Copy + 'static>(columns: &[T], words: &[String]) -> Option<Cond> {
    let mut holding = Cond::any();
    for word in words {
        for column in columns {
            holding = holding.add(like::may_hold_folded(Expr::col(*column), word)?);
        }
    }
    Some(holding)
}

/// Scrubs `table`: every value of the rows whose event is one of `about`, and the values that hold
/// one of the person's words in every row; the reason of an event the same way. The database picks
/// the rows that may hold a word ([`candidates`]), and `scrub` decides. How many rows changed.
async fn scrub_table<T: Iden + Copy + 'static>(
    transaction: &mut Transaction,
    table: &Scrubbed<T>,
    about: &SelectStatement,
    words: &[String],
    scrub: &Scrub,
) -> Result<u64, StoreError> {
    let mut rewritten = rewrite(
        transaction,
        table,
        Some(Cond::all().add(Expr::col(table.event).in_subquery(about.clone()))),
        Scrubbing::Every,
    )
    .await?;
    if !words.is_empty() {
        let columns: Vec<T> = std::iter::once(table.changes).chain(table.reason).collect();
        let rows = candidates(&columns, words);
        rewritten += rewrite(transaction, table, rows, Scrubbing::Holding(scrub)).await?;
    }
    Ok(rewritten)
}

/// The rows whose key comes after `last` in the order of `key`.
fn after_key<T: Iden + Copy + 'static>(key: &[T], last: &[Vec<u8>]) -> Cond {
    let mut after = Cond::any();
    for (at, column) in key.iter().enumerate() {
        let mut this = Cond::all();
        for (before, value) in key.iter().zip(last).take(at) {
            this = this.add(Expr::col(*before).eq(value.clone()));
        }
        if let Some(value) = last.get(at) {
            this = this.add(Expr::col(*column).gt(value.clone()));
        }
        after = after.add(this);
    }
    after
}

/// Writes the scrubbed changes (and reason) of each row of `table` that matches `rows` (every row
/// for none), going through them by key a page at a time: each row is read once with its key and
/// updated by it.
async fn rewrite<T: Iden + Copy + 'static>(
    transaction: &mut Transaction,
    table: &Scrubbed<T>,
    rows: Option<Cond>,
    how: Scrubbing<'_>,
) -> Result<u64, StoreError> {
    let mut rewritten = 0;
    let mut last: Option<Vec<Vec<u8>>> = None;
    loop {
        let mut select = Query::select();
        select
            .columns(table.key.iter().copied())
            .column(table.changes)
            .from(table.table)
            .limit(SCRUB_PAGE);
        if let Some(rows) = &rows {
            select.cond_where(rows.clone());
        }
        if let Some(last) = &last {
            select.cond_where(after_key(table.key, last));
        }
        for column in table.key {
            select.order_by(*column, Order::Asc);
        }
        if let Some(reason) = table.reason {
            select.column(reason);
        }
        let page = transaction.fetch_all(&select).await?;
        for row in &page {
            let key: Vec<Vec<u8>> = table
                .key
                .iter()
                .map(|column| row.get::<Vec<u8>>(*column))
                .collect::<Result<_, _>>()?;
            let old: String = row.get(table.changes)?;
            let new = scrubbed(&old, how)?;
            let reason: Option<String> = match table.reason {
                Some(column) => row.get(column)?,
                None => None,
            };
            let drop_reason = reason.as_deref().is_some_and(|text| how.takes_text(text));
            if new != old || drop_reason {
                let mut update = Query::update();
                update.table(table.table).value(table.changes, new);
                if let (true, Some(column)) = (drop_reason, table.reason) {
                    update.value(column, Option::<String>::None);
                }
                for (column, value) in table.key.iter().zip(&key) {
                    update.and_where(Expr::col(*column).eq(value.clone()));
                }
                rewritten += transaction.execute(&update).await?;
            }
            last = Some(key);
        }
        if (page.len() as u64) < SCRUB_PAGE {
            return Ok(rewritten);
        }
    }
}

#[cfg(test)]
#[path = "removal_tests.rs"]
mod tests;

/// The name of the one value the sweep reads.
#[cfg(any(test, feature = "testing"))]
#[derive(Clone, Copy, Iden)]
enum Swept {
    Value,
}

/// The value the sweep read, as text: bytes that are not UTF-8 (SQLite casts a stored ID as it is)
/// are read with the characters they cannot hold replaced.
///
/// # Errors
/// The error of reading it as text, when it can be read neither way.
#[cfg(any(test, feature = "testing"))]
fn swept_text(row: &crate::rows::Row) -> Result<Option<String>, StoreError> {
    row.get::<Option<String>>(Swept::Value).or_else(|error| {
        row.get::<Option<Vec<u8>>>(Swept::Value)
            .map(|bytes| bytes.map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
            .map_err(|_| error)
    })
}

#[cfg(any(test, feature = "testing"))]
impl AuthStorage {
    /// The columns (`table.column`) of every stored table that hold one of `texts` in some row,
    /// folded as names are (`bb_core::names::name_key`): the sweep that proves a removal leaves no
    /// name, handle or address behind. The database picks the candidates as a scrub does, and Rust
    /// decides; a candidate that cannot be read as text counts as found, so the sweep fails rather
    /// than overlooks.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn columns_holding(&self, texts: &[&str]) -> Result<Vec<String>, StoreError> {
        use bb_core::names::name_key;
        let folded: Vec<String> = texts.iter().map(|text| name_key(text)).collect();
        let mut found = Vec::new();
        for table in tables() {
            for (column, _) in &table.columns {
                let as_text = sea_query::Func::cast_as(
                    Expr::col(Alias::new(column.as_str())),
                    Alias::new("text"),
                );
                let mut select = Query::select();
                select
                    .expr_as(as_text.clone(), Swept::Value)
                    .from(Alias::new(table.name.as_str()));
                let mut any = Cond::any();
                let mut narrowed = true;
                for word in &folded {
                    match like::may_hold_folded(as_text.clone(), word) {
                        Some(may) => any = any.add(may),
                        None => narrowed = false,
                    }
                }
                if narrowed {
                    select.cond_where(any);
                }
                let holds = self.database().fetch_all(&select).await?.iter().any(|row| {
                    match swept_text(row) {
                        Ok(value) => value.is_some_and(|value| {
                            let value = name_key(&value);
                            folded.iter().any(|word| value.contains(word.as_str()))
                        }),
                        Err(_) => true,
                    }
                });
                if holds {
                    found.push(format!("{}.{column}", table.name));
                }
            }
        }
        found.sort();
        found.dedup();
        Ok(found)
    }
}
