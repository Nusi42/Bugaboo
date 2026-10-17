//! The starter workflows (WF-06, ADR-0031 decision 5, M4-Q1), defined once: *Simple*, *Bug* and
//! *Development*.
//!
//! *Simple* is the starting workflow every issue type had before workflows were data: *Open*, *In
//! progress* and *Done*, and any status can follow any other. It counts until somebody changes a
//! workflow, so a database written by M3 keeps every issue as it is. A new organization also gets
//! *Bug* and *Development*, as copies it may change, for the issue types they fit. The names are
//! messages, written in the language of the catalog when the workflow is made, and can then be
//! renamed.

use super::{StatusChange, StatusKey, StatusRules, WorkflowContent, WorkflowStatus};
use crate::issue::StatusGroup;
use crate::message::{self, Message, MessageKey};

/// The key of *Open*.
pub const OPEN: &str = "open";
/// The key of *In progress*.
pub const IN_PROGRESS: &str = "in-progress";
/// The key of *Done*.
pub const DONE: &str = "done";
/// The key of *Testing*.
pub const TESTING: &str = "testing";
/// The key of *In review*.
pub const IN_REVIEW: &str = "in-review";
/// The key of *Won't fix*.
pub const WONT_FIX: &str = "wont-fix";

const _: () = {
    let keys = [OPEN, IN_PROGRESS, DONE, TESTING, IN_REVIEW, WONT_FIX];
    let mut at = 0;
    while at < keys.len() {
        assert!(
            crate::technical::TechnicalName::is_valid(keys[at]),
            "a key of a starter status is not a technical name"
        );
        at += 1;
    }
};

/// A status of a starter workflow.
#[derive(Clone, Copy, Debug)]
pub struct StarterStatus {
    /// Its key.
    pub key: &'static str,
    /// Its name.
    pub name: MessageKey,
    /// Its status group.
    pub group: StatusGroup,
}

/// A status change of a starter workflow, without rules.
#[derive(Clone, Copy, Debug)]
pub struct StarterChange {
    /// The key of the status it starts from.
    pub from: &'static str,
    /// The key of the status it ends in.
    pub to: &'static str,
    /// The text of its button.
    pub button: MessageKey,
}

/// A starter workflow.
#[derive(Clone, Copy, Debug)]
pub struct Starter {
    /// Its name.
    pub name: MessageKey,
    /// Its statuses, the first of which is where a new issue starts.
    pub statuses: &'static [StarterStatus],
    /// Its status changes.
    pub changes: &'static [StarterChange],
    /// The names of the starting issue types a new organization uses it for.
    pub used_by: &'static [MessageKey],
}

const fn status(key: &'static str, name: MessageKey, group: StatusGroup) -> StarterStatus {
    StarterStatus { key, name, group }
}

const fn change(from: &'static str, to: &'static str, button: MessageKey) -> StarterChange {
    StarterChange { from, to, button }
}

/// *Simple*: three statuses, and any status can follow any other.
pub const SIMPLE: Starter = Starter {
    name: message::WORKFLOW_SIMPLE,
    statuses: &[
        status(OPEN, message::STATUS_OPEN, StatusGroup::NotStarted),
        status(
            IN_PROGRESS,
            message::STATUS_IN_PROGRESS,
            StatusGroup::InProgress,
        ),
        status(DONE, message::STATUS_DONE, StatusGroup::Done),
    ],
    changes: &[
        change(OPEN, IN_PROGRESS, message::WORKFLOW_CHANGE_START_WORKING),
        change(OPEN, DONE, message::WORKFLOW_CHANGE_MARK_AS_DONE),
        change(IN_PROGRESS, OPEN, message::WORKFLOW_CHANGE_STOP_WORKING),
        change(IN_PROGRESS, DONE, message::WORKFLOW_CHANGE_MARK_AS_DONE),
        change(DONE, OPEN, message::WORKFLOW_CHANGE_REOPEN),
        change(DONE, IN_PROGRESS, message::WORKFLOW_CHANGE_REOPEN_AND_START),
    ],
    used_by: &[message::ISSUE_TYPE_TASK, message::ISSUE_TYPE_GOAL],
};

/// *Bug*: with *Testing* and *Won't fix*.
pub const BUG: Starter = Starter {
    name: message::WORKFLOW_BUG,
    statuses: &[
        status(OPEN, message::STATUS_OPEN, StatusGroup::NotStarted),
        status(
            IN_PROGRESS,
            message::STATUS_IN_PROGRESS,
            StatusGroup::InProgress,
        ),
        status(TESTING, message::STATUS_TESTING, StatusGroup::InProgress),
        status(DONE, message::STATUS_DONE, StatusGroup::Done),
        status(WONT_FIX, message::STATUS_WONT_FIX, StatusGroup::Done),
    ],
    changes: &[
        change(OPEN, IN_PROGRESS, message::WORKFLOW_CHANGE_START_WORKING),
        change(OPEN, WONT_FIX, message::WORKFLOW_CHANGE_WONT_FIX),
        change(IN_PROGRESS, OPEN, message::WORKFLOW_CHANGE_STOP_WORKING),
        change(
            IN_PROGRESS,
            TESTING,
            message::WORKFLOW_CHANGE_SEND_TO_TESTING,
        ),
        change(IN_PROGRESS, WONT_FIX, message::WORKFLOW_CHANGE_WONT_FIX),
        change(TESTING, DONE, message::WORKFLOW_CHANGE_MARK_AS_DONE),
        change(TESTING, IN_PROGRESS, message::WORKFLOW_CHANGE_BACK_TO_WORK),
        change(TESTING, WONT_FIX, message::WORKFLOW_CHANGE_WONT_FIX),
        change(DONE, OPEN, message::WORKFLOW_CHANGE_REOPEN),
        change(WONT_FIX, OPEN, message::WORKFLOW_CHANGE_REOPEN),
    ],
    used_by: &[message::ISSUE_TYPE_BUG],
};

/// *Development*: with *In review* and *Testing*.
pub const DEVELOPMENT: Starter = Starter {
    name: message::WORKFLOW_DEVELOPMENT,
    statuses: &[
        status(OPEN, message::STATUS_OPEN, StatusGroup::NotStarted),
        status(
            IN_PROGRESS,
            message::STATUS_IN_PROGRESS,
            StatusGroup::InProgress,
        ),
        status(
            IN_REVIEW,
            message::STATUS_IN_REVIEW,
            StatusGroup::InProgress,
        ),
        status(TESTING, message::STATUS_TESTING, StatusGroup::InProgress),
        status(DONE, message::STATUS_DONE, StatusGroup::Done),
    ],
    changes: &[
        change(OPEN, IN_PROGRESS, message::WORKFLOW_CHANGE_START_WORKING),
        change(IN_PROGRESS, OPEN, message::WORKFLOW_CHANGE_STOP_WORKING),
        change(
            IN_PROGRESS,
            IN_REVIEW,
            message::WORKFLOW_CHANGE_ASK_FOR_REVIEW,
        ),
        change(IN_REVIEW, TESTING, message::WORKFLOW_CHANGE_SEND_TO_TESTING),
        change(
            IN_REVIEW,
            IN_PROGRESS,
            message::WORKFLOW_CHANGE_BACK_TO_WORK,
        ),
        change(TESTING, DONE, message::WORKFLOW_CHANGE_MARK_AS_DONE),
        change(TESTING, IN_PROGRESS, message::WORKFLOW_CHANGE_BACK_TO_WORK),
        change(DONE, OPEN, message::WORKFLOW_CHANGE_REOPEN),
    ],
    used_by: &[message::ISSUE_TYPE_FEATURE],
};

/// The starter workflows of a new organization, in the order they are made.
pub const STARTERS: [Starter; 3] = [SIMPLE, BUG, DEVELOPMENT];

impl Starter {
    /// The name, in the language of the catalog.
    #[must_use]
    pub fn title(&self) -> String {
        Message::from(self.name).render()
    }

    /// The workflow with its texts written in the language of the catalog. A change has no rules:
    /// every person who may change the issue may make it.
    #[must_use]
    pub fn content(&self) -> WorkflowContent {
        WorkflowContent {
            statuses: self
                .statuses
                .iter()
                .map(|each| WorkflowStatus {
                    key: StatusKey::written(each.key),
                    name: Message::from(each.name).render(),
                    group: each.group,
                })
                .collect(),
            changes: self
                .changes
                .iter()
                .map(|each| StatusChange {
                    from: StatusKey::written(each.from),
                    to: StatusKey::written(each.to),
                    button: Message::from(each.button).render(),
                    rules: StatusRules::default(),
                })
                .collect(),
        }
    }
}
