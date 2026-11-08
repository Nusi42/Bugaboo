//! The history of an issue for a reader who may not see the moment time entries were saved
//! (PRIV-09, ADR-0035, decision 8).
//!
//! A row of time is dated, not timed, for such a reader, and it is also *placed* by its date: the
//! database puts the rows in the order of the moments they are shown at (the start of the date for a
//! time entry of somebody else, the moment itself for every other row), tied by a hash of the event
//! that follows no order of making, and cuts the pages by a keyset of that moment and hash. A time
//! row between two events of one day therefore tells nothing by its place, and the cursor holds the
//! moment the reader was shown and the hash, never a place in the outbox.

use bb_core::ErrorCode;
use bb_core::history::HistoryKind;
use bb_core::issue::Issue;
use bb_core::paging::SkipCursor;
use bb_core::relation::RelationKind;
use bb_policy::{Authorized, Evaluation};

use super::read::Reading;
use super::store::ShownKey;
use super::types::{HistoryItem, HistoryPage};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::walk::{Walk, fill_past_hidden};

/// A row with its place in the order, shaped for the reader unless it has nothing in it for them.
pub(super) struct Dated {
    key: ShownKey,
    item: Option<HistoryItem>,
}

/// The history walked in the order of shown moments.
struct DatedHistory<'a, 'c, 'r, S: AuthStore> {
    ctx: &'a Ctx<'c, S>,
    issue: &'a Issue,
    kind: Option<HistoryKind>,
    kinds: &'a [RelationKind],
    reading: &'a mut Reading<'r>,
}

impl<S: AuthStore> Walk for DatedHistory<'_, '_, '_, S> {
    type Row = Dated;
    type Key = ShownKey;

    async fn fetch(
        &mut self,
        after: Option<&ShownKey>,
        skip: u64,
        limit: u64,
    ) -> Result<Vec<Dated>, AppError> {
        let found = self
            .ctx
            .store()
            .history_page_shown(
                self.issue.id,
                self.kind,
                self.reading.actor.person.id,
                after.cloned(),
                skip,
                limit,
            )
            .await
            .map_err(|error| internal::<S>(&error))?;
        let entries: Vec<_> = found.iter().map(|(entry, _)| entry.clone()).collect();
        self.ctx.read_apps(self.reading, &entries).await?;
        let batch = self.ctx.batch_of(&entries, self.kinds).await?;
        let mut shaped = Vec::with_capacity(found.len());
        for (entry, key) in found {
            let item = self
                .ctx
                .history_item(self.reading, self.issue, &batch, &entry)
                .await?;
            shaped.push(Dated { key, item });
        }
        Ok(shaped)
    }

    fn costly_rows(&self) -> bool {
        true
    }

    async fn keeps(&mut self, row: &Dated) -> Result<bool, AppError> {
        Ok(row.item.is_some())
    }

    fn key_of(row: &Dated) -> Result<ShownKey, AppError> {
        Ok(row.key.clone())
    }
}

/// What a page of the history is asked: the cursor and how many rows.
pub(super) struct PageQuery<'c> {
    pub(super) cursor: Option<&'c str>,
    pub(super) len: usize,
}

/// One page of the history in the order of shown moments, for a reader who may not see the moment
/// of every time entry.
///
/// # Errors
/// `request.invalid` for a cursor that is not one.
pub(super) async fn dated_page<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    issue: &Issue,
    kind: Option<HistoryKind>,
    kinds: &[RelationKind],
    reading: &mut Reading<'_>,
    held: &Evaluation,
    asked: PageQuery<'_>,
) -> Result<Authorized<HistoryPage>, AppError> {
    let start = SkipCursor::<ShownKey>::read(asked.cursor)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    let mut walk = DatedHistory {
        ctx,
        issue,
        kind,
        kinds,
        reading,
    };
    let filled = fill_past_hidden(&mut walk, &start, asked.len).await?;
    Ok(held.authorize(HistoryPage {
        items: filled.rows.into_iter().filter_map(|row| row.item).collect(),
        next_cursor: filled.next.as_ref().map(SkipCursor::write),
    }))
}
