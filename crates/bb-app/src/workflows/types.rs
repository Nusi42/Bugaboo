//! What the workflow use cases give back.

use bb_core::fields::FieldKey;
use bb_core::id::{GroupId, IssueTypeId, LockToken, RoleId, UserId, WorkflowId};
use bb_core::issue::{Issue, StatusGroup};
use bb_core::workflow::judge::Why;
use bb_core::workflow::{
    ChangeDraft, StatusDraft, StatusKey, StatusRules, WorkflowContent, WorkflowStatus,
};
use bb_policy::Masked;
use serde::Serialize;

use crate::issues::IssueTypeRef;
use crate::layouts::InheritedFrom;
use crate::projects::ProjectRef;

/// A status as it is shown: its key, what it is called, and its status group.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StatusView {
    /// The key, which never changes.
    pub key: StatusKey,
    /// What it is called.
    pub name: String,
    /// The status group it belongs to.
    pub group: StatusGroup,
}

impl From<&WorkflowStatus> for StatusView {
    fn from(status: &WorkflowStatus) -> Self {
        Self {
            key: status.key.clone(),
            name: status.name.clone(),
            group: status.group,
        }
    }
}

impl StatusView {
    /// The status an issue is in, as its workflow names it. An issue in a status that its workflow
    /// does not have (the status was dropped, or the issue moved to a workflow without it) shows
    /// the key as its name, and the group *Done* while it has a closed date.
    #[must_use]
    pub fn of_issue(workflow: &WorkflowContent, issue: &Issue) -> Self {
        workflow.status(&issue.status).map_or_else(
            || Self {
                key: issue.status.clone(),
                name: issue.status.to_string(),
                group: if issue.closed_at.is_some() {
                    StatusGroup::Done
                } else {
                    StatusGroup::NotStarted
                },
            },
            Self::from,
        )
    }
}

/// A status change the reader may make now: the status it ends in, the text of its button, and
/// the fields it still needs (*Must be filled in* and *Ask for*).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StatusChangeView {
    /// The status it ends in.
    pub to: StatusView,
    /// The text of the button.
    pub button: String,
    /// The fields the change needs before it can be made; empty when it needs none.
    pub needs: Vec<FieldKey>,
}

/// Ask for the workflow of a place to edit.
pub struct WorkflowEditorQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, by its ID or its short name; none for the whole organization.
    pub project: Option<String>,
    /// The issue type; none for all types.
    pub issue_type: Option<IssueTypeId>,
}

/// Ask which workflows there are, for choosing one for a place.
pub struct WorkflowsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project the choice is for, by its ID or its short name; none for the whole organization.
    pub project: Option<String>,
}

/// Create the workflow of a place as a copy of the one that counts there now, so the place can
/// change it.
pub struct CreateWorkflow {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, by its ID or its short name; none for the whole organization.
    pub project: Option<String>,
    /// The issue type; none for all types.
    pub issue_type: Option<IssueTypeId>,
}

/// Say which workflow a place uses, or that it follows its parent again.
pub struct UseWorkflow {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, by its ID or its short name; none for the whole organization.
    pub project: Option<String>,
    /// The issue type; none for all types.
    pub issue_type: Option<IssueTypeId>,
    /// The workflow the place uses from now on; none to follow the parent again.
    pub workflow: Option<WorkflowId>,
}

/// Where the issues of a status go when the workflow no longer has it.
#[derive(Clone, Debug)]
pub struct StatusMoveInput {
    /// The status that is dropped.
    pub from: StatusKey,
    /// The status of the new workflow the issues go to.
    pub to: StatusKey,
}

/// Publish a workflow under the edit lock the person holds: it counts from now on.
pub struct PublishWorkflow {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The workflow.
    pub workflow: WorkflowId,
    /// The version the person looked at.
    pub version: i64,
    /// The token the edit lock was taken with.
    pub lock_token: LockToken,
    /// What the workflow is called.
    pub name: String,
    /// All the statuses, in order. The first is where a new issue starts.
    pub statuses: Vec<StatusDraft>,
    /// All the status changes. They replace the ones that are stored.
    pub changes: Vec<ChangeDraft>,
    /// Where the issues of a dropped status go.
    pub moves: Vec<StatusMoveInput>,
}

/// Take a workflow away, together with the places it was used at.
pub struct DeleteWorkflow {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The workflow.
    pub workflow: WorkflowId,
    /// The version the person looked at.
    pub version: i64,
}

/// Ask which status changes a person would get on an issue, and why not the others (*Try it*).
pub struct TryWorkflowQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
    /// The person to try it for.
    pub person: UserId,
}

/// The workflow set at exactly the place that was asked about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct WorkflowSlot {
    /// The ID of the workflow, which the edit lock is about.
    pub id: WorkflowId,
    /// Counts up with every change. A change is made on the version that was looked at.
    pub version: i64,
}

/// A status change in the editor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EditorChange {
    /// The status it starts from.
    pub from: StatusKey,
    /// The status it ends in.
    pub to: StatusKey,
    /// The text of the button.
    pub button: String,
    /// Its rules.
    pub rules: StatusRules,
}

/// A role a rule can name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EditorRole {
    /// The ID of the role.
    pub id: RoleId,
    /// Its name.
    pub name: String,
}

/// A group a rule can name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EditorGroup {
    /// The ID of the group.
    pub id: GroupId,
    /// Its name.
    pub name: String,
}

/// A field a rule can name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RuleField {
    /// The field.
    pub field: FieldKey,
    /// The name of an own field the editor may know; a built-in field is named by its word.
    pub name: Option<String>,
}

/// What the editor shows for a place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowEditor {
    /// The project of the place; none for the whole organization.
    pub project: Option<ProjectRef>,
    /// The issue type of the place; none for all types.
    pub issue_type: Option<IssueTypeRef>,
    /// The workflow set at this place. Without it the place uses the workflow it inherits (or the
    /// starting workflow), and *Create* makes one.
    pub workflow: Option<WorkflowSlot>,
    /// Where the workflow that counts is set, when it is another place.
    pub inherited_from: Option<InheritedFrom>,
    /// What the workflow is called.
    pub name: String,
    /// Its statuses, in order.
    pub statuses: Vec<StatusView>,
    /// Its status changes.
    pub changes: Vec<EditorChange>,
    /// The roles *Who can do it* can name.
    pub roles: Vec<EditorRole>,
    /// The groups *Who can do it* can name.
    pub groups: Vec<EditorGroup>,
    /// The fields the rules can name.
    pub fields: Vec<RuleField>,
}

impl Masked for WorkflowEditor {}

/// A workflow in the list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorkflowListItem {
    /// The ID of the workflow.
    pub id: WorkflowId,
    /// What it is called.
    pub name: String,
    /// Counts up with every change.
    pub version: i64,
}

/// The workflows there are.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorkflowList {
    /// The workflows, by name.
    pub items: Vec<WorkflowListItem>,
}

impl Masked for WorkflowList {}

/// What *Publish* did: the workflow with its new version. The issues of the dropped statuses were
/// moved by changes to many issues, each with its history; one the person could not change stays
/// where it is and shows its old status by its key.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Published {
    /// The workflow, with its new version.
    pub workflow: WorkflowSlot,
}

impl Masked for Published {}

/// The judge's answer for one status change, as *Try it* tells it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum TriedVerdict {
    /// The person may make it.
    Allowed,
    /// The person may make it once these fields have a value.
    Needs {
        /// The fields.
        fields: Vec<FieldKey>,
    },
    /// The person may not make it.
    NotAllowed {
        /// Why.
        why: Why,
        /// The field it is about, when the person who tries may see it too.
        field: Option<FieldKey>,
    },
}

/// One status change *Try it* looked at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TriedChange {
    /// The status it ends in.
    pub to: StatusView,
    /// The text of the button.
    pub button: String,
    /// What the judge says.
    pub verdict: TriedVerdict,
}

/// What *Try it* found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Tried {
    /// The status the issue is in.
    pub status: StatusView,
    /// The status changes from there, each with the answer for the person.
    pub changes: Vec<TriedChange>,
}

impl Masked for Tried {}
