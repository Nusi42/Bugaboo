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

use crate::fields::{Field, FieldKey};
use crate::id::{GroupId, IssueTypeId, ProjectId, RoleId, WorkflowId};
use crate::issue::StatusGroup;
use crate::lookup::nearest_row;
use crate::personal::{Entity, NotPersonal};
use crate::technical::{InvalidTechnicalName, TechnicalName};

pub mod judge;
pub mod starter;

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

words_in_api!(Test, "What a condition of a status change asks of a field.");

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

/// What a workflow holds: its statuses, the first of which is where a new issue starts, and the
/// status changes between them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkflowContent {
    /// The statuses, in order.
    pub statuses: Vec<WorkflowStatus>,
    /// The status changes, in order.
    pub changes: Vec<StatusChange>,
}

/// Why a workflow cannot be stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// More statuses, status changes or names in a rule than a workflow may have.
    TooBig,
    /// A workflow needs at least one status.
    NoStatus,
    /// Two statuses have the same key.
    KeyTwice,
    /// A name of a workflow, a status or a button is empty.
    NameEmpty,
    /// A name of a workflow, a status or a button is longer than [`NAME_MAX`] characters.
    NameTooLong,
    /// A status change names a status the workflow does not have.
    UnknownStatus,
    /// A status change ends where it starts.
    SameStatus,
    /// Two status changes have the same start and end.
    ChangeTwice,
    /// A rule names a built-in field that a rule cannot ask about.
    FieldNotAllowed,
}

/// Whether a rule may name the field. A rule asks whether a field has a value, so it needs a field
/// an issue may lack: the description, the people it is assigned to, and every own field. The
/// others always have a value.
#[must_use]
pub const fn rule_may_name(field: FieldKey) -> bool {
    match field {
        FieldKey::Own(_) => true,
        FieldKey::BuiltIn(built_in) => matches!(built_in, Field::Description | Field::AssignedTo),
    }
}

fn name_problem(name: &str) -> Option<Problem> {
    let length = name.trim().chars().count();
    if length == 0 {
        Some(Problem::NameEmpty)
    } else if length > NAME_MAX {
        Some(Problem::NameTooLong)
    } else {
        None
    }
}

/// Checks the name of a workflow.
///
/// # Errors
/// Names the rule that is broken.
pub fn check_name(name: &str) -> Result<(), Problem> {
    name_problem(name).map_or(Ok(()), Err)
}

impl WorkflowContent {
    /// The key of the status a new issue starts in.
    #[must_use]
    pub fn initial(&self) -> Option<&StatusKey> {
        self.statuses.first().map(|status| &status.key)
    }

    /// The status with this key.
    #[must_use]
    pub fn status(&self, key: &StatusKey) -> Option<&WorkflowStatus> {
        self.statuses.iter().find(|status| status.key == *key)
    }

    /// The status change from one status to another.
    #[must_use]
    pub fn change(&self, from: &StatusKey, to: &StatusKey) -> Option<&StatusChange> {
        self.changes
            .iter()
            .find(|change| change.from == *from && change.to == *to)
    }

    /// The status changes that start in this status, in order.
    pub fn changes_from<'a>(
        &'a self,
        from: &'a StatusKey,
    ) -> impl Iterator<Item = &'a StatusChange> {
        self.changes
            .iter()
            .filter(move |change| change.from == *from)
    }

    /// Whether the workflow can be stored. Own fields in the rules are judged by the caller, who
    /// knows which of them exist.
    ///
    /// # Errors
    /// Names the first rule that is broken.
    pub fn check(&self) -> Result<(), Problem> {
        if self.statuses.is_empty() {
            return Err(Problem::NoStatus);
        }
        if self.statuses.len() > STATUSES_MAX || self.changes.len() > CHANGES_MAX {
            return Err(Problem::TooBig);
        }
        for (at, status) in self.statuses.iter().enumerate() {
            if let Some(problem) = name_problem(&status.name) {
                return Err(problem);
            }
            if self.statuses[..at]
                .iter()
                .any(|each| each.key == status.key)
            {
                return Err(Problem::KeyTwice);
            }
        }
        for (at, change) in self.changes.iter().enumerate() {
            if let Some(problem) = name_problem(&change.button) {
                return Err(problem);
            }
            if self.status(&change.from).is_none() || self.status(&change.to).is_none() {
                return Err(Problem::UnknownStatus);
            }
            if change.from == change.to {
                return Err(Problem::SameStatus);
            }
            if self.changes[..at]
                .iter()
                .any(|each| each.from == change.from && each.to == change.to)
            {
                return Err(Problem::ChangeTwice);
            }
            change.rules.check()?;
        }
        Ok(())
    }

    /// The rows to store for this workflow.
    #[must_use]
    pub fn rows(&self, workflow: WorkflowId) -> (Vec<StoredStatus>, Vec<StoredChange>) {
        let statuses = (0_i64..)
            .zip(&self.statuses)
            .map(|(position, status)| StoredStatus {
                workflow,
                key: status.key.clone(),
                name: status.name.clone(),
                group: status.group,
                position,
            })
            .collect();
        let changes = (0_i64..)
            .zip(&self.changes)
            .map(|(position, change)| StoredChange {
                workflow,
                from: change.from.clone(),
                to: change.to.clone(),
                button: change.button.clone(),
                rules: change.rules.clone(),
                position,
            })
            .collect();
        (statuses, changes)
    }

    /// The workflow the stored rows make. Rows of one workflow only.
    #[must_use]
    pub fn from_rows(statuses: &[StoredStatus], changes: &[StoredChange]) -> Self {
        let mut statuses: Vec<&StoredStatus> = statuses.iter().collect();
        statuses.sort_by_key(|status| status.position);
        let mut changes: Vec<&StoredChange> = changes.iter().collect();
        changes.sort_by_key(|change| change.position);
        Self {
            statuses: statuses
                .into_iter()
                .map(|status| WorkflowStatus {
                    key: status.key.clone(),
                    name: status.name.clone(),
                    group: status.group,
                })
                .collect(),
            changes: changes
                .into_iter()
                .map(|change| StatusChange {
                    from: change.from.clone(),
                    to: change.to.clone(),
                    button: change.button.clone(),
                    rules: change.rules.clone(),
                })
                .collect(),
        }
    }

    /// The keys of the statuses that `after` no longer has.
    #[must_use]
    pub fn dropped_by(&self, after: &Self) -> Vec<StatusKey> {
        self.statuses
            .iter()
            .filter(|status| after.status(&status.key).is_none())
            .map(|status| status.key.clone())
            .collect()
    }
}

impl StatusRules {
    /// Whether the rules can be stored.
    ///
    /// # Errors
    /// Names the first rule that is broken.
    pub fn check(&self) -> Result<(), Problem> {
        let named = self.fields();
        if self.only_if.len() > RULE_FIELDS_MAX
            || self.must_fill.len() > RULE_FIELDS_MAX
            || self.ask_for.len() > RULE_FIELDS_MAX
            || self.who.roles.len() + self.who.groups.len() > WHO_MAX
        {
            return Err(Problem::TooBig);
        }
        if named.iter().any(|field| !rule_may_name(*field)) {
            return Err(Problem::FieldNotAllowed);
        }
        Ok(())
    }
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
