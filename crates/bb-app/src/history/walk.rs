//! The history of one issue for one reader as a list that skips what they may not see (M3-F68):
//! the walk of [`fill_past_hidden`](crate::walk::fill_past_hidden), so a cursor never names a row
//! the reader may not see.

use bb_core::history::HistoryKind;
use bb_core::issue::Issue;
use bb_core::relation::RelationKind;

use super::read::Reading;
use super::types::HistoryItem;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::walk::Walk;

/// A row of the history: its place in the outbox and what is in it for the reader, if anything.
pub(super) struct Shaped {
    pub(super) position: i64,
    pub(super) item: Option<HistoryItem>,
}

/// The history of one issue for one reader, walked by [`fill_past_hidden`]: rows are shaped as
/// they are fetched and one with nothing for the reader is skipped (M3-F68).
pub(super) struct IssueHistory<'a, 'c, 'r, S: AuthStore> {
    pub(super) ctx: &'a Ctx<'c, S>,
    pub(super) issue: &'a Issue,
    pub(super) kind: Option<HistoryKind>,
    pub(super) kinds: &'a [RelationKind],
    pub(super) reading: &'a mut Reading<'r>,
}

impl<S: AuthStore> Walk for IssueHistory<'_, '_, '_, S> {
    type Row = Shaped;
    type Key = i64;

    async fn fetch(
        &mut self,
        after: Option<&i64>,
        skip: u64,
        limit: u64,
    ) -> Result<Vec<Shaped>, AppError> {
        let rows = self
            .ctx
            .store()
            .history_page(self.issue.id, self.kind, after.copied(), skip, limit)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let batch = self.ctx.batch_of(&rows, self.kinds).await?;
        let mut shaped = Vec::with_capacity(rows.len());
        for row in &rows {
            let item = self
                .ctx
                .history_item(self.reading, self.issue, &batch, row)
                .await?;
            shaped.push(Shaped {
                position: row.position,
                item,
            });
        }
        Ok(shaped)
    }

    async fn keeps(&mut self, row: &Shaped) -> Result<bool, AppError> {
        Ok(row.item.is_some())
    }

    fn key_of(row: &Shaped) -> Result<i64, AppError> {
        Ok(row.position)
    }
}
