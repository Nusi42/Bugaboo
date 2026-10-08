//! The authorization engine: permission catalogue, field masks, visibility and `Authorized<T>` (ADR-0004).
//!
//! `bb-policy` makes every permission decision and nothing else can (ADR-0025): it holds the
//! permission catalogue ([`Permission`]), the starting roles ([`StartingRole`]), the evaluation of
//! roles, groups and assignments on the project tree with *No access* ([`evaluate`]), the
//! narrowing for access keys ([`Narrowing`]), the field mask and [`Authorized`], `PersonRef`
//! ([`PersonRef`]), the rules that keep the organization from locking itself out ([`guards`]), the reasons of
//! *Check access* ([`AccessReport`]) and what a move of a project changes ([`preview_move`]). It does no I/O: the rows come in as plain
//! data through a port of `bb-app`, and an evaluation lives for one request.

#![forbid(unsafe_code)]

mod catalogue;
mod evaluate;
mod explain;
pub mod guards;
mod mask;
mod moves;
mod narrowing;
mod people;
mod starting;
mod tree;

pub use catalogue::{Counts, Kind, NOT_FOR_KEYS, Permission, Topic};
pub use evaluate::{
    Evaluation, FieldRule, HeldRole, Inputs, LevelSource, PeopleRule, Place, RoleData, RoleLevel,
    Row, RowStatus, RowTrace, Who, evaluate, nearest_rule,
};
pub use explain::{AccessReport, LevelDecision, PermissionDecision, Reason, ReasonRow};
pub use mask::{Authorized, FieldMask, Masked, drop_hidden, mask_member};
pub use moves::{Access, Change, Lift, Preview, preview_move};
pub use narrowing::{Choice, Narrowing};
pub use people::{
    FormerUser, PeopleContext, PersonRef, Target, former_user_label, resolve, taken_in,
};
pub use starting::StartingRole;
pub use tree::{Policy, Tree};

// Used by the integration tests in tests/engine only (`unused_crate_dependencies`).
#[cfg(test)]
use jiff as _;
#[cfg(test)]
use proptest as _;
#[cfg(test)]
use trybuild as _;
