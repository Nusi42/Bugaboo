//! The limits of texts people type, as constants for the browser client.

use super::super::TS_HEADER;

/// The limits of texts people type that a box of the browser app keeps to, from the linked
/// `bb-core`: the most characters of a reason, which the server checks again.
pub(crate) fn texts() -> String {
    format!(
        "{TS_HEADER}/** The most characters of a reason (`REASON_MAX`). */
export const reasonMax = {reason};
",
        reason = bb_core::text::REASON_MAX,
    )
}
