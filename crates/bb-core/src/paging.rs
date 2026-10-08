//! How big a page of a list is (PERF-02), how a page is cut and how its position is written:
//! defined once, for the use cases that list and for the API that documents and cuts the page
//! size.

use std::fmt::Display;
use std::str::FromStr;

use jiff::Timestamp;

/// The most items one page holds. A bigger request is cut to this.
pub const PAGE_SIZE_MAX: u32 = 100;
/// How many items a page holds when the request does not say.
pub const PAGE_SIZE_DEFAULT: u32 = 50;

/// The page size to use: what was asked for (or the default), at least 1 and at most
/// [`PAGE_SIZE_MAX`].
#[must_use]
pub fn page_size(asked: Option<u32>) -> u32 {
    asked.unwrap_or(PAGE_SIZE_DEFAULT).clamp(1, PAGE_SIZE_MAX)
}

/// The page size as a count of items, for a list that is cut in memory.
#[must_use]
pub fn page_len(asked: Option<u32>) -> usize {
    usize::try_from(page_size(asked)).unwrap_or(usize::MAX)
}

/// How many items a read asks the store for to fill a page of `len`: one more, to know whether
/// there is a next page.
#[must_use]
pub fn read_len(len: usize) -> u64 {
    u64::try_from(read_count(len)).unwrap_or(u64::MAX)
}

/// [`read_len`] as a count of items, for a list that is cut in memory.
#[must_use]
pub fn read_count(len: usize) -> usize {
    len.saturating_add(1)
}

/// A page cut from what a read of [`read_len`] items found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cut<T> {
    /// The items of the page: at most `len`.
    pub items: Vec<T>,
    /// There is a next page: the read found more than `len`.
    pub more: bool,
}

impl<T> Cut<T> {
    /// The page of at most `len` items out of `found`.
    #[must_use]
    pub fn of(mut found: Vec<T>, len: usize) -> Self {
        let more = found.len() > len;
        found.truncate(len);
        Self { items: found, more }
    }

    /// The cursor of the next page, made from the last item of this one; `None` on the last page.
    #[must_use]
    pub fn next_cursor(&self, write: impl FnOnce(&T) -> String) -> Option<String> {
        self.more.then(|| self.items.last().map(write)).flatten()
    }
}

/// Where a page of a list ordered by time and then by ID ends, as text: the time of the last item
/// (microseconds) and its ID.
#[must_use]
pub fn write_time_cursor(time: Timestamp, id: impl Display) -> String {
    format!("{}.{id}", time.as_microsecond())
}

/// The position [`write_time_cursor`] wrote; `None` for text that is not one.
#[must_use]
pub fn read_time_cursor<I: FromStr>(text: &str) -> Option<(Timestamp, I)> {
    let (micros, id) = text.split_once('.')?;
    let time = Timestamp::from_microsecond(micros.parse().ok()?).ok()?;
    Some((time, id.parse().ok()?))
}

/// Where one page of a list that is held in memory starts and ends, and the cursor of the next
/// page, for a list of `len` items. The cursor of a page is the number of items before it, as
/// text: use it only for short lists that a use case builds whole (a person's rows, a group's
/// members); a long list is read by keys from the database. `None` for a cursor that is not one.
#[must_use]
pub fn window(len: usize, cursor: Option<&str>, limit: u32) -> Option<Window> {
    let start = match cursor {
        Some(text) => text.parse::<usize>().ok()?,
        None => 0,
    };
    let start = start.min(len);
    let wanted = usize::try_from(page_size(Some(limit))).unwrap_or(usize::MAX);
    let end = start.saturating_add(wanted).min(len);
    Some(Window {
        start,
        end,
        next: (end < len).then(|| end.to_string()),
    })
}

/// One page of a list held in memory (see [`window`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    /// The first item of the page.
    pub start: usize,
    /// One past the last item of the page.
    pub end: usize,
    /// The cursor of the next page; `None` on the last page.
    pub next: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_a_list_in_memory_is_cut_into_pages_that_walk_every_item_once() {
        let mut seen = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let page = window(7, cursor.as_deref(), 3).unwrap();
            seen.extend(page.start..page.end);
            match page.next {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        assert_eq!(seen, (0..7).collect::<Vec<_>>());
        assert!(window(7, Some("not a number"), 3).is_none());
        // A cursor past the end is an empty last page, not an error: the list got shorter.
        let past = window(2, Some("9"), 3).unwrap();
        assert_eq!((past.start, past.end, past.next), (2, 2, None));
    }

    #[test]
    fn perf_02_a_page_read_one_item_too_long_is_cut_and_says_there_is_more() {
        assert_eq!(read_len(3), 4);
        let page = Cut::of(vec![1, 2, 3, 4], 3);
        assert_eq!((page.items.clone(), page.more), (vec![1, 2, 3], true));
        assert_eq!(page.next_cursor(ToString::to_string), Some("3".to_owned()));
        let last = Cut::of(vec![1, 2], 3);
        assert_eq!((last.items.clone(), last.more), (vec![1, 2], false));
        assert_eq!(last.next_cursor(ToString::to_string), None);
    }

    #[test]
    fn perf_02_the_position_of_a_page_by_time_survives_a_round_trip() {
        let time = Timestamp::from_microsecond(1_767_225_600_123_456).unwrap();
        let text = write_time_cursor(time, 42_u32);
        assert_eq!(read_time_cursor::<u32>(&text), Some((time, 42)));
        for bad in ["", "7", "x.1", "7.x", "99999999999999999999.1"] {
            assert_eq!(read_time_cursor::<u32>(bad), None, "{bad}");
        }
    }

    #[test]
    fn perf_02_the_page_size_is_at_least_one_and_at_most_the_maximum() {
        assert_eq!(page_size(None), PAGE_SIZE_DEFAULT);
        assert_eq!(page_size(Some(0)), 1);
        assert_eq!(page_size(Some(PAGE_SIZE_MAX)), PAGE_SIZE_MAX);
        assert_eq!(page_size(Some(u32::MAX)), PAGE_SIZE_MAX);
    }
}
