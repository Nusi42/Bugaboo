//! What the relation use cases take and give back (ISS-03, ISS-04). Inputs that change something
//! are named after what they do; inputs that only read end in `Query` (ARCH-EVENTS). Everything
//! that is given back is wrapped in `Authorized<T>` by the use case.
//!
//! An issue appears in an answer only to a person who may see it, as a [`IssueLine`] that holds
//! the fields of the catalogue the person may see *in the project of that issue*: a relation or a
//! sub-issue can lie in another project, whose roles can see other fields.

use bb_core::id::{IssueId, RelationId, RelationKindId};
use bb_core::issue::{Status, StatusGroup};
use bb_core::relation::{Direction, RelationKind};
use bb_policy::Masked;
use serde::Serialize;

use crate::issues::IssueTypeRef;
use crate::projects::ProjectRef;

/// An issue as one line of a list in the issue page: its readable ID, its title and its status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IssueLine {
    /// The internal ID of the issue.
    pub id: IssueId,
    /// The readable ID now (`WEB-142`).
    pub key: String,
    /// The project it is in.
    pub project: ProjectRef,
    /// Its issue type.
    pub issue_type: IssueTypeRef,
    /// What it is about; left out where the viewer's role hides the field in that project.
    pub title: Option<String>,
    /// Where it stands; left out where the viewer's role hides the field in that project.
    pub status: Option<Status>,
    /// The group of its status; left out with the status.
    pub status_group: Option<StatusGroup>,
}

// The members are dropped by the line's own project, which can differ from the place of the
// answer, so the mask of the answer has nothing to do here.
impl Masked for IssueLine {}

/// How far the sub-issues of an issue are. Only the sub-issues the viewer may see are in it
/// (PRIV-13).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Progress {
    /// Those in the group *Done*.
    pub done: u32,
    /// All that the viewer sees.
    pub total: u32,
}

/// One relation as one end of it reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RelationView {
    /// The ID of the relation.
    pub id: RelationId,
    /// What it means.
    pub kind: RelationKindId,
    /// Which end the issue asked about is at.
    pub direction: Direction,
    /// What that end reads (*blocks*, *is blocked by*).
    pub name: String,
    /// The issue at the other end.
    pub issue: IssueLine,
}

impl Masked for RelationView {}

/// Where an issue stands among the others, as the viewer may see it: the issue it sits under and
/// how far its sub-issues are. The lists themselves come a page at a time (`SubIssuePage`,
/// `RelatedIssuePage`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IssueRelations {
    /// The issue these belong to.
    pub issue: IssueId,
    /// The version of the issue: what *Part of* is changed from.
    pub version: i64,
    /// The issue it sits under, if the viewer may see it.
    pub part_of: Option<IssueLine>,
    /// How far the sub-issues are; `None` without sub-issues, or when the viewer may not see the
    /// status.
    pub progress: Option<Progress>,
    /// Whether the viewer may add and take away sub-issues and relations here.
    pub may_change: bool,
}

impl Masked for IssueRelations {}

/// Which issue and what is asked of it.
pub struct IssueRelationsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
}

/// The sub-issues of an issue that the viewer may see, a page at a time.
pub struct SubIssuesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many issues a page holds at most.
    pub limit: u32,
}

/// A page of sub-issues, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SubIssuePage {
    /// The sub-issues the viewer may see.
    pub items: Vec<IssueLine>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for SubIssuePage {}

/// The related issues of an issue whose other end the viewer may see, a page at a time.
pub struct RelatedIssuesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many relations a page holds at most.
    pub limit: u32,
}

/// A page of relations, by kind and then oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RelatedIssuePage {
    /// The relations the viewer may see both ends of.
    pub items: Vec<RelationView>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for RelatedIssuePage {}

/// Put an issue under another one, or take it out from under its parent.
pub struct SetParent {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The sub-issue, by its readable ID or its internal ID.
    pub issue: String,
    /// The version of the sub-issue the person looked at.
    pub version: i64,
    /// The issue it goes under, or `None` to take it out.
    pub parent: Option<String>,
}

/// Add a relation between two issues.
pub struct AddRelation {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue the person starts from.
    pub issue: String,
    /// The issue at the other end.
    pub other: String,
    /// What the relation means.
    pub kind: RelationKindId,
    /// Which end `issue` is: *this issue blocks the other* is [`Direction::Forward`].
    pub direction: Direction,
}

/// Take a relation away.
pub struct RemoveRelation {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The relation.
    pub relation: RelationId,
}

/// List the kinds of relation.
pub struct RelationKindsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many kinds the page holds at most.
    pub limit: u32,
}

/// Make a kind of relation.
pub struct MakeRelationKind {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What the issue it starts from reads, as typed.
    pub forward: String,
    /// What the other issue reads, as typed.
    pub backward: String,
}

/// Change a kind of relation: what is `Some` is changed.
pub struct ChangeRelationKind {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The kind.
    pub kind: RelationKindId,
    /// The version the person looked at.
    pub version: i64,
    /// A new forward name.
    pub forward: Option<String>,
    /// A new backward name.
    pub backward: Option<String>,
    /// `true` to retire it (no new relations of this kind), `false` to offer it again.
    pub retired: Option<bool>,
}

/// A kind of relation as the screen of kinds shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RelationKindView {
    /// The ID of the kind.
    pub id: RelationKindId,
    /// What the issue it starts from reads.
    pub forward: String,
    /// What the other issue reads.
    pub backward: String,
    /// Whether it is retired.
    pub retired: bool,
    /// Counts up with every change.
    pub version: i64,
}

impl Masked for RelationKindView {}

impl From<&RelationKind> for RelationKindView {
    fn from(kind: &RelationKind) -> Self {
        Self {
            id: kind.id,
            forward: kind.forward.clone(),
            backward: kind.backward.clone(),
            retired: kind.retired_at.is_some(),
            version: kind.version,
        }
    }
}

/// A page of kinds of relation, in the order they are offered in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RelationKindPage {
    /// The kinds.
    pub items: Vec<RelationKindView>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for RelationKindPage {}

/// Makes the kinds of relation of the first start if there are none yet.
pub struct MakeStartingRelationKinds;
