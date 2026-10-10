//! Clears what outlived its keep period while the server runs (M3-F30).
//!
//! A timer runs the use case `purge_drafts` when the server starts and once a day after that:
//! drafts past their 30 days, notes for people who were editing, and lock rows that ran out. It
//! stands in for the job `editing.purge_drafts` until the job runner exists, and then moves into
//! it. A round that fails is reported in the log and tried again at the next tick.

use std::time::Duration;

use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;

use crate::wiring::Housekeeping;

/// How often the clearing runs.
pub(super) const CLEAR_EVERY: Duration = Duration::from_hours(24);

/// Starts the timer; the first round runs at once. The caller stops it with [`JoinHandle::abort`]
/// when the server stops.
pub(super) fn spawn(housekeeping: Housekeeping, every: Duration) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticks = tokio::time::interval(every);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticks.tick().await;
            match housekeeping.clear_old().await {
                Ok(drafts) => tracing::info!(drafts, "old drafts were cleared"),
                Err(error) => tracing::warn!(
                    cause = %error,
                    "old drafts could not be cleared; the next round tries again"
                ),
            }
        }
    })
}
