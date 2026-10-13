//! What the own-field use cases take and give back: inputs that change something are named after
//! what they do; inputs that only read end in `Query` (ARCH-EVENTS). Everything given back is
//! wrapped in `Authorized<T>` by the use case.
//!
//! A field definition is shown to a person who may see the field in some issue type (or who
//! manages fields); a hidden field is not in any answer, not even its name (CF-06).

use bb_core::id::{ChoiceId, FieldId};
use bb_core::issue::IssueIcon;
use bb_core::own_fields::{ChoiceColor, Edit, FieldFor, FieldType, KindTrait, Look, Rules};
use bb_core::own_fields::{FieldChoice, OwnField, Scope};
use bb_policy::Masked;
use serde::Serialize;
use serde_json::Value;

use crate::issues::IssueTypeRef;
use crate::projects::ProjectRef;

/// A choice to make together with a new field.
#[derive(Clone, Debug)]
pub struct NewChoice {
    /// What it is called, as typed.
    pub name: String,
    /// Its color; the next one in turn when left out.
    pub color: Option<ChoiceColor>,
    /// Its icon, if it has one.
    pub icon: Option<IssueIcon>,
}

/// Make an own field.
pub struct MakeField {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What it is called, as typed.
    pub name: String,
    /// Words that help people fill it in; none when left out.
    pub help: Option<String>,
    /// What kind of information it holds.
    pub kind: FieldType,
    /// What it is added to.
    pub applies_to: FieldFor,
    /// The rules.
    pub rules: Rules,
    /// The value a new item starts with, as JSON; `None` for none.
    pub default_value: Option<Value>,
    /// How the value is shown; the starting look of the type when left out.
    pub look: Option<Look>,
    /// Whether the assistant must never be sent the value.
    pub never_to_assistant: bool,
    /// Where it is used.
    pub scope: Scope,
    /// The choices of a *Pick one* or *Pick several* field.
    pub choices: Vec<NewChoice>,
}

/// Change an own field: what is `Some` is changed.
pub struct ChangeField {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The field.
    pub field: FieldId,
    /// The version the person looked at.
    pub version: i64,
    /// A new name.
    pub name: Option<String>,
    /// New help text; an empty text takes it away.
    pub help: Option<String>,
    /// New rules.
    pub rules: Option<Rules>,
    /// A new default; `Clear` takes it away.
    pub default_value: Edit<Value>,
    /// A new look.
    pub look: Option<Look>,
    /// Whether the assistant must never be sent the value.
    pub never_to_assistant: Option<bool>,
    /// A new scope.
    pub scope: Option<Scope>,
    /// `true` to retire the field (no new values, old ones stay), `false` to offer it again.
    pub retired: Option<bool>,
}

/// Add a choice to a *Pick one* or *Pick several* field.
pub struct AddChoice {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The field.
    pub field: FieldId,
    /// The version of the field the person looked at.
    pub version: i64,
    /// What it is called, as typed.
    pub name: String,
    /// Its color; the next one in turn when left out.
    pub color: Option<ChoiceColor>,
    /// Its icon.
    pub icon: Option<IssueIcon>,
}

/// Change a choice: what is `Some` is changed.
pub struct ChangeChoice {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The field.
    pub field: FieldId,
    /// The choice.
    pub choice: ChoiceId,
    /// The version of the field the person looked at.
    pub version: i64,
    /// A new name.
    pub name: Option<String>,
    /// A new color.
    pub color: Option<ChoiceColor>,
    /// A new icon; `Clear` takes it away.
    pub icon: Edit<IssueIcon>,
    /// `true` to retire the choice, `false` to offer it again.
    pub retired: Option<bool>,
}

/// Put the choices of a field in a new order.
pub struct ReorderChoices {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The field.
    pub field: FieldId,
    /// The version of the field the person looked at.
    pub version: i64,
    /// Every choice of the field, in the new order.
    pub order: Vec<ChoiceId>,
}

/// Move every value of a choice to another choice, and retire the first one (*Combine with
/// another choice*).
pub struct CombineChoices {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The field.
    pub field: FieldId,
    /// The choice that goes away.
    pub choice: ChoiceId,
    /// The choice that takes its values.
    pub into: ChoiceId,
    /// The version of the field the person looked at.
    pub version: i64,
}

/// List the fields a person sees.
pub struct FieldsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Only fields for this kind of item.
    pub applies_to: Option<FieldFor>,
    /// Only fields used in this project (by its ID or short name) and so offered there.
    pub project: Option<String>,
    /// Also the retired fields.
    pub retired: bool,
}

/// One field and where it is used.
pub struct FieldQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The field.
    pub field: FieldId,
}

/// A choice as it is shown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChoiceView {
    /// The ID of the choice.
    pub id: ChoiceId,
    /// What it is called.
    pub name: String,
    /// Its color.
    pub color: ChoiceColor,
    /// Its icon.
    pub icon: Option<IssueIcon>,
    /// Whether it is retired: not offered for new values, kept on old ones.
    pub retired: bool,
}

impl From<&FieldChoice> for ChoiceView {
    fn from(choice: &FieldChoice) -> Self {
        Self {
            id: choice.id,
            name: choice.name.clone(),
            color: choice.color,
            icon: choice.icon,
            retired: choice.retired_at.is_some(),
        }
    }
}

/// Where a field is used, as the person may know it: the projects they see and the issue types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScopeView {
    /// The whole organization.
    pub whole_organization: bool,
    /// The projects the field is limited to that the person sees.
    pub projects: Vec<ProjectRef>,
    /// The issue types the field is limited to; empty for all.
    pub issue_types: Vec<IssueTypeRef>,
}

/// An own field as it is shown.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FieldView {
    /// The ID of the field, which is its key in levels, values and history.
    pub id: FieldId,
    /// What it is called.
    pub name: String,
    /// Words that help people fill it in.
    pub help: String,
    /// What kind of information it holds.
    pub kind: FieldType,
    /// What it is added to.
    pub applies_to: FieldFor,
    /// The rules, which the browser checks too for instant feedback; the server decides.
    pub rules: Rules,
    /// The value a new item starts with.
    pub default_value: Option<Value>,
    /// How the value is shown.
    pub look: Look,
    /// Whether the assistant must never be sent the value.
    pub never_to_assistant: bool,
    /// Whether the field is retired.
    pub retired: bool,
    /// Where it is used.
    pub scope: ScopeView,
    /// The choices, in order; empty for a field without choices.
    pub choices: Vec<ChoiceView>,
    /// Whether the person may change the definition (*Manage fields*).
    pub may_manage: bool,
    /// Counts up with every change.
    pub version: i64,
}

impl Masked for FieldView {}

/// List the kinds of information a field can hold.
pub struct FieldTypesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// One kind of information, with what the screens need to know about it, so they never write it a
/// second time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldTypeInfo {
    /// The kind.
    pub kind: FieldType,
    /// What is true of it.
    pub traits: Vec<KindTrait>,
    /// The looks that can show it.
    pub looks: Vec<Look>,
    /// The look a new field of this kind starts with.
    pub starting_look: Look,
}

impl FieldTypeInfo {
    /// What is known of a kind.
    #[must_use]
    pub fn of(kind: FieldType) -> Self {
        Self {
            kind,
            traits: kind.traits(),
            looks: kind.looks(),
            starting_look: Look::starting(kind),
        }
    }
}

/// The kinds of information a field can hold, in the order they are offered in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldTypePage {
    /// The kinds.
    pub kinds: Vec<FieldTypeInfo>,
}

impl Masked for FieldTypePage {}

/// The fields a person sees.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FieldPage {
    /// The fields, in the order they are offered in.
    pub fields: Vec<FieldView>,
}

impl Masked for FieldPage {}

/// Where an own field is used now, before a change (the field page, *Where it's used*).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FieldUsage {
    /// The field has no limit to projects: it is used wherever its items are.
    pub whole_organization: bool,
    /// How many projects the person sees use the field: where it is limited to, or where a value
    /// is set.
    pub project_count: usize,
    /// The first of those projects, at most `USAGE_PROJECTS_MAX`.
    pub projects: Vec<ProjectRef>,
    /// The list of projects was cut.
    pub truncated: bool,
}

impl Masked for FieldUsage {}

/// An own field with where it is used and its choices, as the store keeps them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Definition {
    /// The field.
    pub field: OwnField,
    /// Where it is used.
    pub scope: Scope,
    /// Its choices, in order.
    pub choices: Vec<FieldChoice>,
}

impl Definition {
    /// The names of the choices a value of a *Pick one* or *Pick several* field holds (an ID, or a
    /// list of them), in the order of the value; an ID that is no choice of the field has no name.
    #[must_use]
    pub fn choice_names(&self, value: &Value) -> Vec<&str> {
        let ids: Vec<&str> = match value {
            Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
            Value::String(text) => vec![text.as_str()],
            _ => Vec::new(),
        };
        ids.into_iter()
            .filter_map(|text| text.parse::<ChoiceId>().ok())
            .filter_map(|id| self.choices.iter().find(|choice| choice.id == id))
            .map(|choice| choice.name.as_str())
            .collect()
    }
}
