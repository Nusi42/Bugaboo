//! The plan of the timeline (PLAN-08, PLAN-10, PLAN-12, PLAN-13, PLAN-14, ADR-0030 and its
//! amendment M8, ADR-0043): worked out once, in Rust, on the server, for the reader. The rules are
//! in `bb_core::plan`; this module reads what they need and answers what the reader may see.
//!
//! - `factor.rs`: the time factor of an issue, who set it, and its reset with the first person.
//! - `store.rs`: the port of what the plan keeps (time factors, accepted warnings).
//!
//! Restricted issues are left out entirely: no bar, no arrow, no warning, no count. The minutes of
//! a day come only from the port of work times ([`Ctx::work_minutes`](crate::auth::Ctx)).

mod build;
mod factor;
mod read;
mod store;
mod types;
mod ways;

pub(crate) use build::DateTargets;
pub use factor::{
    FactorGiven, SetTimeFactor, TimeFactorQuery, TimeFactorView, person_on_bar, reset_with_first,
    set_time_factor, time_factor,
};
pub use read::{timeline, timeline_groups_by};
pub use store::TimelineStore;
pub use types::*;
pub use ways::{accept_warning, move_after_preview};
