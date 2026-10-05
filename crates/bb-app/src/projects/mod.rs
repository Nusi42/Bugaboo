//! Projects (PRJ-01, PRJ-02, PRJ-03): making, seeing, renaming and moving them.
//!
//! The use cases are plain `async fn`s that take the context and an input (`types.rs`): *make a
//! project* (`make.rs`), *seeing projects* (`read.rs`), *change a project* (`change.rs`) and
//! *moving with a preview* (`moving.rs`). They depend on storage only through the port
//! [`ProjectStore`] (`store.rs`) and on the policy engine for every decision.
//!
//! Three rules run through all of them:
//! - **A project that a person may not see answers `resource.not_found`**, exactly like one that
//!   does not exist; a visible sub-project of a hidden project is shown at the top and names no
//!   parent (`shown.rs`).
//! - **No answer holds a count** of projects or of anything that could include hidden things.
//! - **A change that depends on what others hold is judged inside the change, after the policy
//!   lock**, like giving a role: the move re-reads the tree and who has which role and refuses a
//!   move after which somebody could take away a *No access* that applies to them.

mod change;
mod make;
mod moving;
mod read;
mod shown;
mod store;
mod types;

use std::sync::{Arc, Mutex};

use bb_core::ErrorCode;

use crate::auth::{AppError, AuthStore, internal};

pub use change::change_project;
pub use make::make_project;
pub use moving::{move_project, preview_move};
pub use read::{project, projects};
pub use store::{Found, ProjectStore};
pub use types::*;

/// Why a change stopped itself, for a change that can stop for more than one reason: the change
/// can only return the store's one "aborted" error, so it says the reason here first.
#[derive(Clone, Default)]
pub(crate) struct Why(Arc<Mutex<Option<ErrorCode>>>);

impl Why {
    /// Stops the change for `code`.
    pub(crate) fn stop<S: AuthStore>(&self, code: ErrorCode) -> S::Error {
        // A lock that a panic poisoned still holds a value that is whole.
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(code);
        S::aborted()
    }

    /// What to answer for a change that failed: the reason it stopped itself (`otherwise` when it
    /// did not say), `otherwise` for a key used twice, the rule of the policy engine that stopped
    /// it, or an internal failure for anything else.
    pub(crate) fn answer<S: AuthStore>(&self, error: &S::Error, otherwise: ErrorCode) -> AppError {
        if S::is_aborted(error) {
            let said = *self
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            AppError::Refused(said.unwrap_or(otherwise))
        } else if S::is_duplicate(error) {
            AppError::Refused(otherwise)
        } else if let Some(refusal) = S::as_refusal(error) {
            refusal.into()
        } else {
            internal::<S>(error)
        }
    }
}
