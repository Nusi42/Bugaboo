//! The port of the projects use cases: what they need from storage beyond the policy port.
//!
//! Like [`PolicyStore`], reads take `&self` and **writes** are associated functions that run
//! inside `record_change`, so a change and its domain event are committed together
//! (ARCH-EVENTS). The reads that load the whole tree and the rows of *Who has which role* exist
//! so that a list of the projects a person sees, and the preview of a move, are made from the
//! same engine as every other decision (`bb_policy::Policy`), with one query for each and not one
//! per project.

use std::future::Future;

use bb_core::id::{ProjectId, UserId};
use bb_core::project::{Project, ProjectAlias, ShortName};
use bb_policy::Policy;

use crate::access::PolicyStore;
use crate::store_macros::{reads, writes};

/// A project found by an address: the project, and whether the address named an old short name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// The project.
    pub project: Project,
    /// The short name was an old one of the project.
    pub old_name: bool,
}

/// What the projects use cases need from storage.
pub trait ProjectStore: PolicyStore + 'static {
    reads! {
        /// Every project, in the order of their IDs: the tree.
        fn project_tree() -> Vec<Project>;
        /// What the engine needs to decide for one person anywhere: their rows and their groups'
        /// rows and the roles those name. The levels of fields and people are left out (a list
        /// of projects only needs permissions).
        fn policy_of(person: UserId) -> Policy;
        /// Everything the engine reads, for everybody: the rows, the groups, the roles and the
        /// levels they set. What the preview of a move is made from.
        fn policy_all() -> Policy;
        /// The project that has this short name, or had it once.
        fn project_by_address(name: &ShortName) -> Option<Found>;
    }

    writes! {
        /// The same as [`ProjectStore::project_tree`], inside the change.
        fn project_tree_in() -> Vec<Project>;
        /// Everything the engine reads, for everybody, inside the change: the rows, the groups,
        /// the roles and the levels they set. What the preview of a move and the rule about
        /// *No access* are judged on, after the policy lock.
        fn policy_in() -> Policy;
        /// The same as [`PolicyStore::project`], inside the change.
        fn project_in(id: ProjectId) -> Option<Project>;
        /// The project that has this short name now or had it once, inside the change.
        fn short_name_owner_in(name: &ShortName) -> Option<ProjectId>;
        /// Changes the name and short name of a project, counts its version up, and says whether
        /// the project was still at `version`.
        fn update_project(
            id: ProjectId,
            name: &str,
            short_name: &ShortName,
            version: i64
        ) -> bool;
        /// Puts a project inside another one (the top level for `None`), counts its version up,
        /// and says whether the project was still at `version`.
        fn move_project(id: ProjectId, parent: Option<ProjectId>, version: i64) -> bool;
        /// Keeps an old short name with its project.
        fn insert_project_alias(alias: &ProjectAlias) -> ();
        /// Lets go of an old short name: its project took it back.
        fn delete_project_alias(name: &ShortName) -> ();
    }
}
