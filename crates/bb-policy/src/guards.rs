//! Nobody locks the organization out, and nobody gives what they do not have (ADR-0025
//! decision 6, SEC-02, SEC-08).
//!
//! Two rules, as pure functions the management use cases call inside their change:
//! - **The last administrator:** a change that would leave no person who holds both *Manage
//!   roles* and *Decide who has which role* for the whole organization (through their own row or
//!   a group) is refused. Enforced by: the test `sec_08_nobody_can_remove_the_last_way_to_manage_roles`.
//! - **No escalation:** a person can add a permission to a role, or give a role, only when they
//!   hold every permission of it at that place themselves. Enforced by: the tests
//!   `sec_08_nobody_adds_a_permission_they_do_not_hold_to_a_role` and
//!   `sec_02_nobody_gives_a_role_with_more_than_they_hold_there`.

use std::collections::{BTreeMap, BTreeSet};

use bb_core::id::{GroupId, RoleId, UserId};

use crate::catalogue::{Counts, Permission};
use crate::evaluate::{Evaluation, Place, RoleData};

/// Why a change was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// It would leave nobody who can manage roles and decide who has which role for the whole
    /// organization.
    WouldLockOut,
    /// It adds or gives more than the person holds at that place.
    TooMuch,
    /// A group has a *No access* in a project the person may not decide in, so they may not lift it
    /// by taking people out of the group or deleting it.
    CannotLift,
    /// A role that is in use cannot be deleted without saying which role takes its place.
    InUse,
}

/// A person or a group that holds a role for the whole organization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holder {
    /// A person, by their own row.
    Person(UserId),
    /// A group; its members hold the role through it.
    Group(GroupId),
}

/// The rows for the whole organization that give a role, with the roles and the groups'
/// members: all that the last-administrator rule needs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OrganizationState {
    /// The roles.
    pub roles: Vec<RoleData>,
    /// The rows for the whole organization that give a role (not *No access*).
    pub rows: Vec<(Holder, RoleId)>,
    /// Who is in which group.
    pub members: Vec<(GroupId, UserId)>,
}

/// The people who hold both *Manage roles* and *Decide who has which role* for the whole
/// organization, through their own row or a group.
#[must_use]
pub fn role_managers(state: &OrganizationState) -> BTreeSet<UserId> {
    let roles: BTreeMap<RoleId, &RoleData> =
        state.roles.iter().map(|role| (role.id, role)).collect();
    let mut held: BTreeMap<UserId, BTreeSet<Permission>> = BTreeMap::new();
    for (holder, role) in &state.rows {
        let Some(role) = roles.get(role) else {
            continue;
        };
        match holder {
            Holder::Person(person) => {
                held.entry(*person)
                    .or_default()
                    .extend(role.permissions.iter().copied());
            }
            Holder::Group(group) => {
                for (member_of, person) in &state.members {
                    if member_of == group {
                        held.entry(*person)
                            .or_default()
                            .extend(role.permissions.iter().copied());
                    }
                }
            }
        }
    }
    held.into_iter()
        .filter(|(_, permissions)| {
            permissions.contains(&Permission::ManageRoles)
                && permissions.contains(&Permission::DecideRoles)
        })
        .map(|(person, _)| person)
        .collect()
}

/// Whether somebody can still manage roles for the whole organization.
#[must_use]
pub fn anybody_manages_roles(state: &OrganizationState) -> bool {
    !role_managers(state).is_empty()
}

/// A change may not leave nobody who manages roles when somebody did before. A state that had
/// nobody already (a fresh installation, where setup is open) is no reason to refuse.
///
/// # Errors
/// [`Refusal::WouldLockOut`] when the change takes the last way to manage roles away.
pub fn ensure_a_manager_remains(
    before: &OrganizationState,
    after: &OrganizationState,
) -> Result<(), Refusal> {
    if anybody_manages_roles(before) && !anybody_manages_roles(after) {
        Err(Refusal::WouldLockOut)
    } else {
        Ok(())
    }
}

/// The permissions of `wanted` that count at the place of the evaluation: a role given on a
/// project holds organization permissions that count nowhere there, so they are left out.
fn counting_here(
    wanted: &BTreeSet<Permission>,
    place: Place,
) -> impl Iterator<Item = Permission> + '_ {
    wanted.iter().copied().filter(move |permission| {
        place == Place::Organization || permission.counts() == Counts::Project
    })
}

/// A person may add permissions to a role, or give a role, only when they hold all of them at
/// that place themselves. `held` is the evaluation of the person at the place; `wanted` the
/// permissions to be added or the permissions of the role to be given.
///
/// # Errors
/// [`Refusal::TooMuch`] when one of them is missing from the person.
pub fn ensure_no_escalation(
    held: &Evaluation,
    wanted: &BTreeSet<Permission>,
) -> Result<(), Refusal> {
    if counting_here(wanted, held.place()).all(|permission| held.allows(permission)) {
        Ok(())
    } else {
        Err(Refusal::TooMuch)
    }
}
