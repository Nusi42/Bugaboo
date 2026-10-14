//! The one answer shape of a long preview (M2C-F80, UX principle 6): a summary first, which is
//! how many things will change and how many are left alone. The previews of moving a project, of
//! changing many issues, and of renaming and combining tags and labels all carry it, so the screen
//! has one component for the top of all of them.

use bb_app::preview::PreviewSummary;
use serde::Serialize;
use utoipa::ToSchema;

/// How many things a preview says will change and how many it leaves alone.
#[derive(Serialize, ToSchema)]
pub(crate) struct PreviewSummaryBody {
    /// How many will change (issues, people).
    will_change: u32,
    /// How many will be skipped.
    skipped: u32,
}

impl From<PreviewSummary> for PreviewSummaryBody {
    fn from(summary: PreviewSummary) -> Self {
        Self {
            will_change: summary.will_change,
            skipped: summary.skipped,
        }
    }
}
