//! What the issues use cases take and give back: inputs that change something are named after what
//! they do; inputs that only read end in `Query` (ARCH-EVENTS). Everything that is given back is
//! wrapped in `Authorized<T>` by the use case.
//!
//! An issue is only ever shown to a person who may see it, and a view holds a field of the
//! catalogue only under the member that the mask can drop (`#[derive(Masked)]`, so a member that
//! is not declared does not compile and a hidden field of a nested view never leaves). No answer
//! holds a number that counts issues (PRIV-13): a list has pages and a cursor, never a total.

use std::collections::BTreeMap;

use bb_core::fields::FieldKey;
use bb_core::id::{IssueId, IssueTypeId, LockToken, UserId};
use bb_core::issue::{IssueIcon, IssueType, Priority};
use bb_core::own_fields_value::OwnValues;
use bb_core::workflow::StatusKey;
use bb_policy::{Masked, PersonRef};
use jiff::Timestamp;
use serde::Serialize;
use serde_json::Value;

use crate::projects::ProjectRef;
use crate::workflows::{StatusChangeView, StatusView};

/// Make an issue in a project.
pub struct CreateIssue {
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
    /// Where it starts; the first status of its workflow when left out. Another status is judged as
    /// a status change from the first.
    pub status: Option<StatusKey>,
    /// How important it is; *Normal* when left out.
    pub priority: Option<Priority>,
    /// The people it is assigned to; nobody when left out.
    pub assigned_to: Option<Vec<UserId>>,
    /// The values of its own fields, by the key of the field; the defaults fill in the rest.
    pub fields: BTreeMap<String, Value>,
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
    /// A new status: judged by the workflow of the issue. Entering the group *Done* sets the
    /// closed date; leaving it clears it.
    pub status: Option<StatusKey>,
    /// A new priority.
    pub priority: Option<Priority>,
    /// The people it is assigned to from now on (all of them, not the ones to add).
    pub assigned_to: Option<Vec<UserId>>,
    /// The values of own fields to set, by the key of the field; `null` takes a value away. A
    /// long text is saved on its own route, under its edit lock.
    pub fields: BTreeMap<String, Value>,
}

/// Move an issue into another project.
pub struct MoveIssue {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
    /// The version the person looked at; a change made meanwhile stops this one.
    pub version: i64,
    /// The project it goes into, by its ID or its short name (an old one too).
    pub project: String,
}

/// Save the description of an issue under the edit lock the person holds.
pub struct SaveDescription {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
    /// The version the person started from; a change made meanwhile stops the save.
    pub version: i64,
    /// The token of the edit lock the person holds on the description.
    pub lock_token: LockToken,
    /// The description, in Markdown.
    pub text: String,
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
    /// Its issue type. The levels set for it count for the fields of this issue (ADR-0027).
    #[masked(nested, item_type)]
    pub issue_type: IssueTypeRef,
    /// What it is about.
    #[masked(field = "title")]
    pub title: String,
    /// The long text, in Markdown.
    #[masked(field = "description")]
    pub description: String,
    /// Where it stands in its workflow: the key, the name and the status group.
    #[masked(field = "status")]
    pub status: StatusView,
    /// The status changes the viewer may make now, with the text of each button and the fields
    /// it still needs. Empty when they may not change the issue. Hidden with the status.
    #[masked(field = "status")]
    pub status_changes: Vec<StatusChangeView>,
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
    /// The values of its own fields, by the key of the field, as JSON of the type. Only the
    /// fields the viewer may see on an issue of this type.
    #[masked(own_fields)]
    pub own_fields: OwnValues,
    /// Counts up with every change; a change is made on the version that was looked at.
    #[masked(open)]
    pub version: i64,
}

/// A long text of an own field, rendered for the page: the field and the safe HTML.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct OwnHtml {
    /// The field.
    #[masked(open)]
    pub field: FieldKey,
    /// The text as safe HTML, rendered by `bb-markdown` with the image hosts of the organization.
    /// Only for a field the viewer may see on an issue of this type.
    #[masked(open)]
    pub html: String,
}

/// One issue with what the person may do with it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct IssueDetail {
    /// The issue.
    #[masked(nested)]
    pub issue: IssueView,
    /// The description as safe HTML, rendered by `bb-markdown` with the image hosts of the
    /// organization (the same renderer the editor's preview runs in the browser). Hidden with the
    /// description.
    #[masked(field = "description")]
    pub description_html: String,
    /// The long-text own fields as safe HTML, one for each the viewer may see.
    #[masked(nested)]
    pub own_fields_html: Vec<OwnHtml>,
    /// Whether the person may change the issue at all (in the fields they may change).
    #[masked(open)]
    pub may_change: bool,
    /// Whether the person may delete the issue into the trash.
    #[masked(open)]
    pub may_delete: bool,
    /// The fields the person may change in this issue: empty when they may not change it.
    #[masked(open)]
    pub changeable: Vec<FieldKey>,
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
pub struct CreateIssueType {
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
}
