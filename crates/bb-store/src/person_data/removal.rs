//! The adapter of `bb_app::privacy::removal::RemovalStore` (PRIV-04, ADR-0044, decision 6): the
//! rows a removal deletes and the changes it scrubs, found by the conditions of the walk and by
//! nothing else.

use bb_app::privacy::removal::{RemovalStore, Scrub};
use bb_core::event::{DomainEvent, FieldChange};
use bb_core::id::UserId;
use bb_core::personal::{Entity as _, Link, LinkScope};
use sea_query::{Alias, Cond, Expr, ExprTrait as _, Iden, Query, SelectStatement};

use super::{UNSTORED_LINK, column, linked, records_of_kinds, table_of};
use crate::AuthStorage;
use crate::db::{StoredId as _, Transaction};
use crate::error::StoreError;
use crate::like;
use crate::schema::{Event, HistoryEntryRow, NotificationRow, tables};

impl RemovalStore for AuthStorage {
    async fn delete_linked(
        transaction: &mut Transaction,
        person: UserId,
        kind: &str,
        links: &[Link],
    ) -> Result<u64, StoreError> {
        let all = tables();
        let Some((table, any)) = linked(&all, person, kind, links)? else {
            return Ok(0);
        };
        let mut delete = Query::delete();
        delete
            .from_table(Alias::new(table.name.as_str()))
            .cond_where(any);
        transaction.execute(&delete).await
    }

    async fn scrub_changes(
        transaction: &mut Transaction,
        person: UserId,
        scrub: &Scrub,
    ) -> Result<u64, StoreError> {
        let about = events_about(person, scrub)?;
        let words: Vec<String> = scrub
            .words
            .iter()
            .filter(|word| !word.is_empty())
            .map(|word| as_in_json(word))
            .collect();
        let mut changed = 0;
        changed += scrub_table(
            transaction,
            Event::Table,
            Event::Changes,
            Event::Id,
            &about,
            &words,
        )
        .await?;
        changed += scrub_table(
            transaction,
            HistoryEntryRow::Table,
            HistoryEntryRow::Changes,
            HistoryEntryRow::Event,
            &about,
            &words,
        )
        .await?;
        changed += scrub_table(
            transaction,
            NotificationRow::Table,
            NotificationRow::Changes,
            NotificationRow::Event,
            &about,
            &words,
        )
        .await?;
        Ok(changed)
    }
}

/// The IDs of the events about a record that `scrub` names: of one of its kinds and the person's,
/// as the walk finds them, or one of its records.
fn events_about(person: UserId, scrub: &Scrub) -> Result<SelectStatement, StoreError> {
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
    let mut any = records_of_kinds(&all, &scrub.kinds, &kind_column, &id_column, person)?;
    for named in &scrub.records {
        any = any.add(
            Cond::all()
                .add(Expr::col(kind_column.clone()).eq(named.kind.as_str()))
                .add(Expr::col(id_column.clone()).eq(named.id.stored())),
        );
    }
    let mut select = Query::select();
    select.column(Event::Id).from(Event::Table).cond_where(any);
    Ok(select)
}

/// `word` as JSON writes it inside a string, so a name with a quote is found as it is stored.
fn as_in_json(word: &str) -> String {
    let quoted = serde_json::Value::String(word.to_owned()).to_string();
    quoted
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(&quoted)
        .to_owned()
}

/// How the changes of a row are scrubbed.
#[derive(Clone, Copy)]
enum Scrubbing<'w> {
    /// Every value goes: the change is about a record of the person.
    Every,
    /// The values that hold one of these texts go.
    Holding(&'w [String]),
}

/// The changes `stored` with the values `how` names taken out.
fn scrubbed(stored: &str, how: Scrubbing<'_>) -> Result<String, StoreError> {
    let corrupt = |_| StoreError::Corrupt("the changes of an event");
    let mut changes: Vec<FieldChange> = serde_json::from_str(stored).map_err(corrupt)?;
    let goes = |value: &Option<serde_json::Value>| match how {
        Scrubbing::Every => true,
        Scrubbing::Holding(words) => value.as_ref().is_some_and(|value| {
            let text = value.to_string();
            words.iter().any(|word| text.contains(word.as_str()))
        }),
    };
    for change in &mut changes {
        if goes(&change.before) {
            change.before = None;
        }
        if goes(&change.after) {
            change.after = None;
        }
    }
    serde_json::to_string(&changes).map_err(corrupt)
}

/// Scrubs the column `changes` of `table`: every value of the rows whose `event` is one of `about`,
/// and the values that hold one of `words` in every row. How many rows changed.
async fn scrub_table<T, C, E>(
    transaction: &mut Transaction,
    table: T,
    changes: C,
    event: E,
    about: &SelectStatement,
    words: &[String],
) -> Result<u64, StoreError>
where
    T: Iden + Copy + 'static,
    C: Iden + Copy + 'static,
    E: Iden + Copy + 'static,
{
    let mut rewritten = rewrite(
        transaction,
        table,
        changes,
        Cond::all().add(Expr::col(event).in_subquery(about.clone())),
        Scrubbing::Every,
    )
    .await?;
    if !words.is_empty() {
        let mut holding = Cond::any();
        for word in words {
            holding = holding.add(Expr::col(changes).like(like::containing(word)));
        }
        rewritten += rewrite(
            transaction,
            table,
            changes,
            holding,
            Scrubbing::Holding(words),
        )
        .await?;
    }
    Ok(rewritten)
}

/// Writes the scrubbed changes of each row of `table` that matches `rows`. Rows with the same
/// changes are scrubbed the same, so each distinct text is rewritten once, wherever it stands.
async fn rewrite<T, C>(
    transaction: &mut Transaction,
    table: T,
    changes: C,
    rows: Cond,
    how: Scrubbing<'_>,
) -> Result<u64, StoreError>
where
    T: Iden + Copy + 'static,
    C: Iden + Copy + 'static,
{
    let mut select = Query::select();
    select
        .distinct()
        .column(changes)
        .from(table)
        .cond_where(rows.clone());
    let stored: Vec<String> = transaction
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| row.get(changes))
        .collect::<Result<_, _>>()?;
    let mut rewritten = 0;
    for old in stored {
        let new = scrubbed(&old, how)?;
        if new == old {
            continue;
        }
        let mut update = Query::update();
        update
            .table(table)
            .value(changes, new)
            .cond_where(rows.clone().add(Expr::col(changes).eq(old)));
        rewritten += transaction.execute(&update).await?;
    }
    Ok(rewritten)
}

#[cfg(test)]
#[path = "removal_tests.rs"]
mod tests;
