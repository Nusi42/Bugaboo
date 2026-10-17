//! What a workflow holds, and the rules that decide whether it can be stored (WF-01, WF-02):
//! the statuses, the status changes, the drafts an editor sends, and the check.

use super::{
    CHANGES_MAX, NAME_MAX, RULE_FIELDS_MAX, STATUSES_MAX, StatusChange, StatusKey, StatusRules,
    StoredChange, StoredStatus, WHO_MAX, WorkflowStatus,
};
use crate::fields::{Field, FieldKey};
use crate::id::WorkflowId;
use crate::issue::StatusGroup;

/// A status as an editor sends it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusDraft {
    /// The key of a status the workflow has; none for a new one, which gets a key made from its
    /// name.
    pub key: Option<StatusKey>,
    /// What it is called.
    pub name: String,
    /// The status group it belongs to.
    pub group: StatusGroup,
}

/// A status change as an editor sends it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeDraft {
    /// The status it starts from: a key, or the name of a new status.
    pub from: String,
    /// The status it ends in: a key, or the name of a new status.
    pub to: String,
    /// The text of the button.
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
    /// Two statuses have the same name, so a person could not tell them apart.
    NameTwice,
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
    /// A rule names a built-in field that a rule cannot ask about, or asks for the description.
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

/// Whether a rule may ask for the field in the small form of a status change. The description is
/// long text with a lock of its own, which a status change cannot carry, so asking for it could
/// never be met (it can still be required to be filled in).
#[must_use]
pub const fn ask_may_name(field: FieldKey) -> bool {
    rule_may_name(field) && !matches!(field, FieldKey::BuiltIn(Field::Description))
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
            let name = status.name.trim().to_lowercase();
            if self.statuses[..at]
                .iter()
                .any(|each| each.name.trim().to_lowercase() == name)
            {
                return Err(Problem::NameTwice);
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

    /// The workflow an editor sends: a new status gets a key made from its name (one that is not
    /// used), and a status change names its two statuses by a key or, for a new status, by its
    /// name. The result is checked.
    ///
    /// # Errors
    /// Names the first rule that is broken; a status change that names a status the editor did not
    /// send is [`Problem::UnknownStatus`].
    pub fn from_drafts(statuses: &[StatusDraft], changes: &[ChangeDraft]) -> Result<Self, Problem> {
        let mut taken: Vec<String> = statuses
            .iter()
            .filter_map(|draft| draft.key.as_ref())
            .map(|key| key.as_str().to_owned())
            .collect();
        let mut made = Vec::with_capacity(statuses.len());
        for draft in statuses {
            let key = if let Some(given) = &draft.key {
                given.clone()
            } else {
                let key = StatusKey::from_name(draft.name.trim(), |each| {
                    taken.iter().any(|used| used == each)
                });
                taken.push(key.as_str().to_owned());
                key
            };
            made.push(WorkflowStatus {
                key,
                name: draft.name.trim().to_owned(),
                group: draft.group,
            });
        }
        let given: Vec<&StatusKey> = statuses.iter().filter_map(|d| d.key.as_ref()).collect();
        let named = |text: &str| {
            status_named(&made, text, |status| !given.contains(&&status.key))
                .map(|status| status.key.clone())
        };
        let mut made_changes = Vec::with_capacity(changes.len());
        for draft in changes {
            made_changes.push(StatusChange {
                from: named(&draft.from).ok_or(Problem::UnknownStatus)?,
                to: named(&draft.to).ok_or(Problem::UnknownStatus)?,
                button: draft.button.trim().to_owned(),
                rules: draft.rules.clone(),
            });
        }
        let content = Self {
            statuses: made,
            changes: made_changes,
        };
        content.check()?;
        Ok(content)
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
        if named.iter().any(|field| !rule_may_name(*field))
            || self.ask_for.iter().any(|field| !ask_may_name(*field))
        {
            return Err(Problem::FieldNotAllowed);
        }
        Ok(())
    }
}

/// The status that a text names among `statuses`: the one with that key, else (only among the ones
/// `is_new` accepts) the one with that name, ignoring capitals and spaces at the ends. The editor
/// writes a status that is new by its name because it has no key yet; publishing reads the same
/// words the same way (ADR-0031 amendment 2).
pub fn status_named<'a>(
    statuses: &'a [WorkflowStatus],
    text: &str,
    is_new: impl Fn(&WorkflowStatus) -> bool,
) -> Option<&'a WorkflowStatus> {
    let wanted = text.trim().to_lowercase();
    statuses
        .iter()
        .find(|status| status.key.as_str() == text)
        .or_else(|| {
            statuses
                .iter()
                .find(|status| is_new(status) && status.name.trim().to_lowercase() == wanted)
        })
}
