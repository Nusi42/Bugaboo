//! A page of a list that skips the rows a person may not see (any more), written once: the projects
//! to choose from for an access key and the saved searches shared with a person walk by it.
//!
//! The walk reads [`PAGE_SIZE_MAX`] rows at a time, asks of each whether the person may see it, and
//! stops when the page is full, the rows are used up or [`LOOKED_AT_MAX`] rows were looked at, so
//! the work of a request does not depend on how many rows are hidden (PERF-05). A page that stops
//! at the bound is short and has a cursor that goes on past the rows looked at; it never holds the
//! key of a row that was skipped ([`SkipCursor`]). A list whose rows cost a read for each name they
//! hold can also end a page before a row that is more than the request may spend, and goes on
//! after the last row shown.

use std::future::Future;
use std::str::FromStr;

use bb_core::paging::{LOOKED_AT_MAX, PAGE_SIZE_MAX, SKIP_MAX, SkipCursor};

use crate::auth::AppError;

/// What a walk needs to know of the list it goes through.
pub(crate) trait Walk: Send {
    /// A row of the list.
    type Row: Send;
    /// What a row is known by, in the cursor.
    type Key: FromStr + std::fmt::Display + Clone + Send + Sync;

    /// The rows after `after`, leaving out the first `skip`, at most `limit`.
    fn fetch(
        &mut self,
        after: Option<&Self::Key>,
        skip: u64,
        limit: u64,
    ) -> impl Future<Output = Result<Vec<Self::Row>, AppError>> + Send;

    /// Whether the person may see the row.
    fn keeps(&mut self, row: &Self::Row) -> impl Future<Output = Result<bool, AppError>> + Send;

    /// Whether the last row judged needed more work than the request may spend: its answer from
    /// [`Walk::keeps`] is no answer, and the page ends before it (a list whose rows cost a read for
    /// each name of their text). A walk whose rows cost the same says no.
    fn budget_spent(&self) -> bool {
        false
    }

    /// Lifts that limit for the rest of the request.
    fn lift_budget(&mut self) {}

    /// Whether a row costs work to judge, so the first batch is no more than the page needs
    /// (the length asked for, and one more) and the next batches double only while rows are
    /// skipped. A walk whose rows are cheap reads [`PAGE_SIZE_MAX`] rows at a time.
    fn costly_rows(&self) -> bool {
        false
    }

    /// The key of a row.
    ///
    /// # Errors
    /// An internal error for a stored row whose key cannot be read: the list is not cut short.
    fn key_of(row: &Self::Row) -> Result<Self::Key, AppError>;
}

/// A page of rows and where the next one starts.
pub(crate) struct Filled<W: Walk> {
    /// The rows of the page, at most the length asked for.
    pub(crate) rows: Vec<W::Row>,
    /// The cursor of the next page; none when the rows are used up.
    pub(crate) next: Option<SkipCursor<W::Key>>,
}

/// Fills a page of `len` rows that `walk` keeps, starting at `start`.
///
/// # Errors
/// The error of the walk.
pub(crate) async fn fill_past_hidden<W: Walk>(
    walk: &mut W,
    start: &SkipCursor<W::Key>,
    len: usize,
) -> Result<Filled<W>, AppError> {
    let largest = u64::from(PAGE_SIZE_MAX);
    let bound = LOOKED_AT_MAX as u64;
    let mut batch = if walk.costly_rows() {
        largest.min(u64::try_from(len).unwrap_or(largest).saturating_add(1))
    } else {
        largest
    };
    let (mut rows, mut looked_at, mut more) = (Vec::new(), 0_u64, false);
    'scan: while looked_at < bound {
        batch = batch.min(bound - looked_at);
        let found = walk
            .fetch(start.after.as_ref(), start.skip + looked_at, batch)
            .await?;
        let full = u64::try_from(found.len()).is_ok_and(|count| count == batch);
        for row in found {
            let mut kept = walk.keeps(&row).await?;
            if walk.budget_spent() {
                if rows.is_empty() {
                    // No row to continue after, and a cursor may name only a row that was shown:
                    // go on to the first row that can be, whatever it costs (bounded by the
                    // rows looked at).
                    walk.lift_budget();
                    kept = walk.keeps(&row).await?;
                } else {
                    // The page ends before this row, and the next one starts after the last row
                    // shown, with a budget of its own.
                    more = true;
                    break 'scan;
                }
            }
            if kept {
                if rows.len() == len {
                    more = true;
                    break 'scan;
                }
                rows.push(row);
            }
            looked_at += 1;
        }
        if !full {
            break;
        }
        if walk.costly_rows() {
            batch = (batch * 2).min(largest);
        }
    }
    let next = if more {
        // A full page continues after its last row, which the person has just been shown.
        match rows.last() {
            Some(last) => Some(SkipCursor {
                after: Some(W::key_of(last)?),
                skip: 0,
            }),
            None => None,
        }
    } else if looked_at >= bound && start.skip + bound <= SKIP_MAX {
        // A page that looked at enough rows continues past them, as far as a cursor can say: the
        // reader refuses a skip above SKIP_MAX, so the list ends there (never a cursor it refuses).
        Some(SkipCursor {
            after: start.after.clone(),
            skip: start.skip + bound,
        })
    } else {
        None
    };
    Ok(Filled { rows, next })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A list of rows nobody may see, as long as anybody asks.
    struct AllHidden;

    impl Walk for AllHidden {
        type Row = u64;
        type Key = u64;

        async fn fetch(
            &mut self,
            _after: Option<&u64>,
            skip: u64,
            limit: u64,
        ) -> Result<Vec<u64>, AppError> {
            Ok((skip..skip + limit).collect())
        }

        async fn keeps(&mut self, _row: &u64) -> Result<bool, AppError> {
            Ok(false)
        }

        fn key_of(row: &u64) -> Result<u64, AppError> {
            Ok(*row)
        }
    }

    /// The skip of the cursor that the walk writes, read back as a client sends it.
    async fn next_after_skipping(skip: u64) -> Result<Option<u64>, AppError> {
        let start = SkipCursor { after: None, skip };
        let filled = fill_past_hidden(&mut AllHidden, &start, 10).await?;
        Ok(filled.next.map(|next| {
            let read = SkipCursor::<u64>::read(Some(&next.write()));
            read.map_or(u64::MAX, |read| read.skip)
        }))
    }

    #[tokio::test]
    async fn qry_06_a_walk_never_writes_a_cursor_that_the_reader_refuses() -> Result<(), AppError> {
        // The last stretch that a cursor can say is written, and read back.
        let last = SKIP_MAX - LOOKED_AT_MAX as u64;
        assert_eq!(next_after_skipping(last).await?, Some(SKIP_MAX));
        // The page after it ends the list: there is no cursor to write.
        assert_eq!(next_after_skipping(SKIP_MAX).await?, None);
        Ok(())
    }

    /// A list of three rows whose row `spent_at` is more than the request may spend, until the
    /// bound is lifted: then it is judged in full, by `truth` (the rows that are kept).
    struct Costly {
        spent_at: u64,
        truth: [bool; 3],
        lifted: bool,
        spent: bool,
        lifts: u32,
    }

    impl Costly {
        fn new(spent_at: u64, truth: [bool; 3]) -> Self {
            Self {
                spent_at,
                truth,
                lifted: false,
                spent: false,
                lifts: 0,
            }
        }
    }

    impl Walk for Costly {
        type Row = u64;
        type Key = u64;

        async fn fetch(
            &mut self,
            after: Option<&u64>,
            skip: u64,
            limit: u64,
        ) -> Result<Vec<u64>, AppError> {
            let first = after.map_or(0, |key| key + 1) + skip;
            Ok((first..3.min(first + limit)).collect())
        }

        async fn keeps(&mut self, row: &u64) -> Result<bool, AppError> {
            self.spent = *row == self.spent_at && !self.lifted;
            // A search out of budget is answered provisionally, and the answer must not be used.
            Ok(self.spent || self.truth[usize::try_from(*row).unwrap_or(0)])
        }

        fn budget_spent(&self) -> bool {
            self.spent
        }

        fn lift_budget(&mut self) {
            self.lifted = true;
            self.lifts += 1;
        }

        fn key_of(row: &u64) -> Result<u64, AppError> {
            Ok(*row)
        }
    }

    #[tokio::test]
    async fn perf_05_a_spent_search_on_an_empty_page_is_judged_again_and_shown_once()
    -> Result<(), AppError> {
        // The first search is out of budget and, in truth, hidden: the page has no row yet, so the
        // bound is lifted and the search judged again; the provisional answer is never used.
        let mut walk = Costly::new(0, [false, true, true]);
        let filled = fill_past_hidden(&mut walk, &SkipCursor::start(), 5).await?;
        assert_eq!(filled.rows, [1, 2]);
        assert_eq!(walk.lifts, 1);
        assert!(filled.next.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn perf_05_a_spent_search_after_a_shown_one_ends_the_page_after_the_shown_one()
    -> Result<(), AppError> {
        let mut walk = Costly::new(1, [true, true, true]);
        let filled = fill_past_hidden(&mut walk, &SkipCursor::start(), 5).await?;
        assert_eq!(filled.rows, [0]);
        assert_eq!(walk.lifts, 0);
        let next = filled
            .next
            .ok_or(AppError::Internal("no cursor".to_owned()))?;
        assert_eq!((next.after, next.skip), (Some(0), 0));
        Ok(())
    }

    /// A list that records how many rows each fetch asked for; the rows below `hidden` are hidden.
    struct Recording {
        costly: bool,
        hidden: u64,
        asked: Vec<u64>,
    }

    impl Walk for Recording {
        type Row = u64;
        type Key = u64;

        async fn fetch(
            &mut self,
            _after: Option<&u64>,
            skip: u64,
            limit: u64,
        ) -> Result<Vec<u64>, AppError> {
            self.asked.push(limit);
            Ok((skip..(skip + limit).min(1000)).collect())
        }

        async fn keeps(&mut self, row: &u64) -> Result<bool, AppError> {
            Ok(*row >= self.hidden)
        }

        fn costly_rows(&self) -> bool {
            self.costly
        }

        fn key_of(row: &u64) -> Result<u64, AppError> {
            Ok(*row)
        }
    }

    #[tokio::test]
    async fn perf_05_a_walk_of_costly_rows_fetches_the_page_and_one_then_doubles_past_hidden_rows()
    -> Result<(), AppError> {
        // Nothing hidden: one fetch of the page and one more row, not a whole batch.
        let mut walk = Recording {
            costly: true,
            hidden: 0,
            asked: Vec::new(),
        };
        let filled = fill_past_hidden(&mut walk, &SkipCursor::start(), 10).await?;
        assert_eq!((walk.asked.as_slice(), filled.rows.len()), (&[11][..], 10));
        assert!(filled.next.is_some());
        // Hidden rows make the batches grow, up to the largest, and the rows looked at stay
        // within the bound.
        let mut walk = Recording {
            costly: true,
            hidden: 300,
            asked: Vec::new(),
        };
        fill_past_hidden(&mut walk, &SkipCursor::start(), 10).await?;
        assert_eq!(walk.asked, [11, 22, 44, 88, 100, 100]);
        // A list of cheap rows reads the largest batch at once.
        let mut walk = Recording {
            costly: false,
            hidden: 0,
            asked: Vec::new(),
        };
        fill_past_hidden(&mut walk, &SkipCursor::start(), 10).await?;
        assert_eq!(walk.asked, [u64::from(PAGE_SIZE_MAX)]);
        Ok(())
    }
}
