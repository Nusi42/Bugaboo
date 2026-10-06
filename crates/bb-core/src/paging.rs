//! How big a page of a list is (PERF-02): defined once, for the use cases that list and for the
//! API that documents and cuts the page size.

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
    fn perf_02_the_page_size_is_at_least_one_and_at_most_the_maximum() {
        assert_eq!(page_size(None), PAGE_SIZE_DEFAULT);
        assert_eq!(page_size(Some(0)), 1);
        assert_eq!(page_size(Some(PAGE_SIZE_MAX)), PAGE_SIZE_MAX);
        assert_eq!(page_size(Some(u32::MAX)), PAGE_SIZE_MAX);
    }
}
