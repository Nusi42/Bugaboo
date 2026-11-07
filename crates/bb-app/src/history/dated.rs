//! The history of an issue for a reader who may not see the moment time entries were saved
//! (PRIV-09, ADR-0035, decision 8).
//!
//! A row of time is dated, not timed, for such a reader, and it is also *placed* by its date: the
//! rows are put in the order of their shown moments (the start of the date for a time entry), so a
//! time row between two events of one day tells nothing by its place. The order is not the order
//! the rows were written in, so the page is cut from the whole history of the issue, which is read
//! once and sorted here. The cursor holds the shown moment and the place of the row after which the
//! next page starts; it is sealed over HTTP, and the moment in it is the one the reader has seen.

use std::cmp::Reverse;
use std::fmt;
use std::str::FromStr;

use bb_core::ErrorCode;
use bb_core::history::{HistoryEntry, HistoryKind};
use bb_core::issue::Issue;
use bb_core::paging::SkipCursor;
use bb_core::relation::RelationKind;
use bb_policy::{Authorized, Evaluation, Permission};
use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::read::{Reading, is_time};
use super::time::hidden_day;
use super::types::{HistoryItem, HistoryPage};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::walk::{Walk, fill_past_hidden};

/// How many rows of the history one read of the store takes while the whole history is built.
const WHOLE_HISTORY_STEP: u64 = 500;

/// Where a row stands in the order of shown moments: the moment it is shown at (milliseconds), then
/// its place in the outbox, which only tells the rows of one moment apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Spot {
    millis: i64,
    position: i64,
}

impl fmt::Display for Spot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.millis, self.position)
    }
}

impl FromStr for Spot {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, ()> {
        let (millis, position) = text.split_once(':').ok_or(())?;
        Ok(Self {
            millis: millis.parse().map_err(|_| ())?,
            position: position.parse().map_err(|_| ())?,
        })
    }
}

/// The spot of a row for a reader: the start of its date for a time entry whose moment the reader
/// may not see, else the moment it happened.
fn spot_of(row: &HistoryEntry, sees_moment: bool) -> Spot {
    let shown = if is_time(row) {
        hidden_day(row, sees_moment)
            .and_then(|day| day.to_zoned(TimeZone::UTC).ok())
            .map(|start| start.timestamp())
    } else {
        None
    };
    Spot {
        millis: shown.unwrap_or(row.at).as_millisecond(),
        position: row.position,
    }
}

/// A row with its spot, shaped for the reader unless it has nothing in it for them.
pub(super) struct Dated {
    spot: Spot,
    item: Option<HistoryItem>,
}

/// The history walked in the order of shown moments.
struct DatedHistory<'a, 'c, 'r, S: AuthStore> {
    ctx: &'a Ctx<'c, S>,
    issue: &'a Issue,
    kind: Option<HistoryKind>,
    kinds: &'a [RelationKind],
    reading: &'a mut Reading<'r>,
    /// Whether the reader holds *See working hours of others*.
    sees_moments: bool,
    /// The whole history, latest shown moment first; read when the first page asks for it.
    ordered: Option<Vec<(Spot, HistoryEntry)>>,
}

impl<S: AuthStore> DatedHistory<'_, '_, '_, S> {
    /// The whole history of the issue in the order of shown moments, read once.
    async fn whole(&mut self) -> Result<&[(Spot, HistoryEntry)], AppError> {
        if self.ordered.is_none() {
            let mut rows = Vec::new();
            loop {
                let step = self
                    .ctx
                    .store()
                    .history_page(
                        self.issue.id,
                        self.kind,
                        None,
                        u64::try_from(rows.len()).unwrap_or(u64::MAX),
                        WHOLE_HISTORY_STEP,
                    )
                    .await
                    .map_err(|error| internal::<S>(&error))?;
                let done = u64::try_from(step.len()).is_ok_and(|count| count < WHOLE_HISTORY_STEP);
                rows.extend(step);
                if done {
                    break;
                }
            }
            let reader = self.reading.actor.person.id;
            let mut placed: Vec<(Spot, HistoryEntry)> = rows
                .into_iter()
                .map(|row| {
                    let own = row.by == Some(reader);
                    (spot_of(&row, own || self.sees_moments), row)
                })
                .collect();
            placed.sort_by_key(|(spot, _)| Reverse(*spot));
            self.ordered = Some(placed);
        }
        Ok(self.ordered.as_deref().unwrap_or_default())
    }
}

impl<S: AuthStore> Walk for DatedHistory<'_, '_, '_, S> {
    type Row = Dated;
    type Key = Spot;

    async fn fetch(
        &mut self,
        after: Option<&Spot>,
        skip: u64,
        limit: u64,
    ) -> Result<Vec<Dated>, AppError> {
        let whole = self.whole().await?;
        let first = after.map_or(0, |after| whole.partition_point(|(spot, _)| spot >= after));
        let first = first.saturating_add(usize::try_from(skip).unwrap_or(usize::MAX));
        let rows: Vec<(Spot, HistoryEntry)> = whole
            .iter()
            .skip(first)
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .cloned()
            .collect();
        let entries: Vec<HistoryEntry> = rows.iter().map(|(_, row)| row.clone()).collect();
        let batch = self.ctx.batch_of(&entries, self.kinds).await?;
        let mut shaped = Vec::with_capacity(rows.len());
        for (spot, row) in &rows {
            let mut item = self
                .ctx
                .history_item(self.reading, self.issue, &batch, row)
                .await?;
            if let Some(item) = item.as_mut() {
                // The row is shown at the moment it is placed by.
                item.at = Timestamp::from_millisecond(spot.millis).unwrap_or(item.at);
            }
            shaped.push(Dated { spot: *spot, item });
        }
        Ok(shaped)
    }

    fn costly_rows(&self) -> bool {
        true
    }

    async fn keeps(&mut self, row: &Dated) -> Result<bool, AppError> {
        Ok(row.item.is_some())
    }

    fn key_of(row: &Dated) -> Result<Spot, AppError> {
        Ok(row.spot)
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
    let start = SkipCursor::<Spot>::read(asked.cursor)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    let sees_moments = ctx
        .evaluation_of(reading.actor, bb_policy::Place::Organization)
        .await?
        .allows(Permission::SeeWorkingHoursOfOthers);
    let mut walk = DatedHistory {
        ctx,
        issue,
        kind,
        kinds,
        reading,
        sees_moments,
        ordered: None,
    };
    let filled = fill_past_hidden(&mut walk, &start, asked.len).await?;
    Ok(held.authorize(HistoryPage {
        items: filled.rows.into_iter().filter_map(|row| row.item).collect(),
        next_cursor: filled.next.as_ref().map(SkipCursor::write),
    }))
}
