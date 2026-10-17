//! The two sides of a changed field, worded for the reader: the same words in the history of an
//! issue and in a notification, so a person is a `PersonRef` (and *Someone* stays *Someone*) in both.

use bb_app::history::{ChangeValue, FieldChangeView};
use bb_core::own_fields::FieldType;
use bb_policy::PersonRef;
use serde_json::Value;

use super::projects::ProjectRef;
use super::relations::IssueRef;

/// The places in an item where one side of a change goes.
#[derive(Default)]
pub(super) struct Side {
    pub(super) text: Option<String>,
    pub(super) value: Option<Value>,
    pub(super) names: Option<Vec<String>>,
    pub(super) people: Option<Vec<PersonRef>>,
    pub(super) issue: Option<IssueRef>,
    pub(super) project: Option<ProjectRef>,
    pub(super) kind: Option<FieldType>,
}

impl Side {
    /// Where a value of a change goes in an item.
    fn of(value: Option<&ChangeValue>) -> Self {
        let mut side = Self::default();
        match value {
            Some(ChangeValue::Text { text }) => side.text = Some(text.clone()),
            Some(ChangeValue::Typed { kind, value }) => {
                side.kind = Some(*kind);
                side.value = Some(value.clone());
            }
            Some(ChangeValue::Names { names }) => side.names = Some(names.clone()),
            Some(ChangeValue::People { people }) => side.people = Some(people.clone()),
            Some(ChangeValue::Issue { issue }) => side.issue = Some(IssueRef::from(issue)),
            Some(ChangeValue::Project { project }) => {
                side.project = Some(ProjectRef::from(project));
            }
            None => {}
        }
        side
    }
}

/// The side before and the side after of a changed field.
pub(super) fn sides_of(change: &FieldChangeView) -> (Side, Side) {
    (
        Side::of(change.before.as_ref()),
        Side::of(change.after.as_ref()),
    )
}
