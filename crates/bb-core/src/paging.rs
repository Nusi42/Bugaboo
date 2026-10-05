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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_page_size_is_at_least_one_and_at_most_the_maximum() {
        assert_eq!(page_size(None), PAGE_SIZE_DEFAULT);
        assert_eq!(page_size(Some(0)), 1);
        assert_eq!(page_size(Some(PAGE_SIZE_MAX)), PAGE_SIZE_MAX);
        assert_eq!(page_size(Some(u32::MAX)), PAGE_SIZE_MAX);
    }
}
