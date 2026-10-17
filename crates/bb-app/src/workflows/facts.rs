//! What the judge is told about the fields of an issue, as it will be after the change.

use std::collections::BTreeSet;

use bb_core::access::FieldLevel;
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{FieldId, IssueTypeId};
use bb_core::workflow::judge::FieldFacts;
use bb_policy::Evaluation;

/// The facts of one issue for one person.
pub(crate) struct IssueFacts<'a> {
    /// What the person may do with each field.
    pub(crate) viewer: &'a Evaluation,
    /// The type of the issue: the levels of a field can differ between types (ADR-0027).
    pub(crate) issue_type: IssueTypeId,
    /// Whether the description has a text.
    pub(crate) description_filled: bool,
    /// Whether the issue is assigned to anybody after the change.
    pub(crate) assigned_filled: bool,
    /// The own fields that have a value after the change.
    pub(crate) own_filled: &'a BTreeSet<FieldId>,
    /// The fields the request carries.
    pub(crate) given: &'a BTreeSet<FieldKey>,
}

impl FieldFacts for IssueFacts<'_> {
    fn level(&self, field: FieldKey) -> FieldLevel {
        self.viewer.field_level_for(Some(self.issue_type), field)
    }

    fn filled(&self, field: FieldKey) -> bool {
        match field {
            FieldKey::BuiltIn(Field::Description) => self.description_filled,
            FieldKey::BuiltIn(Field::AssignedTo) => self.assigned_filled,
            // The others always have a value, and a rule cannot name them (`rule_may_name`).
            FieldKey::BuiltIn(_) => true,
            FieldKey::Own(id) => self.own_filled.contains(&id),
        }
    }

    fn given(&self, field: FieldKey) -> bool {
        self.given.contains(&field)
    }
}
