//! What the role use cases take and give back: inputs that change something are named after what
//! they do; inputs that only read end in `Query` (ARCH-EVENTS). Everything that is given back is
//! wrapped in `Authorized<T>` by the use case.

use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, GroupId, ProjectId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Masked, Permission, PersonRef};
use jiff::Timestamp;
use serde::Serialize;

use crate::access::ProjectLabel;
use crate::people::GroupView;

/// Whom a row of *Who has which role* is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subject {
    /// A person.
    Person(UserId),
    /// A group.
    Group(GroupId),
}

/// What a row gives: a role, or *No access*.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Given {
    /// A role.
    Role(RoleId),
    /// *No access*: the person or group sees and can do nothing in the project and its
    /// sub-projects, whatever role they have from higher up.
    NoAccess,
}

/// Make a role.
pub struct MakeRole {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What the role is called.
    pub name: String,
    /// What it allows.
    pub permissions: Vec<Permission>,
    /// Whether everybody with it needs two-step sign-in.
    pub two_step: bool,
}

/// Change a role: what is `Some` is changed.
pub struct ChangeRole {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The role.
    pub role: RoleId,
    /// A new name.
    pub name: Option<String>,
    /// The permissions from now on.
    pub permissions: Option<Vec<Permission>>,
    /// The two-step switch from now on.
    pub two_step: Option<bool>,
}

/// Delete a role, with the rows that give it.
pub struct DeleteRole {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The role.
    pub role: RoleId,
}

/// Give a starting role back the permissions and levels it had at the start (*Reset to the
/// starting permissions*).
pub struct ResetRole {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The role.
    pub role: RoleId,
}

/// Give a role, or *No access*, to a person or a group.
pub struct AssignRole {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Whom.
    pub subject: Subject,
    /// What.
    pub given: Given,
    /// The project the row is on, or `None` for the whole organization.
    pub project: Option<ProjectId>,
    /// Where it counts.
    pub applies_to: AppliesTo,
}

/// Take a row of *Who has which role* away.
pub struct RemoveAssignment {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The row.
    pub assignment: AssignmentId,
}

/// Set (`Some`) or take away (`None`) the level of a field for a role: for a project, or the
/// role's own for the whole organization (which cannot be taken away).
pub struct SetFieldLevel {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The role.
    pub role: RoleId,
    /// The project, or `None` for the role's own level.
    pub project: Option<ProjectId>,
    /// The field.
    pub field: Field,
    /// The level.
    pub level: Option<FieldLevel>,
}

/// Set (`Some`) or take away (`None`) how a role sees one kind of people information.
pub struct SetPeopleLevel {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The role.
    pub role: RoleId,
    /// The project, or `None` for the role's own level.
    pub project: Option<ProjectId>,
    /// The kind of people information.
    pub kind: PeopleKind,
    /// The level.
    pub level: Option<PeopleLevel>,
}

/// List the roles, for the whole organization or for choosing one on a project.
pub struct RolesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project the person chooses a role for, or `None` for *Organization settings*.
    pub place: Option<ProjectId>,
}

/// List the levels a role has set.
pub struct LevelsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The role.
    pub role: RoleId,
    /// The project, or `None` for the role's own levels.
    pub project: Option<ProjectId>,
}

/// List *Who has which role* for the whole organization or one project, with the rows that come
/// from above.
pub struct AssignmentsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, or `None` for the whole organization.
    pub project: Option<ProjectId>,
}

/// *Check access*: what a person can do in a project (or the whole organization), and why.
pub struct CheckAccessQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The person to check.
    pub person: UserId,
    /// The project, or `None` for the whole organization.
    pub place: Option<ProjectId>,
}

/// A role as it is shown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RoleView {
    /// The ID of the role.
    pub id: RoleId,
    /// What it is called.
    pub name: String,
    /// The starting role it is a copy of (`administrator`, …), if it is one.
    pub starting: Option<String>,
    /// The place in the order of roles.
    pub position: i64,
    /// Whether everybody with it needs two-step sign-in.
    pub two_step: bool,
    /// What it allows, in the order of the catalogue.
    pub permissions: Vec<Permission>,
}

impl Masked for RoleView {}

/// The roles, in the order of roles.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RoleList {
    /// The roles.
    pub roles: Vec<RoleView>,
}

impl Masked for RoleList {}

/// A role named in another answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RoleRef {
    /// The ID of the role.
    pub id: RoleId,
    /// Its name; empty when the role was deleted meanwhile.
    pub name: String,
}

/// The level a role has set for one field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct FieldLevelView {
    /// The field.
    pub field: Field,
    /// Its level.
    pub level: FieldLevel,
}

/// The level a role has set for one kind of people information.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PeopleLevelView {
    /// The kind.
    pub kind: PeopleKind,
    /// Its level.
    pub level: PeopleLevel,
}

/// The levels a role has set in one place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Levels {
    /// The fields.
    pub fields: Vec<FieldLevelView>,
    /// The kinds of people information.
    pub people: Vec<PeopleLevelView>,
}

impl Masked for Levels {}

/// Whom a row is for, as the viewer may see it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WhoView {
    /// A person, shown as the viewer may see them.
    Person {
        /// The person.
        person: PersonRef,
    },
    /// A group.
    Group {
        /// The group.
        group: GroupView,
    },
}

/// One row of *Who has which role*.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AssignmentView {
    /// The ID of the row.
    pub id: AssignmentId,
    /// Whom it is for.
    pub who: WhoView,
    /// The role, or `None` for *No access*.
    pub role: Option<RoleRef>,
    /// The project the row is on, or `None` for the whole organization; a project the viewer
    /// cannot see is not named.
    pub project: Option<ProjectLabel>,
    /// Where it counts.
    pub applies_to: AppliesTo,
    /// Whether the row comes from above the place that was asked for.
    pub inherited: bool,
    /// When it was made.
    pub given_at: Timestamp,
}

impl Masked for AssignmentView {}

/// *Who has which role* for a place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AssignmentList {
    /// The rows: those of the place first, then those that come from above.
    pub rows: Vec<AssignmentView>,
}

impl Masked for AssignmentList {}
