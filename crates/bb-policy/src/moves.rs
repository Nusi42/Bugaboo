//! What a move does to who may do what (PRJ-03, M2-F23): the difference of two evaluations, and
//! the rule that a move lifts no *No access*.
//!
//! Moving project X puts X and everything in it under another parent. Nothing in the rows of
//! *Who has which role* changes, but the rows on the parents above X do: those of the old
//! parents stop reaching X and those of the new parents start to. So only people who have a row
//! (or a group that has one) on an old or a new parent, or who hold a role that sets a field level
//! or a people level on one of them (which the projects below inherit), can be affected, and for them the preview
//! evaluates every project of X's subtree in the tree as it is and as it would be, and keeps what
//! differs. The preview is the difference of those evaluations and nothing else, so it can never
//! say something the engine does not.
//!
//! ***No access* is never lifted by a move** (ADR-0025, decision 6, M2-F23). *Decide who has
//! which role* on a project is kept from a row strictly above a *No access*, so that nobody is
//! locked out by a project below them. Moving X under a project where somebody has a role with
//! its sub-projects gives them such a row above every *No access* inside X, and with it the right
//! to take that *No access* away, also when it is their own. [`Preview::lifts`] lists every such
//! person and project; a move with a lift is refused.

use std::collections::{BTreeMap, BTreeSet};

use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::{ProjectId, UserId};
use bb_core::people_kinds::PeopleKind;

use crate::catalogue::Permission;
use crate::evaluate::{Evaluation, Place};
use crate::narrowing::Narrowing;
use crate::tree::{Policy, Tree};

/// Why a move is refused before it is previewed or done.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveRefusal {
    /// The project, or the new parent, is one the person may not know about (or there is none).
    /// The two answer alike.
    NotFound,
    /// The person knows the places but may not move there or from there.
    Forbidden,
    /// The new parent is the project itself or something inside it.
    IntoItself,
    /// The project is inside that parent already.
    SameParent,
    /// The project, with what is inside it, would go deeper than a tree may.
    TooDeep,
}

/// A move being asked for by a person (or by the access key they came with).
#[derive(Clone, Copy, Debug)]
pub struct MoveRequest<'a> {
    /// The person who asks.
    pub person: UserId,
    /// The access key they came with, if any: what they may do is narrowed to it.
    pub key: Option<&'a Narrowing>,
    /// The project to move.
    pub project: ProjectId,
    /// The new parent (the top level for `None`).
    pub to: Option<ProjectId>,
}

/// Whether the person may ask for this move at all: the one set of rules the preview and the move
/// both stand on, judged on the tree and on who has which role as they are given. They see the
/// project; they may move projects in the new and in the old parent (a move to or from the top
/// level takes the permission for the whole organization, and to the top level also *Make
/// projects*); the project does not go inside itself, nor where it is already, nor deeper than a
/// tree may go.
///
/// # Errors
/// What is refused, as a [`MoveRefusal`]. A project or a parent the person may not know about is
/// refused like one that does not exist.
pub fn check_move(
    policy: &Policy,
    tree: &Tree,
    request: &MoveRequest<'_>,
) -> Result<(), MoveRefusal> {
    let MoveRequest {
        person,
        key,
        project,
        to,
    } = *request;
    let held = |place: Place| policy.evaluate_as(person, key, tree, place);
    if !held(Place::Project(project)).is_some_and(|here| here.sees_project()) {
        return Err(MoveRefusal::NotFound);
    }
    // The new parent: one the person may not know about answers like one that is not there.
    let new_place = Place::from(to);
    let new_parent = held(new_place).ok_or(MoveRefusal::NotFound)?;
    if !new_parent.allows(Permission::MoveProjects) {
        return Err(
            if matches!(new_place, Place::Project(_)) && !new_parent.knows_project() {
                MoveRefusal::NotFound
            } else {
                MoveRefusal::Forbidden
            },
        );
    }
    // The top level is the organization: a project that goes there is made there.
    if to.is_none()
        && !held(Place::Organization).is_some_and(|whole| whole.allows(Permission::MakeProjects))
    {
        return Err(MoveRefusal::Forbidden);
    }
    // The old parent: a person who sees the project but not what holds it is told they may not,
    // as for a parent they see; nothing says the parent is hidden.
    let old_parent = tree.parent(project).ok_or(MoveRefusal::NotFound)?;
    let old = held(Place::from(old_parent)).ok_or(MoveRefusal::NotFound)?;
    if !old.allows(Permission::MoveProjects) {
        return Err(MoveRefusal::Forbidden);
    }
    if to.is_some_and(|target| tree.is_within(target, project)) {
        return Err(MoveRefusal::IntoItself);
    }
    if old_parent == to {
        return Err(MoveRefusal::SameParent);
    }
    if tree.moved(project, to).too_deep_below(project) {
        return Err(MoveRefusal::TooDeep);
    }
    Ok(())
}

/// What a person has at one project, as far as a move can change it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Access {
    /// Whether the person sees the project.
    pub sees: bool,
    /// The names of the roles that count, in the order of roles.
    pub roles: Vec<String>,
    /// What the person may do there.
    pub permissions: BTreeSet<Permission>,
    /// The level of every field.
    pub fields: BTreeMap<Field, FieldLevel>,
    /// How the person sees every kind of people information.
    pub people: BTreeMap<PeopleKind, PeopleLevel>,
    /// Whether a *No access* applies to the person there (own or of one of their groups).
    pub held_back: bool,
}

impl Access {
    /// Whether what the person may do or see differs. A *No access* that applies on one side only
    /// is no difference when it holds nothing back: it is told with the change, not as one.
    #[must_use]
    pub fn differs_from(&self, other: &Self) -> bool {
        self.sees != other.sees
            || self.roles != other.roles
            || self.permissions != other.permissions
            || self.fields != other.fields
            || self.people != other.people
    }

    /// What the evaluation says; nothing at all when there is none (the project is not there).
    fn of(evaluation: Option<&Evaluation>) -> Self {
        let Some(found) = evaluation else {
            return Self {
                sees: false,
                roles: Vec::new(),
                permissions: BTreeSet::new(),
                fields: BTreeMap::new(),
                people: BTreeMap::new(),
                held_back: false,
            };
        };
        Self {
            sees: found.sees_project(),
            roles: found.roles().iter().map(|role| role.name.clone()).collect(),
            permissions: found.permissions().clone(),
            fields: Field::ALL
                .iter()
                .map(|field| (*field, found.field_level(*field)))
                .collect(),
            people: PeopleKind::ALL
                .iter()
                .map(|kind| (*kind, found.kind_level(*kind)))
                .collect(),
            held_back: found.trace().iter().any(|row| row.row.role.is_none()),
        }
    }
}

/// What a move changes for one person at one project of the moved subtree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    /// The person.
    pub person: UserId,
    /// The project, in the moved subtree.
    pub project: ProjectId,
    /// What they have now.
    pub before: Access,
    /// What they would have after the move.
    pub after: Access,
}

/// A person who, after the move, could take away a *No access* that applies to them, and the
/// project that *No access* is on. Before the move they could not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lift {
    /// The person.
    pub person: UserId,
    /// The project the *No access* is on.
    pub project: ProjectId,
}

/// What a move would do: who gains or loses access or changes role, and whose *No access* it
/// would let them lift.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Preview {
    /// Every difference, by person and then by project.
    pub changes: Vec<Change>,
    /// Every person who could lift a *No access* after the move and could not before.
    pub lifts: Vec<Lift>,
}

/// The projects above `project` and the parent it moves to, in the tree as it is and as it would
/// be: the places whose rows decide what a move changes.
fn parents_that_matter(
    before: &Tree,
    after: &Tree,
    project: ProjectId,
    new_parent: Option<ProjectId>,
) -> BTreeSet<ProjectId> {
    let mut places = BTreeSet::new();
    for chain in [
        before.chain(project),
        new_parent.and_then(|parent| before.chain(parent)),
        after.chain(project),
    ]
    .into_iter()
    .flatten()
    {
        places.extend(chain);
    }
    places
}

/// What moving `project` inside `new_parent` (the top level for `None`) would change.
///
/// `before` is the tree as it is. A project that is not in the tree, or a move that puts a project
/// inside itself, changes nothing here: the caller refuses those first.
#[must_use]
pub fn preview_move(
    policy: &Policy,
    before: &Tree,
    project: ProjectId,
    new_parent: Option<ProjectId>,
) -> Preview {
    let after = before.moved(project, new_parent);
    let mattering = parents_that_matter(before, &after, project, new_parent);
    let subtree = before.subtree(project);
    let mut preview = Preview::default();
    for person in policy.people_affected_by(&mattering) {
        let rows = policy.rows_of(person);
        let sees = |tree: &Tree, place: ProjectId| {
            policy.evaluate_with(person, &rows, tree, Place::Project(place))
        };
        for place in &subtree {
            let (old, new) = (sees(before, *place), sees(&after, *place));
            let (was, will_be) = (Access::of(old.as_ref()), Access::of(new.as_ref()));
            if was.differs_from(&will_be) {
                preview.changes.push(Change {
                    person,
                    project: *place,
                    before: was,
                    after: will_be,
                });
            }
        }
        // A *No access* can only be lifted from the project it is on, by somebody who decides who
        // has which role there; so the places to look at are those of the subtree with such a
        // row for the person, and the right they have there after and before.
        let on_subtree: BTreeSet<ProjectId> = rows
            .iter()
            .filter(|row| row.role.is_none())
            .filter_map(|row| row.project)
            .filter(|row| subtree.contains(row))
            .collect();
        for place in on_subtree {
            let decides = |tree: &Tree| {
                sees(tree, place).is_some_and(|found| found.allows(Permission::DecideRoles))
            };
            if decides(&after) && !decides(before) {
                preview.lifts.push(Lift {
                    person,
                    project: place,
                });
            }
        }
    }
    preview.lifts.sort();
    preview
}
