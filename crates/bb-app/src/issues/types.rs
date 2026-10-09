//! What the issues use cases take and give back: inputs that change something are named after what
//! they do; inputs that only read end in `Query` (ARCH-EVENTS). Everything that is given back is
//! wrapped in `Authorized<T>` by the use case.
//!
//! An issue is only ever shown to a person who may see it, and a view holds a field of the
//! catalogue only under the member that the mask can drop (`#[derive(Masked)]`, so a member that
//! is not declared does not compile and a hidden field of a nested view never leaves). No answer
//! holds a number that counts issues (PRIV-13): a list has pages and a cursor, never a total.

use bb_core::fields::Field;
use bb_core::id::{IssueId, IssueTypeId, UserId};
use bb_core::issue::{IssueIcon, IssueType, Priority, Status, StatusGroup};
use bb_policy::{Masked, PersonRef};
use jiff::Timestamp;
use serde::Serialize;

use crate::projects::ProjectRef;

/// Make an issue in a project.
pub struct MakeIssue {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project it goes into, by its ID or its short name (an old one too).
    pub project: String,
    /// Its issue type; a retired type takes no new issues.
    pub issue_type: IssueTypeId,
    /// What it is about, as typed.
    pub title: String,
    /// The long text in Markdown; empty when left out.
    pub description: Option<String>,
    /// Where it starts; the first status of the starting workflow when left out.
    pub status: Option<Status>,
    /// How important it is; *Normal* when left out.
    pub priority: Option<Priority>,
    /// The people it is assigned to; nobody when left out.
    pub assigned_to: Option<Vec<UserId>>,
}

/// Change the fields of an issue: what is `Some` is changed.
pub struct ChangeIssue {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
    /// The version the person looked at; a change made meanwhile stops this one.
    pub version: i64,
    /// A new issue type; a retired type is refused.
    pub issue_type: Option<IssueTypeId>,
    /// A new title, as typed.
    pub title: Option<String>,
    /// A new description.
    pub description: Option<String>,
    /// A new status. Entering the group *Done* sets the closed date; leaving it clears it.
    pub status: Option<Status>,
    /// A new priority.
    pub priority: Option<Priority>,
    /// The people it is assigned to from now on (all of them, not the ones to add).
    pub assigned_to: Option<Vec<UserId>>,
}

/// Put an issue into the trash, or take it out again (*Undo*).
pub struct DeleteIssue {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
}

/// Take an issue out of the trash.
pub struct RestoreIssue {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
}

/// Which issue a request names.
pub struct IssueQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
}

/// The issues of a project that a person sees, a page at a time, newest first.
pub struct IssuesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, by its ID or its short name (an old one too).
    pub project: String,
    /// Whether the people each issue is assigned to are wanted: they are read only then.
    pub assigned_to: bool,
    /// Where the page after the last one starts (`next_cursor` of the page before), if any.
    pub cursor: Option<String>,
    /// How many issues a page holds at most.
    pub limit: u32,
}

/// An issue type as a small reference: what the line of an issue needs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct IssueTypeRef {
    /// The ID of the type.
    #[masked(open)]
    pub id: IssueTypeId,
    /// What it is called.
    #[masked(open)]
    pub name: String,
    /// Its icon.
    #[masked(open)]
    pub icon: IssueIcon,
    /// Whether it is retired: old issues keep it, new ones cannot have it.
    #[masked(open)]
    pub retired: bool,
}

impl From<&IssueType> for IssueTypeRef {
    fn from(issue_type: &IssueType) -> Self {
        Self {
            id: issue_type.id,
            name: issue_type.name.clone(),
            icon: issue_type.icon,
            retired: issue_type.retired_at.is_some(),
        }
    }
}

/// An issue as it is shown. A member that holds a field of the catalogue is dropped when the
/// viewer's level for the field is *Hidden*; the people in it are shown as the viewer's level of
/// *Who sees people* says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct IssueView {
    /// The internal ID, which never changes.
    #[masked(open)]
    pub id: IssueId,
    /// The readable ID now (`WEB-142`, `WEB-K7Q2X`).
    #[masked(open)]
    pub key: String,
    /// The project it is in.
    #[masked(nested)]
    pub project: ProjectRef,
    /// Its issue type.
    #[masked(nested)]
    pub issue_type: IssueTypeRef,
    /// What it is about.
    #[masked(field = "title")]
    pub title: String,
    /// The long text, in Markdown.
    #[masked(field = "description")]
    pub description: String,
    /// Where it stands in the workflow.
    #[masked(field = "status")]
    pub status: Status,
    /// The group of its status.
    #[masked(field = "status")]
    pub status_group: StatusGroup,
    /// How important it is.
    #[masked(field = "priority")]
    pub priority: Priority,
    /// Who made it.
    #[masked(field = "reported-by", nested)]
    pub reported_by: PersonRef,
    /// Who works on it; empty when the people were not asked for.
    #[masked(field = "assigned-to", nested)]
    pub assigned_to: Vec<PersonRef>,
    /// When it was made.
    #[masked(open)]
    pub created_at: Timestamp,
    /// When it was last changed.
    #[masked(open)]
    pub updated_at: Timestamp,
    /// When it entered the group *Done*, while it is in it. Hidden with the status.
    #[masked(field = "status")]
    pub closed_at: Option<Timestamp>,
    /// Counts up with every change; a change is made on the version that was looked at.
    #[masked(open)]
    pub version: i64,
}

/// One issue with what the person may do with it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct IssueDetail {
    /// The issue.
    #[masked(nested)]
    pub issue: IssueView,
    /// Whether the person may change the issue at all (in the fields they may change).
    #[masked(open)]
    pub may_change: bool,
    /// Whether the person may delete the issue into the trash.
    #[masked(open)]
    pub may_delete: bool,
    /// The fields the person may change in this issue: empty when they may not change it.
    #[masked(open)]
    pub changeable: Vec<Field>,
}

/// A page of issues.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct IssuePage {
    /// The issues, newest first.
    #[masked(nested)]
    pub items: Vec<IssueView>,
    /// The cursor of the next page, if there is one.
    #[masked(open)]
    pub next_cursor: Option<String>,
}

/// Make an issue type.
pub struct MakeIssueType {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What it is called, as typed.
    pub name: String,
    /// Its icon.
    pub icon: IssueIcon,
}

/// Change an issue type: what is `Some` is changed.
pub struct ChangeIssueType {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The type.
    pub issue_type: IssueTypeId,
    /// The version the person looked at.
    pub version: i64,
    /// A new name.
    pub name: Option<String>,
    /// A new icon.
    pub icon: Option<IssueIcon>,
    /// `true` to retire it (no new issues of this type), `false` to offer it again.
    pub retired: Option<bool>,
}

/// List the issue types.
pub struct IssueTypesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many types the page holds at most.
    pub limit: u32,
}

/// An issue type as the list and the screen of issue types show it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct IssueTypeView {
    /// The ID of the type.
    #[masked(open)]
    pub id: IssueTypeId,
    /// What it is called.
    #[masked(open)]
    pub name: String,
    /// Its icon.
    #[masked(open)]
    pub icon: IssueIcon,
    /// Whether it is retired.
    #[masked(open)]
    pub retired: bool,
    /// Counts up with every change.
    #[masked(open)]
    pub version: i64,
}

impl From<&IssueType> for IssueTypeView {
    fn from(issue_type: &IssueType) -> Self {
        Self {
            id: issue_type.id,
            name: issue_type.name.clone(),
            icon: issue_type.icon,
            retired: issue_type.retired_at.is_some(),
            version: issue_type.version,
        }
    }
}

/// A page of issue types, in the order they are offered in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct IssueTypePage {
    /// The types.
    #[masked(nested)]
    pub items: Vec<IssueTypeView>,
    /// The cursor of the next page, if there is one.
    #[masked(open)]
    pub next_cursor: Option<String>,
    /// Whether the person may make and change issue types.
    #[masked(open)]
    pub may_manage: bool,
}
