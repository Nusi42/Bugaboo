//! How big a page of a list is (PERF-02), how a page is cut and how its position is written:
//! defined once, for the use cases that list and for the API that documents and cuts the page
//! size.

use std::fmt::Display;
use std::str::FromStr;

use jiff::Timestamp;

/// The most items one page holds. A bigger request is cut to this.
pub const PAGE_SIZE_MAX: u32 = 100;
/// The most rows one request for a list looks at while it skips the ones the person may not see
/// (any more): a person who lost access to thousands of rows must not make one request read them
/// all. When the limit is reached the page is shorter and the answer says there is more. The
/// notifications, the projects to choose from for an access key and the saved searches shared with
/// a person all walk by this one rule.
pub const LOOKED_AT_MAX: usize = 500;

/// The most rows a cursor of [`SkipCursor`] may ask to leave out: far more than any list holds, and
/// far below what a database can take as an offset, so a forged cursor is refused and never reaches
/// the store.
pub const SKIP_MAX: u64 = 1_000_000;

/// Where a page of a list that skips the rows a person may not see starts: after a row the person
/// was shown (or the start) and past a number of rows, always a multiple of [`LOOKED_AT_MAX`].
///
/// A cursor is readable, so it holds nothing a person may not know about: the key is the last row
/// of the page before, which the person has seen, and the number only counts whole stretches that
/// were looked at without filling a page, which says no more than the page that stopped there
/// ("there is more"). It never holds the key of a row that is hidden from the person, and never a
/// count of how many there are between two they can see. The text is `KEY`, `KEY+500`, `+500`, or
/// nothing for the start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkipCursor<K> {
    /// The row to start after.
    pub after: Option<K>,
    /// How many rows after it to leave out.
    pub skip: u64,
}

impl<K: FromStr + Display> SkipCursor<K> {
    /// The start of a list.
    #[must_use]
    pub const fn start() -> Self {
        Self {
            after: None,
            skip: 0,
        }
    }

    /// What the cursor text says; `None` for text that is none of the forms, a skip that is not a
    /// whole number of stretches or is above [`SKIP_MAX`], and a key that is none.
    #[must_use]
    pub fn read(cursor: Option<&str>) -> Option<Self> {
        let Some(text) = cursor else {
            return Some(Self::start());
        };
        let (key, skip) = match text.split_once('+') {
            Some((key, skip)) => (key, skip.parse::<u64>().ok()?),
            None => (text, 0),
        };
        if skip % (LOOKED_AT_MAX as u64) != 0 || skip > SKIP_MAX {
            return None;
        }
        let after = if key.is_empty() {
            None
        } else {
            Some(key.parse().ok()?)
        };
        Some(Self { after, skip })
    }

    /// The text of the cursor.
    #[must_use]
    pub fn write(&self) -> String {
        let key = self
            .after
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        if self.skip == 0 {
            key
        } else {
            format!("{key}+{}", self.skip)
        }
    }
}

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
mod skip_cursor_tests {
    use super::*;

    type Cursor = SkipCursor<u32>;

    #[test]
    fn a_cursor_is_a_key_a_count_of_whole_stretches_or_both_and_reads_back() {
        for (text, after, skip) in [
            ("", None, 0),
            ("7", Some(7), 0),
            ("7+500", Some(7), 500),
            ("+1000", None, 1000),
        ] {
            let read = Cursor::read((!text.is_empty()).then_some(text)).unwrap();
            assert_eq!((read.after, read.skip), (after, skip), "{text}");
            assert_eq!(read.write(), text);
        }
        assert_eq!(Cursor::read(None), Some(Cursor::start()));
    }

    #[test]
    fn a_cursor_that_does_not_fit_is_refused_and_never_a_panic() {
        for text in [
            "x",
            "7+",
            "7+499",
            "7+-500",
            "+500.5",
            "7+9223372036854776000",
        ] {
            assert_eq!(Cursor::read(Some(text)), None, "{text}");
        }
        let at_most = SKIP_MAX - SKIP_MAX % LOOKED_AT_MAX as u64;
        assert!(Cursor::read(Some(&format!("+{at_most}"))).is_some());
        assert_eq!(Cursor::read(Some(&format!("+{}", at_most + 500))), None);
    }
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
