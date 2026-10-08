//! What the projects use cases take and give back: inputs that change something are named after
//! what they do; inputs that only read end in `Query` (ARCH-EVENTS). Everything that is given back
//! is wrapped in `Authorized<T>` by the use case.
//!
//! A project is only ever named to a person who may see it, and its parent only when the person
//! may see that too: a visible sub-project of a hidden project is shown at the top of the tree, and
//! nothing in an answer names the hidden parent, not even by its ID (UX §2.1, ADR-0025 decision
//! 13). No answer holds a number that counts projects: *has sub-projects* is a yes or no about the
//! sub-projects the person sees (PRIV-13).

use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::ProjectId;
use bb_core::people_kinds::PeopleKind;
use bb_core::project::{Project, ShortName};
use bb_policy::{Masked, Permission, PersonRef};
use serde::Serialize;

/// Make a project at the top level, or inside another project.
pub struct MakeProject {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project it goes into, or `None` for the top level.
    pub parent: Option<ProjectId>,
    /// The letters of its issue IDs and address, as typed.
    pub short_name: String,
    /// What it is called.
    pub name: String,
}

/// Change the name or the short name of a project: what is `Some` is changed.
pub struct ChangeProject {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project.
    pub project: ProjectId,
    /// The version the person looked at; a change made meanwhile stops this one.
    pub version: i64,
    /// A new name.
    pub name: Option<String>,
    /// A new short name, as typed. The old one stays with the project.
    pub short_name: Option<String>,
}

/// Put a project inside another one, or at the top level, after looking at the preview.
pub struct MoveProject {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project that moves, with everything in it.
    pub project: ProjectId,
    /// The project it goes into, or `None` for the top level.
    pub to: Option<ProjectId>,
    /// The `preview` of what the person looked at: the move happens only when nothing it
    /// depends on has changed since.
    pub preview: String,
}

/// What a move would do, as a person looked at it: before anything happens.
pub struct MovePreviewQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project that would move.
    pub project: ProjectId,
    /// Where it would go: another project, or the top level for `None`.
    pub to: Option<ProjectId>,
}

/// Which project a request names: its ID, or a short name (also an old one).
pub struct ProjectQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The ID or the short name, as it stands in the address.
    pub address: String,
}

/// The projects a person sees, a page at a time.
pub struct ProjectsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The projects directly inside this one; the top of the tree when `None`. Ignored when
    /// `search` is set.
    pub inside: Option<ProjectId>,
    /// Words that every found project holds in its name or short name, anywhere in the tree;
    /// the text may be empty, which finds every project the person sees.
    pub search: Option<String>,
    /// Leave out this project and everything inside it, when the person sees the project; for a
    /// project they do not see, or that does not exist, nothing is left out.
    pub outside: Option<ProjectId>,
    /// List only the places this project (which the person sees) may be moved into by them:
    /// not itself, not inside itself, not where it is, and only where they may move projects.
    /// For a project they do not see, or that does not exist, it is ignored.
    pub movable: Option<ProjectId>,
    /// Where the page after the last one starts (`next_cursor` of the page before), if any.
    pub cursor: Option<String>,
    /// How many projects a page holds at most; more than `bb_core::paging::PAGE_SIZE_MAX` is cut
    /// to it.
    pub limit: u32,
}

/// A project as a small reference: what a name in a sentence needs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectRef {
    /// The ID of the project.
    pub id: ProjectId,
    /// The letters of its issue IDs and address.
    pub short_name: ShortName,
    /// What it is called.
    pub name: String,
}

impl Masked for ProjectRef {}

impl From<&Project> for ProjectRef {
    fn from(project: &Project) -> Self {
        Self {
            id: project.id,
            short_name: project.short_name.clone(),
            name: project.name.clone(),
        }
    }
}

/// A project as it is shown after it was made, changed or moved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectView {
    /// The ID of the project.
    pub id: ProjectId,
    /// The project it is inside, when the person may see that one; `None` at the top level and
    /// for a project whose parent they cannot see.
    pub parent: Option<ProjectId>,
    /// The letters of its issue IDs and address.
    pub short_name: ShortName,
    /// What it is called.
    pub name: String,
    /// Counts up with every change; a change is made on the version that was looked at.
    pub version: i64,
}

impl Masked for ProjectView {}

/// One project in a list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectItem {
    /// The ID of the project.
    pub id: ProjectId,
    /// The letters of its issue IDs and address.
    pub short_name: ShortName,
    /// What it is called.
    pub name: String,
    /// Whether it holds sub-projects the person sees. A yes or no, never a number.
    pub has_sub_projects: bool,
    /// The project it is inside, when the person may see that one.
    pub parent: Option<ProjectRef>,
}

impl Masked for ProjectItem {}

/// A page of projects.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectPage {
    /// The projects, by name.
    pub items: Vec<ProjectItem>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for ProjectPage {}

/// How many projects of the way down to a project are named at most.
pub const TRAIL_MAX: usize = 16;

/// Where a person may move a project to: they need *Move projects* in the place it sits in, and
/// for the top level *Make top-level projects* for the whole organization as well.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum MoveRight {
    /// Not anywhere.
    Nowhere,
    /// Into other projects, not to the top level.
    IntoProjects,
    /// Into other projects, and to the top level (the project is not there yet).
    IntoProjectsOrTop,
}

/// One project, with where it is and what the person may do there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectDetail {
    /// The project.
    pub project: ProjectView,
    /// *Where you are*: the projects above it that the person may see, from the top down. It
    /// stops at the first one the person cannot see, so no hidden project is named.
    pub trail: Vec<ProjectRef>,
    /// The trail was cut at [`TRAIL_MAX`].
    pub truncated: bool,
    /// What the person may do here.
    pub permissions: Vec<Permission>,
    /// Whether the person may change the short name: they change this project's settings and
    /// may make projects where it sits, so a short name that is taken can be told to them
    /// (ADR-0025, decision 13).
    pub may_change_short_name: bool,
    /// Where the person may move the project to.
    pub may_move: MoveRight,
}

impl Masked for ProjectDetail {}

/// A level of a field that a move changes for a person.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct FieldLevelChange {
    /// The field.
    pub field: Field,
    /// What it was.
    pub before: FieldLevel,
    /// What it would be.
    pub after: FieldLevel,
}

/// A level of a kind of people information that a move changes for a person.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PeopleLevelChange {
    /// The kind.
    pub kind: PeopleKind,
    /// What it was.
    pub before: PeopleLevel,
    /// What it would be.
    pub after: PeopleLevel,
}

/// What a person has in one project: whether they see it, whether a *No access* holds them back
/// there, and the names of their roles in the order of roles.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Standing {
    /// The person sees the project.
    pub sees: bool,
    /// A *No access* holds the person back there.
    pub held_back: bool,
    /// The names of their roles there.
    pub roles: Vec<String>,
}

/// What a move changes for one person in one project that moves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectChange {
    /// The project.
    pub project: ProjectRef,
    /// What the person has there now.
    pub now: Standing,
    /// What they would have after the move.
    pub after: Standing,
    /// What they could not do and could after the move.
    pub gains: Vec<Permission>,
    /// What they could do and could not after the move.
    pub loses: Vec<Permission>,
    /// The fields whose level changes.
    pub fields: Vec<FieldLevelChange>,
    /// The kinds of people information whose level changes.
    pub people: Vec<PeopleLevelChange>,
}

/// What a move changes for one person, in every project that moves where it changes something.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PersonChange {
    /// The person, as the person who asks may see them.
    pub person: PersonRef,
    /// The projects where something changes for them.
    pub projects: Vec<ProjectChange>,
}

/// Somebody who could take away a *No access* that applies to them after the move, and the project
/// that *No access* is on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LiftView {
    /// The person, as the person who asks may see them.
    pub person: PersonRef,
    /// The project the *No access* is on.
    pub project: ProjectRef,
}

/// The most people a preview names, and the most changes it lists.
pub const PREVIEW_PEOPLE_MAX: usize = 100;
/// The most changes a preview lists in all.
pub const PREVIEW_CHANGES_MAX: usize = 500;

/// What a move would do: who gains or loses access or changes role, and whether it is refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MovePreview {
    /// The project that would move.
    pub project: ProjectRef,
    /// The project it would go into, or `None` for the top level.
    pub to: Option<ProjectRef>,
    /// The version of this preview. Moving with it works only while nothing it was made from has
    /// changed: the projects, who has which role, and the levels the roles set.
    pub preview: String,
    /// The move would be refused, because somebody could take away a *No access* that applies to
    /// them (`lifts` says who).
    pub blocked: bool,
    /// Something in a project the person cannot see also stops the move: one sentence says so,
    /// without a name or a number.
    pub blocked_out_of_sight: bool,
    /// Who gains or loses access or changes role, where the person who asks can see the project.
    pub people: Vec<PersonChange>,
    /// Who could take away a *No access* after the move and could not before.
    pub lifts: Vec<LiftView>,
    /// The list is cut at [`PREVIEW_PEOPLE_MAX`] people or [`PREVIEW_CHANGES_MAX`] changes.
    pub truncated: bool,
}

impl Masked for MovePreview {}
