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
mod create;
mod moving;
mod read;
mod shown;
mod store;
mod types;

use std::sync::{Arc, Mutex};

use bb_core::ErrorCode;
use bb_core::id::ProjectId;
use bb_policy::Tree;

use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, internal};

pub use change::change_project;
pub use create::create_project;
pub use moving::{move_project, preview_move};
pub use read::{project, projects};
pub(crate) use shown::Shown;
pub use store::{Found, ProjectStore};
pub use types::*;

impl<S: AuthStore> Ctx<'_, S> {
    /// The tree of the projects as it is stored now, read once for a request.
    pub(crate) async fn project_tree_now(&self) -> Result<Tree, AppError> {
        Ok(Tree::of(
            &self
                .store()
                .project_tree()
                .await
                .map_err(|error| internal::<S>(&error))?,
        ))
    }
}

/// The tree of the projects as it is stored in this transaction.
///
/// # Errors
/// The error of the store.
pub(crate) async fn tree_in<S: AuthStore>(
    transaction: &mut S::Transaction,
) -> Result<Tree, S::Error> {
    Ok(Tree::of(&S::project_tree_in(transaction).await?))
}

/// Inside the transaction that writes an issue (or a label on one) in a project: takes the lock of
/// the workflows shared, before any lock on the codes of a project (the one order of the locks,
/// see `LockClass`), and reads the projects from the top down to the project, which the offers and
/// the workflow are judged on.
///
/// # Errors
/// The error of the store.
pub(crate) async fn lock_chain_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    project: ProjectId,
) -> Result<Vec<ProjectId>, S::Error> {
    S::lock_workflows_shared(transaction).await?;
    Ok(tree_in::<S>(transaction)
        .await?
        .chain(project)
        .unwrap_or_default())
}

/// Every project below `project`, at any depth, as the tree is stored now (inside a change, after
/// the policy lock).
///
/// # Errors
/// The error of the store.
pub(crate) async fn sub_projects_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    project: ProjectId,
) -> Result<Vec<ProjectId>, S::Error> {
    let projects = S::project_tree_in(transaction).await?;
    Ok(Tree::of(&projects)
        .subtree(project)
        .into_iter()
        .filter(|inside| *inside != project)
        .collect())
}

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
        self.answer_with::<S>(error, otherwise, otherwise)
    }

    /// The reason the change gave for stopping itself, if it gave one.
    pub(crate) fn said(&self) -> Option<ErrorCode> {
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Like [`Self::answer`], for a change whose key used twice says another thing than its
    /// check that stopped itself without a reason: `stopped` for the first, `duplicate` for the
    /// second.
    pub(crate) fn answer_with<S: AuthStore>(
        &self,
        error: &S::Error,
        stopped: ErrorCode,
        duplicate: ErrorCode,
    ) -> AppError {
        let said = *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        change_failed::<S>(error, Stops::new(said.unwrap_or(stopped), duplicate))
    }
}
