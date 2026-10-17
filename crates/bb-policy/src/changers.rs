//! Which roles may change a field at a place (M3-F53, ADR-0031): the one policy question that the
//! sentence *"Only team members and administrators can change Status"* and the *Who can do it* of
//! the workflow editor both ask.
//!
//! A role may change a field when it sees the project and the issues, holds *Change issues* (or
//! only *Change own issues*), and its level for the field at the place, for the issue type, is
//! *Can change*. The levels are read the way the engine reads them (the nearest place wins, and an
//! issue type wins over all types), so this answers what the mask of a person who holds the role
//! would say.

use bb_core::access::FieldLevel;
use bb_core::fields::FieldKey;
use bb_core::id::{IssueTypeId, ProjectId, RoleId};

use crate::catalogue::Permission;
use crate::evaluate::RoleData;
use crate::levels::{LevelRules, LevelSubject as _};

/// A role that may change a field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Changer {
    /// The role.
    pub role: RoleId,
    /// Whether it may change the field only in the issues its holders reported (it holds *Change
    /// own issues* and not *Change issues*).
    pub own_only: bool,
}

/// The roles among `roles` that may change `field` in an issue of this type, at a place whose
/// projects from the top down are `chain`, in the order of `roles`.
#[must_use]
pub fn roles_that_change(
    roles: &[RoleData],
    levels: &LevelRules,
    chain: &[ProjectId],
    field: FieldKey,
    issue_type: Option<IssueTypeId>,
) -> Vec<Changer> {
    roles
        .iter()
        .filter_map(|role| {
            let reads = |permission: Permission| role.permissions.contains(&permission);
            if !(reads(Permission::SeeProjects) && reads(Permission::SeeIssues)) {
                return None;
            }
            let level = FieldKey::level_for(levels, chain, role.id, field, issue_type)
                .map_or(FieldLevel::Hidden, |(_, _, level)| level);
            if level < FieldLevel::CanChange {
                return None;
            }
            if reads(Permission::ChangeIssues) {
                Some(Changer {
                    role: role.id,
                    own_only: false,
                })
            } else if reads(Permission::ChangeOwnIssues) {
                Some(Changer {
                    role: role.id,
                    own_only: true,
                })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bb_core::fields::Field;
    use bb_core::id::SequentialIds;

    use super::*;
    use crate::levels::LevelRule;

    fn role(source: &mut SequentialIds, permissions: &[Permission]) -> RoleData {
        RoleData {
            id: RoleId::generate(source).unwrap(),
            name: String::new(),
            position: 0,
            permissions: permissions.iter().copied().collect(),
            two_step: false,
        }
    }

    fn set(rules: &mut LevelRules, role: &RoleData, level: FieldLevel) {
        rules.add::<FieldKey>(LevelRule {
            role: role.id,
            subject: FieldKey::BuiltIn(Field::Status),
            project: None,
            level,
        });
    }

    #[test]
    fn m3_f53_the_roles_that_may_change_a_field_are_the_ones_the_mask_would_let() {
        let mut source = SequentialIds::default();
        let reads = [Permission::SeeProjects, Permission::SeeIssues];
        let with = |extra: Permission| [&reads[..], &[extra]].concat();
        let (team, own, viewer, blind) = (
            role(&mut source, &with(Permission::ChangeIssues)),
            role(&mut source, &with(Permission::ChangeOwnIssues)),
            role(&mut source, &reads),
            role(&mut source, &with(Permission::ChangeIssues)),
        );
        let mut levels = LevelRules::default();
        set(&mut levels, &team, FieldLevel::CanChange);
        set(&mut levels, &own, FieldLevel::CanChange);
        set(&mut levels, &viewer, FieldLevel::CanChange);
        set(&mut levels, &blind, FieldLevel::CanSee);
        let all = [team.clone(), own.clone(), viewer, blind];
        let found = roles_that_change(&all, &levels, &[], FieldKey::BuiltIn(Field::Status), None);
        assert_eq!(
            found,
            [
                Changer {
                    role: team.id,
                    own_only: false
                },
                Changer {
                    role: own.id,
                    own_only: true
                },
            ],
            "a viewer holds no right to change, a role that only sees the field may not"
        );
    }
}
