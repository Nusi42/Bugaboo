//! The settings of the plan of the timeline (PLAN-14, ADR-0033, amendment M8): whether the person
//! on the bar is told when somebody else changes the time factor of their issue. They are part of
//! the registry like every other setting ([`super::registry`] lists them) and stand in a file of
//! their own, like the settings of work times.

mod catalog;

pub use catalog::*;
