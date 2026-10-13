//! The one answer shape of a long preview (UX principle 6, M2C-F80): a summary first ("118 will
//! change, 2 will be skipped"), then the details grouped by what the preview is about (by issue or
//! by person), which the screen shows a few at a time with *Show all*. The preview of moving a
//! project and the previews of changing many issues, renaming and combining use it, so the
//! screen has one component for all of them.

use bb_policy::Masked;
use serde::Serialize;

/// How many things a preview says will change and how many it leaves alone: the sentence at the
/// top of a long preview.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct PreviewSummary {
    /// How many will change (issues, people).
    pub will_change: u32,
    /// How many will be skipped.
    pub skipped: u32,
}

impl PreviewSummary {
    /// The summary of a preview that counts what will change and what is skipped. A count beyond
    /// what a number of this size holds stops at the largest (a preview names far fewer).
    #[must_use]
    pub fn of(will_change: usize, skipped: usize) -> Self {
        Self {
            will_change: u32::try_from(will_change).unwrap_or(u32::MAX),
            skipped: u32::try_from(skipped).unwrap_or(u32::MAX),
        }
    }
}

impl Masked for PreviewSummary {}
