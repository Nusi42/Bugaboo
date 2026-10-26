//! A page of a list that skips the rows a person may not see (any more), written once: the projects
//! to choose from for an access key and the saved searches shared with a person walk by it.
//!
//! The walk reads [`PAGE_SIZE_MAX`] rows at a time, asks of each whether the person may see it, and
//! stops when the page is full, the rows are used up or [`LOOKED_AT_MAX`] rows were looked at, so
//! the work of a request does not depend on how many rows are hidden (PERF-05). A page that stops
//! at the bound is short and has a cursor that goes on past the rows looked at; it never holds the
//! key of a row that was skipped ([`SkipCursor`]).

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
    let batch = u64::from(PAGE_SIZE_MAX);
    let bound = LOOKED_AT_MAX as u64;
    let (mut rows, mut looked_at, mut more) = (Vec::new(), 0_u64, false);
    'scan: while looked_at < bound {
        let found = walk
            .fetch(start.after.as_ref(), start.skip + looked_at, batch)
            .await?;
        let full = u64::try_from(found.len()).is_ok_and(|count| count == batch);
        for row in found {
            if walk.keeps(&row).await? {
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
}
