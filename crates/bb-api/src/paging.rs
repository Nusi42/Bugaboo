//! Pages of a list: the page size and the cursor (PERF-02, architecture §7 "Performance").
//!
//! A cursor is **opaque**: the use case that lists says where a page ends (the sort key and the
//! ID of the last item, in whatever form it reads back) and [`seal`] turns that into text for the
//! client; [`open`] gives the position back. The text carries a short check value, so a cursor
//! that was changed, cut or made up is refused with `request.bad_cursor` before the position is
//! used. The check is not a signature: a cursor only says where to continue, and the list
//! applies the same permissions whatever the position.

use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_core::ErrorCode;
use sha2::{Digest as _, Sha256};

use crate::errors::ApiError;

/// The most items one page holds. A bigger `limit` is cut to this.
pub(crate) const MAX_LIMIT: u32 = 100;
/// How many items a page holds when the request does not say.
pub(crate) const DEFAULT_LIMIT: u32 = 50;

/// How many bytes of the hash are kept as the check value.
const CHECK_BYTES: usize = 8;
/// Keeps the check value of a cursor apart from any other hash made in the server.
const CHECK_DOMAIN: &[u8] = b"bugaboo.cursor.v1\0";

/// The page size to use: what was asked for, at least 1 and at most [`MAX_LIMIT`].
pub(crate) fn limit(asked: Option<u32>) -> u32 {
    asked.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}

fn check_value(position: &[u8]) -> [u8; CHECK_BYTES] {
    let digest = Sha256::new()
        .chain_update(CHECK_DOMAIN)
        .chain_update(position)
        .finalize();
    let mut check = [0_u8; CHECK_BYTES];
    check.copy_from_slice(&digest[..CHECK_BYTES]);
    check
}

/// The cursor text for a page that ends at `position`.
pub(crate) fn seal(position: &str) -> String {
    let mut bytes = check_value(position.as_bytes()).to_vec();
    bytes.extend_from_slice(position.as_bytes());
    Base64UrlUnpadded::encode_string(&bytes)
}

/// The position inside a cursor.
///
/// # Errors
/// `request.bad_cursor` when the text is not a cursor this server made, or was changed.
pub(crate) fn open(cursor: &str) -> Result<String, ApiError> {
    let bad = || ApiError::new(ErrorCode::BadCursor);
    let bytes = Base64UrlUnpadded::decode_vec(cursor).map_err(|_| bad())?;
    let (check, position) = bytes.split_at_checked(CHECK_BYTES).ok_or_else(bad)?;
    if check != check_value(position) {
        return Err(bad());
    }
    String::from_utf8(position.to_vec()).map_err(|_| bad())
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A list of `count` items, each at the position `"{n:05}"`, walked with the helpers the way
    /// a list route does: ask for `size` items after the cursor, read one more to know whether
    /// there is a next page, answer a sealed cursor.
    fn walk(count: u32, size: u32) -> Result<Vec<u32>, ApiError> {
        let size = limit(Some(size));
        let mut seen = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let after = cursor.as_deref().map(open).transpose()?;
            let start = after.map_or(0, |position| position.parse::<u32>().unwrap_or(0) + 1);
            let page: Vec<u32> = (start..count).take(size as usize + 1).collect();
            let more = page.len() > size as usize;
            let page = &page[..page.len().min(size as usize)];
            seen.extend_from_slice(page);
            match (more, page.last()) {
                (true, Some(last)) => cursor = Some(seal(&last.to_string())),
                _ => return Ok(seen),
            }
        }
    }

    proptest! {
        #[test]
        fn perf_02_paging_walks_every_item_once_and_refuses_a_changed_cursor(
            count in 0_u32..400,
            size in 0_u32..250,
            position in "[ -~]{0,40}",
            at in any::<prop::sample::Index>(),
            replacement in prop::sample::select(
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_".to_vec()
            ),
        ) {
            // Every item once and in order, whatever the page size (0 and over 100 are cut).
            let walked = walk(count, size).unwrap();
            prop_assert_eq!(walked, (0..count).collect::<Vec<_>>());

            // The position comes back as it went in.
            let cursor = seal(&position);
            prop_assert_eq!(open(&cursor).unwrap(), position);

            // Changing any one character of the cursor makes it a bad one.
            let mut changed = cursor.clone().into_bytes();
            let place = at.index(changed.len());
            prop_assume!(changed[place] != replacement);
            changed[place] = replacement;
            let changed = String::from_utf8(changed).unwrap();
            let error = open(&changed).unwrap_err();
            prop_assert_eq!(error.code(), ErrorCode::BadCursor);
        }
    }

    #[test]
    fn perf_02_a_cursor_that_is_cut_empty_or_made_up_is_refused() {
        let sealed = seal("1700000000.abc");
        for bad in [
            "",
            "x",
            "!!!!",
            &sealed[..sealed.len() - 1],
            &sealed[..4],
            "MTcwMDAwMDAwMC5hYmM",
        ] {
            let error = open(bad).unwrap_err();
            assert_eq!(error.code(), ErrorCode::BadCursor, "{bad}");
        }
    }

    #[test]
    fn perf_02_the_page_size_is_at_least_one_and_at_most_the_maximum() {
        assert_eq!(limit(None), DEFAULT_LIMIT);
        assert_eq!(limit(Some(0)), 1);
        assert_eq!(limit(Some(MAX_LIMIT)), MAX_LIMIT);
        assert_eq!(limit(Some(u32::MAX)), MAX_LIMIT);
    }

    #[test]
    fn a_cursor_is_url_safe_text() {
        let cursor = seal("a/b+c=d");
        assert!(
            cursor
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }
}
