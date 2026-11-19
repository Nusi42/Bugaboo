//! *Remove a person* (PRIV-04, ADR-0044, decisions 6 and 7): the plan that says what goes, what
//! stays as *Former user* and what the law keeps until when, and the change that removes them.

mod plan;
mod store;

pub use plan::{KeptByLaw, PlanKind, RemovalPlan, RemovalPlanQuery, removal_plan};
pub use store::{RemovalStore, Scrub};
