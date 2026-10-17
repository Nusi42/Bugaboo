//! The sample organization (architecture §11): a fictional organization for development,
//! end-to-end tests and demos, made through the same use cases the API calls.
//!
//! A developer tool, not part of the product: nothing depends on this crate, and it is not linked
//! into `bugaboo` before *Try it out* (OPS-12). `cargo xtask dataset sample` runs its program
//! (`bb-sample`) as a process, so `xtask` depends on no product crate.
//!
//! - [`make`] makes the people: the first administrator with the setup code, everybody else as
//!   that administrator by invitation. Every change has its event and its security log entry.
//! - [`Later`] is the extension point for what comes after the people; [`Organization`] fills it
//!   with the groups, roles, projects and assignments (M2 WP-13).
//! - [`Sample::dump`] is the canonical dump that TEST-SAMPLE-DATA compares.
//! - The passwords are random for every run and are written only to `sign-in.txt` ([`Output`]).

#![forbid(unsafe_code)]

// The program (`src/main.rs`) is the only user of these, and Cargo gives every dependency to
// the library and the program alike (`unused_crate_dependencies`).
use clap as _;
use tokio as _;

mod clock;
mod comments;
mod dump;
mod dump_issues;
mod dump_organization;
mod dump_work;
mod error;
mod fields;
mod folder;
mod generate;
mod issues;
mod issues_hands;
mod issues_plan;
mod issues_setup;
mod issues_states;
mod issues_text;
mod issues_time;
mod issues_words;
mod names;
mod organization;
mod own_workflow;
mod password;
mod people_plan;
mod random;
mod relations;
#[cfg(test)]
mod tests;
mod together;
mod together_files;

pub use bb_dev_words::sample::{Layout, OrganizationNames, ProjectNames, SubProjectNames};
pub use clock::SampleClock;
pub use dump::Dump;
pub use error::{SampleError, Step};
pub use folder::Output;
pub use generate::{
    DEFAULT_SEED, LATER_SECONDS, Later, LeftBehind, MadePerson, NothingMore, Options, STEP_SECONDS,
    Sample, Target, World, create,
};
pub use issues::Everything;
pub use names::NameList;
pub use organization::Organization;
pub use people_plan::{
    ACCESS_KEYS, EMAIL_DOMAIN, PEOPLE, ROLE_MIX, Region, SamplePerson, TIME_ZONES, sample_people,
};
