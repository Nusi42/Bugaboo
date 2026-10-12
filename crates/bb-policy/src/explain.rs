//! *Check access* (SEC-03, ADR-0025 decision 12): the decision for a person, a place and a
//! permission, a field or a kind of people information, with its **reasons** as data. The browser
//! turns the reasons into sentences from message keys (*"Ana is in the group Backend, which has
//! the role Team member in ACME"*); this crate never writes a sentence.
//!
//! The reasons come from the same pass as the decision (the rows of the evaluation), so the
//! answer and the reasons cannot disagree. A reason names IDs only; whether a project may be
//! named to the person who checks is the use case's business (a project they cannot see is
//! *a project you can't see*).

use bb_core::access::AppliesTo;
use bb_core::fields::FieldKey;
use bb_core::id::{AssignmentId, ProjectId, RoleId};
use bb_core::people_kinds::PeopleKind;

use crate::catalogue::Permission;
use crate::evaluate::{Evaluation, Place, Row, RowStatus, RowTrace, Who};
use crate::levels::{LevelSubject, RoleLevel};

/// One row of *Who has which role* that played a part, as data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReasonRow {
    /// The row.
    pub assignment: AssignmentId,
    /// The person themselves or a group of theirs.
    pub who: Who,
    /// The project the row is on, or `None` for the whole organization.
    pub project: Option<ProjectId>,
    /// Where it counts.
    pub applies_to: AppliesTo,
    /// Whether the row is above the place and reaches it by inheritance.
    pub inherited: bool,
}

impl ReasonRow {
    fn of(trace: &RowTrace) -> Self {
        let Row {
            id,
            who,
            project,
            applies_to,
            ..
        } = trace.row;
        Self {
            assignment: id,
            who,
            project,
            applies_to,
            inherited: trace.inherited,
        }
    }
}

/// Why a permission is given or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// The role of a row gives it.
    GivenByRole {
        /// The row.
        row: ReasonRow,
        /// The role.
        role: RoleId,
    },
    /// *Decide who has which role* is kept from a row that a *No access* removed, so nobody is
    /// locked out (ADR-0025, decision 6). Only from a row strictly above the *No access*.
    KeptFromRemovedRow {
        /// The row that was removed.
        row: ReasonRow,
        /// Its role.
        role: RoleId,
        /// The *No access* row that decided.
        no_access: ReasonRow,
    },
    /// The role of a row has it, but a *No access* row of equal or higher rank removed the role.
    RemovedByNoAccess {
        /// The row that was removed.
        row: ReasonRow,
        /// Its role.
        role: RoleId,
        /// The *No access* row that decided.
        no_access: ReasonRow,
    },
    /// The role of a row has it, but it counts only for the whole organization and the place is
    /// a project (the role page says so).
    CountsOnlyForTheOrganization {
        /// The row.
        row: ReasonRow,
        /// The role.
        role: RoleId,
    },
    /// The access key does not keep this permission.
    NarrowedByKey,
    /// The access key does not count at this place: it is outside the key's projects.
    OutsideKeyProjects,
    /// No role of the person gives it here.
    NoRoleGivesIt,
}

/// The decision for one permission with its reasons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionDecision {
    /// The permission.
    pub permission: Permission,
    /// Whether it counts at the place. Equal to [`Evaluation::allows`].
    pub allowed: bool,
    /// Why.
    pub reasons: Vec<Reason>,
}

/// What one role said about a field or a kind, and the result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelDecision<K: LevelSubject> {
    /// The field or the kind.
    pub subject: K,
    /// The level the person has. Equal to the evaluation's own answer.
    pub level: K::Level,
    /// The level each role that counts here says, and where it was set.
    pub roles: Vec<RoleLevel<K::Level>>,
    /// An access key lowered the level to *Can see*.
    pub capped_by_key: bool,
}

/// The whole answer of *Check access* for one person at one place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessReport {
    /// The place.
    pub place: Place,
    /// Every permission with *yes* or *no*.
    pub permissions: Vec<PermissionDecision>,
    /// Every field with its level.
    pub fields: Vec<LevelDecision<FieldKey>>,
    /// Every kind of people information with its level.
    pub kinds: Vec<LevelDecision<PeopleKind>>,
}

impl Evaluation {
    /// The decision for one permission, with the reasons.
    #[must_use]
    pub fn explain(&self, permission: Permission) -> PermissionDecision {
        let allowed = self.allows(permission);
        let mut reasons = Vec::new();
        let by_id = |id: AssignmentId| {
            self.trace
                .iter()
                .find(|trace| trace.row.id == id)
                .map(ReasonRow::of)
        };
        for trace in &self.trace {
            let Some(role) = trace.row.role else { continue };
            let has_it = self
                .all_role_permissions
                .get(&role)
                .is_some_and(|permissions| permissions.contains(&permission));
            if !has_it {
                continue;
            }
            let row = ReasonRow::of(trace);
            match trace.status {
                RowStatus::Counts => {
                    if self.counts_in_this_place(permission) {
                        reasons.push(Reason::GivenByRole { row, role });
                    } else {
                        reasons.push(Reason::CountsOnlyForTheOrganization { row, role });
                    }
                }
                RowStatus::RemovedBy(decides) | RowStatus::RemovedFromAbove(decides) => {
                    if let Some(no_access) = by_id(decides) {
                        // The engine decided once whether Decide who has which role is kept.
                        let kept = permission == Permission::DecideRoles
                            && matches!(trace.status, RowStatus::RemovedFromAbove(_));
                        reasons.push(if kept {
                            Reason::KeptFromRemovedRow {
                                row,
                                role,
                                no_access,
                            }
                        } else {
                            Reason::RemovedByNoAccess {
                                row,
                                role,
                                no_access,
                            }
                        });
                    }
                }
                RowStatus::NoAccessDecides | RowStatus::NoAccessCovered => {}
            }
        }
        if let Some(narrowed) = &self.narrowing {
            if narrowed.outside_projects {
                reasons.push(Reason::OutsideKeyProjects);
            } else if narrowed.removed.contains(&permission) {
                reasons.push(Reason::NarrowedByKey);
            }
        }
        if reasons.is_empty() {
            reasons.push(Reason::NoRoleGivesIt);
        }
        PermissionDecision {
            permission,
            allowed,
            reasons,
        }
    }

    /// The level of a field or of a kind of people information with what each role said.
    #[must_use]
    pub fn explain_level<K: LevelSubject>(&self, subject: K) -> LevelDecision<K> {
        let roles = K::levels_in(self).roles(subject);
        let from_roles = roles
            .iter()
            .map(|role| role.level)
            .max()
            .unwrap_or(K::CLOSED);
        let level = self.level(subject);
        LevelDecision {
            subject,
            level,
            roles,
            capped_by_key: level < from_roles,
        }
    }

    /// Everything *Check access* shows for the place: at a project only the permissions that
    /// count in a project (the others count nowhere there, architecture §7), at the whole
    /// organization all of them.
    #[must_use]
    pub fn report(&self) -> AccessReport {
        AccessReport {
            place: self.place,
            permissions: Permission::ALL
                .iter()
                .filter(|permission| self.counts_in_this_place(**permission))
                .map(|permission| self.explain(*permission))
                .collect(),
            fields: self
                .fields
                .all()
                .keys()
                .map(|field| self.explain_level(*field))
                .collect(),
            kinds: PeopleKind::ALL
                .iter()
                .map(|kind| self.explain_level(*kind))
                .collect(),
        }
    }
}
