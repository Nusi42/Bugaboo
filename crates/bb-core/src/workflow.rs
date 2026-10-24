//! Workflows (WF-01, WF-03, WF-04, ISS-02, PRJ-02, ADR-0031): the statuses an issue can have and the
//! status changes allowed between them, as data.
//!
//! A **workflow** has statuses (a stored key, a name and a status group) and status changes (a
//! from-status, a to-status, the button text and the rules). It is found for a project and an issue
//! type like a layout: the nearest place with a row wins, and at one place the row for the issue
//! type wins over the row for all types ([`find`], which is `crate::lookup::nearest_row`). Until
//! somebody changes a workflow no row exists, and the starting workflow *Simple* counts
//! ([`starter::SIMPLE`]); a database written by M3 therefore keeps every issue as it is.
//!
//! The key of a status never changes (a rename changes the name only), so stored issues and history
//! keep working. One function decides whether a person may make a status change ([`judge::judge`]).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::fields::FieldKey;
use crate::id::{GroupId, IssueTypeId, ProjectId, RoleId, WorkflowId};
use crate::issue::StatusGroup;
use crate::lookup::nearest_row;
use crate::personal::{Entity, NotPersonal};
use crate::technical::{InvalidTechnicalName, TechnicalName};

mod content;
pub mod judge;
pub mod starter;

pub use content::{
    ChangeDraft, Problem, StatusDraft, WorkflowContent, ask_may_name, check_name, is_new_in,
    rule_may_name, status_named,
};

/// The most statuses a workflow has.
pub const STATUSES_MAX: usize = 30;
/// The most status changes a workflow has.
pub const CHANGES_MAX: usize = 300;
/// The most characters of the name of a workflow, a status or a button.
pub const NAME_MAX: usize = 60;
/// The most fields one rule of a status change names.
pub const RULE_FIELDS_MAX: usize = 20;
/// The most roles and groups *Who can do it* names.
pub const WHO_MAX: usize = 50;
/// The most characters of the key made from the name of a new status.
const KEY_FROM_NAME_MAX: usize = 40;

/// The key of a status: short text that never changes, stored with every issue and in history
/// (`open`, `in-progress`). It cannot hold what a person typed, so it holds no personal data.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StatusKey(TechnicalName);

impl StatusKey {
    /// The key `text`.
    ///
    /// # Errors
    /// Fails unless `text` is a technical name.
    pub fn new(text: &str) -> Result<Self, InvalidTechnicalName> {
        TechnicalName::new(text).map(Self)
    }

    /// A key that is written in the code and checked by a `const` assertion where it is declared.
    pub(crate) fn written(text: &'static str) -> Self {
        Self(TechnicalName::trusted(text))
    }

    /// The key as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The key of a new status that has this name: the name in lower case with hyphens between the
    /// words, and a number after it while `taken` says the key is used. A name without a letter
    /// or digit gives `status`.
    #[must_use]
    pub fn from_name(name: &str, taken: impl Fn(&str) -> bool) -> Self {
        let mut base = String::new();
        for character in name.chars().flat_map(char::to_lowercase) {
            if character.is_ascii_alphanumeric() {
                base.push(character);
            } else if !base.ends_with('-') && !base.is_empty() {
                base.push('-');
            }
        }
        let mut base = base.trim_matches('-').to_owned();
        if !base.starts_with(|first: char| first.is_ascii_lowercase()) {
            base.insert_str(0, "status-");
        }
        base.truncate(KEY_FROM_NAME_MAX);
        let base = base.trim_end_matches('-').to_owned();
        let mut candidate = base.clone();
        let mut number = 1_u32;
        while taken(&candidate) {
            number += 1;
            candidate = format!("{base}-{number}");
        }
        Self(TechnicalName::trusted(candidate))
    }
}

impl fmt::Display for StatusKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for StatusKey {
    type Err = InvalidTechnicalName;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl NotPersonal for StatusKey {}

/// A status of a workflow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowStatus {
    /// Its key, which never changes.
    pub key: StatusKey,
    /// What it is called.
    pub name: String,
    /// The status group it belongs to.
    pub group: StatusGroup,
}

words! {
    /// What a condition of *Only if …* asks of a field.
    pub enum Test {
        /// The field has a value.
        Filled => "filled",
        /// The field has no value.
        Empty => "empty",
    }
}

words_in_api!(Test as "OnlyIfTest", "What a condition of a status change asks of a field.");

/// One condition of *Only if …*: something that must be true of a field first. It is written in
/// the same words as *Must be filled in*.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Condition {
    /// The field.
    pub field: FieldKey,
    /// What it must be like.
    pub test: Test,
}

/// *Who can do it*: the people who may make a status change. A change that names nobody is for
/// everybody who may change the issue; a rule never gives a right *Change issues* does not.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhoCan {
    /// People who hold one of these roles where the issue is.
    #[serde(default)]
    pub roles: Vec<RoleId>,
    /// People who are in one of these groups.
    #[serde(default)]
    pub groups: Vec<GroupId>,
    /// The people the issue is assigned to.
    #[serde(default)]
    pub assigned: bool,
    /// The person who reported the issue.
    #[serde(default)]
    pub reporter: bool,
}

impl WhoCan {
    /// Whether the change names nobody and so is for everybody who may change the issue.
    #[must_use]
    pub fn is_everybody(&self) -> bool {
        self.roles.is_empty() && self.groups.is_empty() && !self.assigned && !self.reporter
    }
}

/// The rules of a status change.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusRules {
    /// *Who can do it*.
    #[serde(default)]
    pub who: WhoCan,
    /// *Only if …*: every condition must hold.
    #[serde(default)]
    pub only_if: Vec<Condition>,
    /// *Must be filled in*: fields that need a value before the change.
    #[serde(default)]
    pub must_fill: Vec<FieldKey>,
    /// *Ask for*: fields shown in a small form during the change.
    #[serde(default)]
    pub ask_for: Vec<FieldKey>,
}

impl StatusRules {
    /// Every field the rules name, in the order they are written, each once.
    #[must_use]
    pub fn fields(&self) -> Vec<FieldKey> {
        let mut fields: Vec<FieldKey> = Vec::new();
        let named = self
            .only_if
            .iter()
            .map(|condition| condition.field)
            .chain(self.must_fill.iter().copied())
            .chain(self.ask_for.iter().copied());
        for field in named {
            if !fields.contains(&field) {
                fields.push(field);
            }
        }
        fields
    }
}

// Roles, groups and field keys: IDs and words, never a name or a value.
impl NotPersonal for StatusRules {}

/// A status change: from one status to another, with the text of its button and its rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusChange {
    /// The status it starts from.
    pub from: StatusKey,
    /// The status it ends in.
    pub to: StatusKey,
    /// The text of the button (*Start working*).
    pub button: String,
    /// Its rules.
    pub rules: StatusRules,
}

/// The header of a workflow. Its statuses and status changes are rows of their own.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Workflow {
    /// The ID of the workflow.
    #[not_personal]
    pub id: WorkflowId,
    /// What it is called. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub name: String,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// A status of a stored workflow.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct StoredStatus {
    /// The workflow.
    #[not_personal]
    pub workflow: WorkflowId,
    /// The key of the status, which never changes.
    #[not_personal]
    pub key: StatusKey,
    /// What it is called. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub name: String,
    /// The status group it belongs to.
    #[not_personal]
    pub group: StatusGroup,
    /// Its place among the others; the first is where a new issue starts.
    #[not_personal]
    pub position: i64,
}

/// A status change of a stored workflow.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct StoredChange {
    /// The workflow.
    #[not_personal]
    pub workflow: WorkflowId,
    /// The status it starts from.
    #[not_personal]
    pub from: StatusKey,
    /// The status it ends in.
    #[not_personal]
    pub to: StatusKey,
    /// The text of the button. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub button: String,
    /// Its rules: roles, groups and fields by their IDs.
    #[not_personal]
    pub rules: StatusRules,
    /// Its place among the others.
    #[not_personal]
    pub position: i64,
}

/// Where a workflow is used: for a project and everything below it (none for the whole
/// organization) and for one issue type (none for all types).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Entity)]
pub struct WorkflowUse {
    /// The project, with its sub-projects; none for the whole organization.
    #[not_personal]
    pub project: Option<ProjectId>,
    /// The issue type; none for all types.
    #[not_personal]
    pub issue_type: Option<IssueTypeId>,
    /// The workflow used there.
    #[not_personal]
    pub workflow: WorkflowId,
}

/// A place a workflow is used at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkflowPlace {
    /// The project, or the whole organization.
    pub project: Option<ProjectId>,
    /// The issue type, or all types.
    pub issue_type: Option<IssueTypeId>,
}

impl WorkflowUse {
    /// Where the workflow is used.
    #[must_use]
    pub const fn place(&self) -> WorkflowPlace {
        WorkflowPlace {
            project: self.project,
            issue_type: self.issue_type,
        }
    }
}

/// A workflow that is stored: its header and what it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredWorkflow {
    /// The header.
    pub workflow: Workflow,
    /// The statuses and status changes.
    pub content: WorkflowContent,
}

/// The use that counts for a place whose projects from the top down are `chain` and for an issue
/// type (ADR-0031, decision 2): the same walk as for layouts and field levels. `None` when nobody
/// set a workflow, and the starting workflow counts.
#[must_use]
pub fn find<'a>(
    chain: &[ProjectId],
    issue_type: Option<IssueTypeId>,
    uses: &'a [WorkflowUse],
) -> Option<&'a WorkflowUse> {
    nearest_row(
        chain,
        issue_type,
        uses.iter()
            .map(|each| (each.project, each.issue_type, each)),
    )
    .map(|(_, _, found)| found)
}

#[cfg(test)]
mod tests;
