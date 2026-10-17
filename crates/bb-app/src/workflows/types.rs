//! What the workflow use cases give back.

use bb_core::fields::FieldKey;
use bb_core::issue::{Issue, StatusGroup};
use bb_core::workflow::{StatusKey, WorkflowContent, WorkflowStatus};
use serde::Serialize;

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
