//! *Remove a person* (PRIV-04, ADR-0044, decisions 6 and 7): the plan that says what goes, what
//! stays as *Former user* and what the law keeps until when, and the change that removes them.

mod change;
mod plan;
mod store;

pub use change::{
    Removal, RemovalAsked, RemovePerson, Removed, prepare_removal, remove_in, remove_person,
};
pub use plan::{KeptByLaw, PlanKind, RemovalPlan, RemovalPlanQuery, removal_plan};
pub use store::{NAME_LETTERS_MIN, PersonWords, RemovalStore, Scrub};
