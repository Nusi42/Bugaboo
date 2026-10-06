//! The actors of the matrix and what each of them may do, written down by hand from the tables of
//! architecture §7 (*Starting roles*) and ADR-0025 (decisions 3, 6 and 8), not read from the engine:
//! the matrix asks the real server and compares its answer with this table.
//!
//! The fixed list (WP-10): nobody signed in; a person without a role; each starting role for the
//! whole organization; a role on the parent project; a role on a sibling project only; *No access*
//! on the project; an access key with *See only*; an access key for another project. Two more
//! actors are the rule of ADR-0025 decision 6: a *No access* under a lead for the whole
//! organization, who keeps *Decide who has which role* without seeing anything.
//!
//! ```text
//! PARENT ─┬─ TARGET        every example that names a project names TARGET
//!         └─ SIBLING
//! ```

use std::collections::BTreeSet;

use bb_app::access::{Permission, StartingRole};

/// The projects of the matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Project {
    /// The parent of the other two.
    Parent,
    /// The project every example that names one is about.
    Target,
    /// Beside the target, inside the same parent.
    Sibling,
}

/// A place a permission is judged at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Where {
    /// The whole organization.
    Organization,
    /// The target project.
    Target,
}

/// Who asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Actor {
    /// Nobody is signed in.
    Nobody,
    /// A person who has no role at all.
    NoRole,
    /// A person with this starting role for the whole organization.
    Role(StartingRole),
    /// A *Project lead* on the parent project, with its sub-projects.
    LeadOnParent,
    /// A *Project lead* on the sibling project only.
    LeadOnSiblingOnly,
    /// A *Viewer* for the whole organization with *No access* on the target.
    NoAccessOnTarget,
    /// A *Project lead* for the whole organization with *No access* on the target: they keep
    /// *Decide who has which role* there and see nothing (ADR-0025, decision 6).
    NoAccessUnderLead,
    /// A person whose only role holds exactly this permission, for the whole organization (none:
    /// a person with no permission, for the routes that need none). Every route with a permission
    /// is run as the person with exactly that one, so a route cannot ask for more than it
    /// declares.
    Exactly(Option<Permission>),
    /// An *Administrator* who came with an access key that may only see (WP-09).
    KeySeeOnly,
    /// An *Administrator* who came with an access key for the sibling project (WP-09).
    KeyForAnotherProject,
}

/// The actors that run today. The two actors with an access key are listed in
/// [`Actor::WITH_A_KEY`]: they run when WP-09 is merged.
pub(super) fn actors() -> Vec<Actor> {
    let mut all = vec![Actor::Nobody, Actor::NoRole];
    all.extend(StartingRole::ALL.iter().copied().map(Actor::Role));
    all.extend([
        Actor::LeadOnParent,
        Actor::LeadOnSiblingOnly,
        Actor::NoAccessOnTarget,
        Actor::NoAccessUnderLead,
    ]);
    all
}

impl Actor {
    /// The actors that come with an access key. TODO(M2-F27): enable at the merge of WP-09 (access
    /// keys): make the key as the owner (`POST /api/v1/access-keys`: *See only*, and one for the
    /// sibling project), send it as `Authorization: Bearer …` (`Browser::set_header`), and add
    /// these two to [`actors`]. Their rows below are written already.
    pub(super) const WITH_A_KEY: [Self; 2] = [Self::KeySeeOnly, Self::KeyForAnotherProject];

    /// The actor that holds exactly what a route declares.
    pub(super) fn exactly(declared: &str) -> Self {
        Self::Exactly(Permission::from_code(declared))
    }

    /// What tells the actors apart, also the ones that share a name in the record.
    pub(super) fn key(self) -> String {
        match self {
            Self::Exactly(Some(permission)) => format!("exactly-{}", permission.code()),
            Self::Exactly(None) => "exactly-nothing".to_owned(),
            other => other.name(),
        }
    }

    /// The name in the record of what the matrix covered.
    pub(super) fn name(self) -> String {
        match self {
            Self::Exactly(_) => "exactly-the-permission".to_owned(),
            Self::Nobody => "nobody".to_owned(),
            Self::NoRole => "no-role".to_owned(),
            Self::Role(role) => format!("role-{}", role.key()),
            Self::LeadOnParent => "lead-on-parent".to_owned(),
            Self::LeadOnSiblingOnly => "lead-on-sibling-only".to_owned(),
            Self::NoAccessOnTarget => "no-access-on-target".to_owned(),
            Self::NoAccessUnderLead => "no-access-under-lead".to_owned(),
            Self::KeySeeOnly => "key-see-only".to_owned(),
            Self::KeyForAnotherProject => "key-for-another-project".to_owned(),
        }
    }

    /// Whether the actor is signed in at all.
    pub(super) fn signed_in(self) -> bool {
        self != Self::Nobody
    }

    /// Whether the actor comes with an access key, which never reaches the owner's own sign-in
    /// data (ADR-0025, decision 8).
    pub(super) fn has_a_key(self) -> bool {
        Self::WITH_A_KEY.contains(&self)
    }
}

/// The permissions that count only from a row for the whole organization, from the table of the
/// catalogue in architecture §7 (*Counts: organization*).
const ORGANIZATION_ONLY: [Permission; 7] = [
    Permission::MakeProjects,
    Permission::ManagePeople,
    Permission::ManageInvitations,
    Permission::ManageGroups,
    Permission::ManageRoles,
    Permission::ChangeOrganizationSettings,
    Permission::SeeStatus,
];

/// The permissions of the kind *See*, which an access key that may only see keeps (*Kind: See*).
const KIND_SEE: [Permission; 3] = [
    Permission::SeeProjects,
    Permission::CheckAccess,
    Permission::SeeStatus,
];

/// The permissions of a starting role, from the table of architecture §7.
fn role_permissions(role: StartingRole) -> BTreeSet<Permission> {
    use Permission::{
        ChangeOrganizationSettings, ChangeProjectSettings, CheckAccess, DecideRoles,
        DecideWhatRolesSee, MakeProjects, MakeSubProjects, ManageGroups, ManageInvitations,
        ManagePeople, ManageRoles, MoveProjects, SeeProjects, SeeStatus,
    };
    match role {
        StartingRole::Administrator => BTreeSet::from([
            SeeProjects,
            MakeProjects,
            MakeSubProjects,
            MoveProjects,
            ChangeProjectSettings,
            DecideRoles,
            DecideWhatRolesSee,
            CheckAccess,
            ManagePeople,
            ManageInvitations,
            ManageGroups,
            ManageRoles,
            ChangeOrganizationSettings,
            SeeStatus,
        ]),
        StartingRole::ProjectLead => BTreeSet::from([
            SeeProjects,
            MakeSubProjects,
            MoveProjects,
            ChangeProjectSettings,
            DecideRoles,
            DecideWhatRolesSee,
            CheckAccess,
        ]),
        StartingRole::TeamMember
        | StartingRole::Contributor
        | StartingRole::Viewer
        | StartingRole::Guest => BTreeSet::from([SeeProjects]),
    }
}

/// What an actor holds at a place: the permissions that count there, and whether they know the
/// project exists (they see it, or they keep *Decide who has which role* there).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Standing {
    pub(super) permissions: BTreeSet<Permission>,
    pub(super) knows: bool,
}

impl Standing {
    fn of(permissions: BTreeSet<Permission>) -> Self {
        let knows = permissions.contains(&Permission::SeeProjects)
            || permissions.contains(&Permission::DecideRoles);
        Self { permissions, knows }
    }

    fn nothing() -> Self {
        Self::of(BTreeSet::new())
    }

    /// Whether they see the project (and what is in it).
    pub(super) fn sees(&self) -> bool {
        self.permissions.contains(&Permission::SeeProjects)
    }
}

/// The permissions of a role for the whole organization at a place: at the organization, all of
/// them; in a project, those that count in projects.
fn given_everywhere(role: StartingRole, at: Where) -> BTreeSet<Permission> {
    let all = role_permissions(role);
    match at {
        Where::Organization => all,
        Where::Target => all
            .into_iter()
            .filter(|permission| !ORGANIZATION_ONLY.contains(permission))
            .collect(),
    }
}

/// What the actor holds at the place.
pub(super) fn standing(actor: Actor, at: Where) -> Standing {
    match actor {
        Actor::Nobody | Actor::NoRole | Actor::Exactly(None) => Standing::nothing(),
        // The one permission, for the whole organization: at a project it counts only when it
        // counts in projects.
        Actor::Exactly(Some(permission)) => match at {
            Where::Target if ORGANIZATION_ONLY.contains(&permission) => Standing::nothing(),
            _ => Standing::of(BTreeSet::from([permission])),
        },
        Actor::Role(role) => Standing::of(given_everywhere(role, at)),
        // A role on a project counts in the project and below, and gives nothing for the whole
        // organization (organization permissions count only from a row for the whole organization).
        Actor::LeadOnParent => match at {
            Where::Organization => Standing::nothing(),
            Where::Target => Standing::of(
                role_permissions(StartingRole::ProjectLead)
                    .into_iter()
                    .filter(|permission| !ORGANIZATION_ONLY.contains(permission))
                    .collect(),
            ),
        },
        Actor::LeadOnSiblingOnly => Standing::nothing(),
        Actor::NoAccessOnTarget => match at {
            Where::Organization => Standing::of(given_everywhere(StartingRole::Viewer, at)),
            Where::Target => Standing::nothing(),
        },
        // The row above the *No access* keeps *Decide who has which role* only.
        Actor::NoAccessUnderLead => match at {
            Where::Organization => Standing::of(given_everywhere(StartingRole::ProjectLead, at)),
            Where::Target => Standing::of(BTreeSet::from([Permission::DecideRoles])),
        },
        // An *Administrator* narrowed to the permissions of kind *See* (WP-09).
        Actor::KeySeeOnly => Standing::of(
            given_everywhere(StartingRole::Administrator, at)
                .into_iter()
                .filter(|permission| KIND_SEE.contains(permission))
                .collect(),
        ),
        // A key for the sibling project counts there and below, and nowhere else, not even for
        // the whole organization (ADR-0025, decision 8).
        Actor::KeyForAnotherProject => Standing::nothing(),
    }
}

/// Which projects the actor sees, in the order of the tree.
pub(super) fn sees(actor: Actor, project: Project) -> bool {
    match actor {
        Actor::Nobody | Actor::NoRole | Actor::KeyForAnotherProject => {
            // A key for the sibling project sees the sibling project (and below).
            actor == Actor::KeyForAnotherProject && project == Project::Sibling
        }
        Actor::Exactly(permission) => permission == Some(Permission::SeeProjects),
        Actor::Role(_) | Actor::LeadOnParent | Actor::KeySeeOnly => true,
        Actor::LeadOnSiblingOnly => project == Project::Sibling,
        Actor::NoAccessOnTarget | Actor::NoAccessUnderLead => project != Project::Target,
    }
}

/// The short names of the projects at the top of the tree as the actor sees it: the parent, or,
/// when the parent is hidden, the projects whose parent is.
pub(super) fn top_of_the_tree(actor: Actor) -> BTreeSet<&'static str> {
    if sees(actor, Project::Parent) {
        return BTreeSet::from([short_name(Project::Parent)]);
    }
    [Project::Target, Project::Sibling]
        .into_iter()
        .filter(|project| sees(actor, *project))
        .map(short_name)
        .collect()
}

/// Every project the actor sees.
pub(super) fn everything_seen(actor: Actor) -> BTreeSet<&'static str> {
    [Project::Parent, Project::Target, Project::Sibling]
        .into_iter()
        .filter(|project| sees(actor, *project))
        .map(short_name)
        .collect()
}

/// The short name of a project of the matrix.
pub(super) const fn short_name(project: Project) -> &'static str {
    match project {
        Project::Parent => "PARENT",
        Project::Target => "TARGET",
        Project::Sibling => "SIBLING",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_of_the_matrix_is_the_table_of_the_starting_roles() {
        for role in StartingRole::ALL {
            let in_the_code: BTreeSet<Permission> = role.permissions().into_iter().collect();
            assert_eq!(role_permissions(role), in_the_code, "{}", role.key());
        }
    }

    #[test]
    fn the_actors_are_named_once_and_the_two_with_a_key_wait_for_their_merge() {
        let names: BTreeSet<String> = actors().into_iter().map(Actor::name).collect();
        assert_eq!(names.len(), actors().len());
        for keyed in Actor::WITH_A_KEY {
            assert!(!actors().contains(&keyed));
            assert!(keyed.has_a_key() && !names.contains(&keyed.name()));
        }
    }

    #[test]
    fn a_role_for_the_whole_organization_counts_organization_permissions_only_at_the_organization()
    {
        let admin = Actor::Role(StartingRole::Administrator);
        assert!(
            standing(admin, Where::Organization)
                .permissions
                .contains(&Permission::ManageRoles)
        );
        assert!(
            !standing(admin, Where::Target)
                .permissions
                .contains(&Permission::ManageRoles)
        );
        assert!(
            standing(admin, Where::Target)
                .permissions
                .contains(&Permission::MoveProjects)
        );
    }

    #[test]
    fn what_each_actor_sees_of_the_tree() {
        assert_eq!(top_of_the_tree(Actor::NoRole), BTreeSet::new());
        assert_eq!(
            top_of_the_tree(Actor::LeadOnSiblingOnly),
            BTreeSet::from(["SIBLING"])
        );
        assert_eq!(
            top_of_the_tree(Actor::NoAccessOnTarget),
            BTreeSet::from(["PARENT"])
        );
        assert_eq!(
            everything_seen(Actor::NoAccessUnderLead),
            BTreeSet::from(["PARENT", "SIBLING"])
        );
        assert_eq!(
            everything_seen(Actor::KeyForAnotherProject),
            BTreeSet::from(["SIBLING"])
        );
        assert_eq!(
            top_of_the_tree(Actor::KeyForAnotherProject),
            BTreeSet::from(["SIBLING"])
        );
        assert_eq!(everything_seen(Actor::Role(StartingRole::Guest)).len(), 3);
    }

    #[test]
    fn a_lead_under_a_no_access_keeps_deciding_roles_and_sees_nothing() {
        let kept = standing(Actor::NoAccessUnderLead, Where::Target);
        assert!(kept.knows && !kept.sees());
        assert_eq!(kept.permissions, BTreeSet::from([Permission::DecideRoles]));
    }
}
