//! The plan of the timeline (PLAN-10, PLAN-12, PLAN-13, PLAN-14, ADR-0030 and its amendment M8,
//! ADR-0043): the pure rules, worked out once, here, for the server (no rule of the plan lives in
//! the browser).
//!
//! - [`Days`]: a stretch of whole days, both ends included: the days of a bar.
//! - [`TimeFactor`]: the person's own estimate for an issue (*×1.5*, or *Takes me 10 days*).
//! - [`bar`]: the days of a bar (ADR-0043, decision 3) and *Plan by effort* (ADR-0030, decision 3).
//! - [`warning`]: the four plan warnings (ADR-0030, decision 5), one enum.
//! - [`order`]: the order along dependencies that *Move it after* moves issues in (ADR-0043,
//!   decision 5), on the one graph of dependencies ([`crate::relation::DependencyGraph`]).
//!
//! Pure: no clock and no I/O. The minutes of a day come from [`crate::work_times::minutes_on`]
//! through [`crate::work_times::DayMinutes`], which the use case reads for the person on the bar.
//! Worked-out days are answered, never stored: only a change of an issue writes a date.

pub mod bar;
mod days;
mod factor;
pub mod order;
pub mod record;
pub mod warning;

pub use days::Days;
pub use factor::{TIME_FACTOR_PERCENT_RANGE, TimeFactor};

/// The most rows one page of the timeline holds: a screen shows far fewer, and the plan of a page
/// is worked out within the budget of timeline data (PERF-01).
pub const TIMELINE_ROWS_MOST: usize = 200;

#[cfg(test)]
mod tests;
