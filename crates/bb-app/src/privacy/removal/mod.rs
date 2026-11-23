//! *Remove a person* (PRIV-04, ADR-0044, decisions 6 and 7): the plan that says what goes, what
//! stays as *Former user* and what the law keeps until when, and the change that removes them.

mod change;
mod finish;
mod plan;
mod store;

pub use change::{
    NotPrepared, Removal, RemovePerson, Removed, SealedWords, prepare_removal, remove_in,
    remove_person,
};
pub use finish::{EarlierFinish, FINISH_REMOVAL_COMMENTS, FinishRemoval, FinishStep};
pub use plan::{KeptByLaw, PlanKind, RemovalPlan, RemovalPlanQuery, removal_plan};
pub use store::{NAME_LETTERS_MIN, PersonWords, RemovalStore, Scrub, ScrubPlace, ScrubbedTable};
