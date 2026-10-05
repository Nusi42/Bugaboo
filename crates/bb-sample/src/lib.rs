//! The sample organization (architecture §11): a fictional organization for development,
//! end-to-end tests and demos, made through the same use cases the API calls.
//!
//! A developer tool, not part of the product: nothing depends on this crate, and it is not linked
//! into `bugaboo` before *Try it out* (OPS-12). `cargo xtask dataset sample` runs its program
//! (`bb-sample`) as a process, so `xtask` depends on no product crate.
//!
//! - [`make`] makes the people: the first administrator with the setup code, everybody else as
//!   that administrator by invitation. Every change has its event and its security log entry.
//! - [`Later`] is the extension point for what comes after the people (groups, roles, projects,
//!   assignments: M2 WP-13).
//! - [`Sample::dump`] is the canonical dump that TEST-SAMPLE-DATA compares.
//! - The passwords are random for every run and are written only to `sign-in.txt` ([`Output`]).

#![forbid(unsafe_code)]

// The program (`src/main.rs`) is the only user of these, and Cargo gives every dependency to
// the library and the program alike (`unused_crate_dependencies`).
use clap as _;
use tokio as _;

mod clock;
mod dump;
mod error;
mod folder;
mod generate;
mod limits;
mod names;
mod password;
mod random;
mod roster;
#[cfg(test)]
mod tests;

pub use clock::SampleClock;
pub use dump::Dump;
pub use error::{SampleError, Step};
pub use folder::{DATA_FOLDER, DUMP_FILE, MARKER_FILE, Output, SIGN_IN_FILE};
pub use generate::{
    DEFAULT_SEED, Later, MadePerson, NothingMore, Options, STEP_SECONDS, Sample, Target, World,
    make,
};
pub use names::NameList;
pub use roster::{EMAIL_DOMAIN, PEOPLE, ROLE_MIX, Role, SamplePerson, roster};

/// The environment variable that holds the address of the empty PostgreSQL database of the
/// program (an address with a password does not belong on a command line, M1-F62).
pub const DATABASE_URL_VARIABLE: &str = "BB_SAMPLE_DATABASE_URL";
