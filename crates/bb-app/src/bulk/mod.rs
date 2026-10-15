//! Changing many issues at once (ISS-09): select issues, see what would happen to each, then do it.
//!
//! - `preview.rs`: the preview, one row for each issue with what changes or why it is left alone.
//! - `judge.rs`: the one judgment of an issue that the preview shows and the change acts on.
//! - `apply.rs`: the change, each issue by the use case of a single change, so it has its own event;
//!   more than 1,000 issues become a job.
//! - `run.rs`: a job in the background: how far it is, and a step of it.
//!
//! Three rules run through all of it:
//! - **The preview is what happens.** Both call `judge_item`; the change then calls the use case of
//!   a single change, which judges the same thing again.
//! - **An issue that changed after the preview is named and left alone** (the version the preview
//!   read is the version the change expects).
//! - **One event for each issue**, made by the single change: History needs no knowledge of this.

mod apply;
mod judge;
mod preview;
mod run;
mod store;
mod types;

pub use apply::apply_bulk;
pub use preview::preview_bulk;
#[cfg(test)]
pub(crate) use run::step_payload;
pub use run::{bulk_run, cancel_bulk_run};
pub use store::BulkStore;
pub use types::*;
