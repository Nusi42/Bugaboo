//! The card limits of the board of a saved search (PLAN-06, M10-Q2): rows of their own, read with
//! the searches they belong to, one statement for any number of searches.

use bb_core::id::SavedSearchId;
use bb_core::saved_search::{BoardOptions, SavedSearch, SearchCardLimit};
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use super::stored_ids;
use crate::db::{Reader, Row, StoredId as _, Transaction};
use crate::error::StoreError;
use crate::schema::SearchCardLimitRow;

/// The card limits of these saved searches, each search's in order.
fn limits_select(ids: &[SavedSearchId]) -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            SearchCardLimitRow::Search,
            SearchCardLimitRow::Position,
            SearchCardLimitRow::ColumnValue,
            SearchCardLimitRow::CardLimit,
        ])
        .from(SearchCardLimitRow::Table)
        .and_where(Expr::col(SearchCardLimitRow::Search).is_in(stored_ids(ids)))
        .order_by(SearchCardLimitRow::Search, Order::Asc)
        .order_by(SearchCardLimitRow::Position, Order::Asc);
    select
}

fn read_limit(row: &Row) -> Result<SearchCardLimit, StoreError> {
    Ok(SearchCardLimit {
        search: row.id::<SavedSearchId>(SearchCardLimitRow::Search)?,
        position: row.get(SearchCardLimitRow::Position)?,
        column: row.get(SearchCardLimitRow::ColumnValue)?,
        limit: row.get(SearchCardLimitRow::CardLimit)?,
    })
}

/// The saved searches `select` finds, each with its card limits: one statement more for all of
/// them, on the pool or in a change.
pub(super) async fn limited(
    reader: &mut impl Reader,
    select: &SelectStatement,
) -> Result<Vec<SavedSearch>, StoreError> {
    let mut searches: Vec<SavedSearch> = reader
        .fetch_all(select)
        .await?
        .iter()
        .map(super::read_search)
        .collect::<Result<_, _>>()?;
    if searches.is_empty() {
        return Ok(searches);
    }
    let ids: Vec<SavedSearchId> = searches.iter().map(|search| search.id).collect();
    let limits: Vec<SearchCardLimit> = reader
        .fetch_all(&limits_select(&ids))
        .await?
        .iter()
        .map(read_limit)
        .collect::<Result<_, _>>()?;
    for search in &mut searches {
        let own: Vec<SearchCardLimit> = limits
            .iter()
            .filter(|limit| limit.search == search.id)
            .cloned()
            .collect();
        search.board.limits = BoardOptions::limits_of(&own);
    }
    Ok(searches)
}

/// Reads the card limits of a search that is not there and drops them: a search that is missing runs the
/// statements of one that is found, so the number of statements tells nothing of whether it exists (ADR-0034,
/// decision 4).
pub(super) async fn read_for_missing(
    reader: &mut impl Reader,
    id: SavedSearchId,
) -> Result<(), StoreError> {
    reader.fetch_all(&limits_select(&[id])).await?;
    Ok(())
}

/// Writes the card limits of a saved search in place of the ones it had.
pub(super) async fn put_limits(
    transaction: &mut Transaction,
    search: &SavedSearch,
) -> Result<(), StoreError> {
    delete_limits(transaction, &[search.id]).await?;
    for limit in search.board.limit_rows(search.id) {
        let mut insert = Query::insert();
        insert
            .into_table(SearchCardLimitRow::Table)
            .columns([
                SearchCardLimitRow::Search,
                SearchCardLimitRow::Position,
                SearchCardLimitRow::ColumnValue,
                SearchCardLimitRow::CardLimit,
            ])
            .values([
                Expr::val(limit.search.stored()),
                Expr::val(limit.position),
                Expr::val(limit.column),
                Expr::val(limit.limit),
            ])
            .map_err(|_| StoreError::Corrupt("a card limit of a saved search"))?;
        transaction.execute(&insert).await?;
    }
    Ok(())
}

/// Deletes the card limits of these saved searches.
pub(super) async fn delete_limits(
    transaction: &mut Transaction,
    ids: &[SavedSearchId],
) -> Result<(), StoreError> {
    let mut delete = Query::delete();
    delete
        .from_table(SearchCardLimitRow::Table)
        .and_where(Expr::col(SearchCardLimitRow::Search).is_in(stored_ids(ids)));
    transaction.execute(&delete).await.map(|_| ())
}
