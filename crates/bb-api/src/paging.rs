//! Cursors of a list (PERF-02, architecture §7 "Performance").
//!
//! A cursor is **opaque**: the use case that lists says where a page ends (the sort key and the
//! ID of the last item, in whatever form it reads back) and [`seal`] turns that into text for the
//! client; [`open`] gives the position back. The text carries a tag made under the key of this
//! server run (`crate::seal`) and the *purpose* of the list it was made for, so a cursor that was
//! changed, cut, made up, left over from before a restart or taken from another list is refused
//! with `request.bad-cursor` before the position is used.
//!
//! A cursor holds only a position. Every filter and every permission comes from the request, every
//! time, so a cursor can never widen what a person sees.

use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_core::ErrorCode;

use crate::errors::ApiError;
use crate::seal::{self, TAG_BYTES};

/// The purpose the tag of a cursor is bound to, before the name of the list.
const PURPOSE_PREFIX: &str = "cursor:";

fn purpose_of(list: &str) -> String {
    format!("{PURPOSE_PREFIX}{list}")
}

/// The cursor text for a page of `list` that ends at `position`.
///
/// # Errors
/// An internal error when the system has no random bytes for the key of the seal.
pub(crate) fn seal(list: &str, position: &str) -> Result<String, ApiError> {
    let tag = seal::process()
        .and_then(|seal| seal.tag(&purpose_of(list), position.as_bytes()))
        .ok_or_else(|| ApiError::internal(&"no random bytes for the key of the seal"))?;
    let mut bytes = tag.to_vec();
    bytes.extend_from_slice(position.as_bytes());
    Ok(Base64UrlUnpadded::encode_string(&bytes))
}

/// The position inside a cursor of `list`.
///
/// # Errors
/// `request.bad-cursor` when the text is not a cursor this server made for this list.
pub(crate) fn open(list: &str, cursor: &str) -> Result<String, ApiError> {
    let bad = || ApiError::new(ErrorCode::BadCursor);
    let bytes = Base64UrlUnpadded::decode_vec(cursor).map_err(|_| bad())?;
    let (tag, position) = bytes.split_at_checked(TAG_BYTES).ok_or_else(bad)?;
    let sealed = seal::process().is_some_and(|seal| seal.verify(&purpose_of(list), position, tag));
    if !sealed {
        return Err(bad());
    }
    String::from_utf8(position.to_vec()).map_err(|_| bad())
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const LIST: &str = "things";

    /// A list of `count` items, each at the position `"{n}"`, walked with the helpers the way a
    /// list route does: ask for `size` items after the cursor, read one more to know whether
    /// there is a next page, answer a sealed cursor.
    fn walk(count: u32, size: u32) -> Result<Vec<u32>, ApiError> {
        let size = bb_core::paging::page_size(Some(size));
        let mut seen = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let after = cursor.as_deref().map(|text| open(LIST, text)).transpose()?;
            let start = after.map_or(0, |position| position.parse::<u32>().unwrap_or(0) + 1);
            let page: Vec<u32> = (start..count).take(size as usize + 1).collect();
            let more = page.len() > size as usize;
            let page = &page[..page.len().min(size as usize)];
            seen.extend_from_slice(page);
            match (more, page.last()) {
                (true, Some(last)) => cursor = Some(seal(LIST, &last.to_string())?),
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
            let cursor = seal(LIST, &position).unwrap();
            prop_assert_eq!(open(LIST, &cursor).unwrap(), position);

            // Changing any one character of the cursor makes it a bad one.
            let mut changed = cursor.clone().into_bytes();
            let place = at.index(changed.len());
            prop_assume!(changed[place] != replacement);
            changed[place] = replacement;
            let changed = String::from_utf8(changed).unwrap();
            let error = open(LIST, &changed).unwrap_err();
            prop_assert_eq!(error.code(), ErrorCode::BadCursor);
        }
    }

    #[test]
    fn perf_02_a_cursor_that_is_cut_empty_made_up_or_for_another_list_is_refused() {
        let sealed = seal(LIST, "1700000000.abc").unwrap();
        // A well-formed text of the right length but without a tag of ours.
        let forged = Base64UrlUnpadded::encode_string(&[7_u8; 40]);
        for bad in [
            "",
            "x",
            "!!!!",
            &sealed[..sealed.len() - 1],
            &sealed[..4],
            "MTcwMDAwMDAwMC5hYmM",
            &forged,
        ] {
            let error = open(LIST, bad).unwrap_err();
            assert_eq!(error.code(), ErrorCode::BadCursor, "{bad}");
        }
        // Sealed by this server, but for another list: the same answer.
        let error = open("other", &sealed).unwrap_err();
        assert_eq!(error.code(), ErrorCode::BadCursor);
    }

    #[test]
    fn perf_02_a_sealed_position_that_is_not_text_is_refused_like_any_other_bad_cursor() {
        // Even a text that carries a valid tag is only accepted when the position is text.
        let junk = [0xFF_u8, 0xFE, 0xFD];
        let tag = seal::process()
            .unwrap()
            .tag(&purpose_of(LIST), &junk)
            .unwrap();
        let mut bytes = tag.to_vec();
        bytes.extend_from_slice(&junk);
        let cursor = Base64UrlUnpadded::encode_string(&bytes);
        assert_eq!(
            open(LIST, &cursor).unwrap_err().code(),
            ErrorCode::BadCursor
        );
    }

    #[test]
    fn a_cursor_is_url_safe_text() {
        let cursor = seal(LIST, "a/b+c=d").unwrap();
        assert!(
            cursor
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }
}
