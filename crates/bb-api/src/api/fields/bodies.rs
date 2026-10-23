//! The bodies, queries and inputs of the routes of own fields.

use super::{
    ApiError, ChoiceColor, ChoiceView, Deserialize, Edit, FieldFor, FieldPage, FieldType,
    FieldTypePage, FieldUsage, FieldView, IssueIcon, IssueTypeId, IssueTypeRef, KindTrait, Look,
    Masked, ProjectId, ProjectRef, Rules, Scope, ScopeView, Serialize, ToSchema, Value, body_id,
};

/// A choice of a field.
#[derive(Serialize, ToSchema)]
pub(crate) struct FieldChoiceBody {
    /// The ID of the choice.
    pub(super) id: String,
    /// What it is called.
    pub(super) name: String,
    /// Its color, as a category token of the theme.
    pub(super) color: ChoiceColor,
    /// Its icon, if it has one.
    pub(super) icon: Option<IssueIcon>,
    /// Whether it is retired: not offered for new values, kept on old ones.
    pub(super) retired: bool,
}

impl From<&ChoiceView> for FieldChoiceBody {
    fn from(choice: &ChoiceView) -> Self {
        Self {
            id: choice.id.to_string(),
            name: choice.name.clone(),
            color: choice.color,
            icon: choice.icon,
            retired: choice.retired,
        }
    }
}

/// Where a field is used, as the person may know it.
#[derive(Serialize, ToSchema)]
pub(crate) struct FieldScopeBody {
    /// The field is not limited to projects.
    pub(super) whole_organization: bool,
    /// The projects the field is limited to that the person sees.
    pub(super) projects: Vec<ProjectRef>,
    /// The issue types the field is limited to; empty for all.
    pub(super) issue_types: Vec<IssueTypeRef>,
}

impl From<&ScopeView> for FieldScopeBody {
    fn from(scope: &ScopeView) -> Self {
        Self {
            whole_organization: scope.whole_organization,
            projects: scope.projects.iter().map(ProjectRef::from).collect(),
            issue_types: scope.issue_types.iter().map(IssueTypeRef::from).collect(),
        }
    }
}

/// An own field.
#[derive(Serialize, ToSchema)]
pub(crate) struct FieldBody {
    /// The ID of the field, which is its key in levels, values and history.
    pub(super) id: String,
    /// What it is called.
    pub(super) name: String,
    /// Words that help people fill it in.
    pub(super) help: String,
    /// What kind of information it holds.
    pub(super) kind: FieldType,
    /// What it is added to.
    pub(super) applies_to: FieldFor,
    /// The rules. The browser checks them for instant feedback; the server decides.
    pub(super) rules: Rules,
    /// The value a new item starts with, in the form of the type; `null` for none.
    #[schema(value_type = Option<Object>)]
    pub(super) default_value: Option<Value>,
    /// How the value is shown.
    pub(super) look: Look,
    /// The assistant is never sent the value.
    pub(super) never_to_assistant: bool,
    /// Whether the field is retired: no new values, old ones stay.
    pub(super) retired: bool,
    /// Where it is used.
    pub(super) scope: FieldScopeBody,
    /// The choices of a *Pick one* or *Pick several* field, in order.
    pub(super) choices: Vec<FieldChoiceBody>,
    /// Whether the person may change the definition (*Manage fields*).
    pub(super) may_manage: bool,
    /// Counts up with every change. A change is made on the version that was looked at.
    pub(super) version: i64,
}

impl Masked for FieldBody {}

impl From<&FieldView> for FieldBody {
    fn from(field: &FieldView) -> Self {
        Self {
            id: field.id.to_string(),
            name: field.name.clone(),
            help: field.help.clone(),
            kind: field.kind,
            applies_to: field.applies_to,
            rules: field.rules.clone(),
            default_value: field.default_value.clone(),
            look: field.look,
            never_to_assistant: field.never_to_assistant,
            retired: field.retired,
            scope: FieldScopeBody::from(&field.scope),
            choices: field.choices.iter().map(FieldChoiceBody::from).collect(),
            may_manage: field.may_manage,
            version: field.version,
        }
    }
}

/// The fields a person sees.
#[derive(Serialize, ToSchema)]
pub(crate) struct FieldListBody {
    /// The fields, in the order they are offered in.
    pub(super) fields: Vec<FieldBody>,
}

impl Masked for FieldListBody {}

impl From<&FieldPage> for FieldListBody {
    fn from(page: &FieldPage) -> Self {
        Self {
            fields: page.fields.iter().map(FieldBody::from).collect(),
        }
    }
}

/// Where an own field is used now.
#[derive(Serialize, ToSchema)]
pub(crate) struct FieldUsedByBody {
    /// The field is not limited to projects.
    pub(super) whole_organization: bool,
    /// How many projects the person sees that use the field.
    pub(super) project_count: u32,
    /// The first of them.
    pub(super) projects: Vec<ProjectRef>,
    /// The list of projects was cut.
    pub(super) truncated: bool,
}

impl Masked for FieldUsedByBody {}

impl From<&FieldUsage> for FieldUsedByBody {
    fn from(usage: &FieldUsage) -> Self {
        Self {
            whole_organization: usage.whole_organization,
            project_count: u32::try_from(usage.project_count).unwrap_or(u32::MAX),
            projects: usage.projects.iter().map(ProjectRef::from).collect(),
            truncated: usage.truncated,
        }
    }
}

/// Where a field is used, in a request.
#[derive(Deserialize, ToSchema, Default)]
pub(crate) struct ScopeInput {
    /// The IDs of the projects the field is limited to, with their sub-projects; empty for the
    /// whole organization.
    #[serde(default)]
    pub(super) projects: Vec<String>,
    /// The IDs of the issue types the field is limited to; empty for all.
    #[serde(default)]
    pub(super) issue_types: Vec<String>,
}

impl ScopeInput {
    pub(super) fn into_scope(self) -> Result<Scope, ApiError> {
        Ok(Scope {
            projects: self
                .projects
                .iter()
                .map(|text| body_id::<ProjectId>(text))
                .collect::<Result<_, _>>()?,
            issue_types: self
                .issue_types
                .iter()
                .map(|text| body_id::<IssueTypeId>(text))
                .collect::<Result<_, _>>()?,
        })
    }
}

/// A choice to make with a new field.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewFieldChoiceBody {
    /// What it is called.
    pub(super) name: String,
    /// Its color; the next in turn when left out.
    pub(super) color: Option<ChoiceColor>,
    /// Its icon.
    pub(super) icon: Option<IssueIcon>,
}

/// An own field to make.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewFieldBody {
    /// What it is called.
    pub(super) name: String,
    /// Words that help people fill it in.
    pub(super) help: Option<String>,
    /// What kind of information it holds.
    pub(super) kind: FieldType,
    /// What it is added to.
    pub(super) applies_to: FieldFor,
    /// The rules; none when left out.
    pub(super) rules: Option<Rules>,
    /// The value a new item starts with, in the form of the type. A *Pick one* or *Pick several*
    /// field gets its default after its choices exist.
    #[schema(value_type = Option<Object>)]
    pub(super) default_value: Option<Value>,
    /// How the value is shown; the starting look of the type when left out.
    pub(super) look: Option<Look>,
    /// The assistant is never sent the value.
    pub(super) never_to_assistant: Option<bool>,
    /// Where it is used; the whole organization and all issue types when left out.
    pub(super) scope: Option<ScopeInput>,
    /// The choices of a *Pick one* or *Pick several* field.
    pub(super) choices: Option<Vec<NewFieldChoiceBody>>,
}

/// What to change about an own field: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeFieldBody {
    /// The version the person looked at.
    pub(super) version: i64,
    /// A new name.
    pub(super) name: Option<String>,
    /// New help text; an empty text takes it away.
    pub(super) help: Option<String>,
    /// New rules.
    pub(super) rules: Option<Rules>,
    /// A new default value; `null` takes it away.
    #[serde(default, deserialize_with = "crate::api::null_or_value")]
    #[schema(value_type = Option<Object>)]
    pub(super) default_value: Edit<Value>,
    /// A new look.
    pub(super) look: Option<Look>,
    /// Whether the assistant is never sent the value.
    pub(super) never_to_assistant: Option<bool>,
    /// A new scope.
    pub(super) scope: Option<ScopeInput>,
    /// `true` retires the field (no new values; old ones stay), `false` offers it again.
    pub(super) retired: Option<bool>,
}

/// A choice to add.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AddChoiceBody {
    /// The version of the field the person looked at.
    pub(super) version: i64,
    /// What it is called.
    pub(super) name: String,
    /// Its color; the next in turn when left out.
    pub(super) color: Option<ChoiceColor>,
    /// Its icon.
    pub(super) icon: Option<IssueIcon>,
}

/// What to change about a choice: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeChoiceBody {
    /// The version of the field the person looked at.
    pub(super) version: i64,
    /// A new name.
    pub(super) name: Option<String>,
    /// A new color.
    pub(super) color: Option<ChoiceColor>,
    /// A new icon; `null` takes it away.
    #[serde(default, deserialize_with = "crate::api::null_or_value")]
    #[schema(value_type = Option<IssueIcon>)]
    pub(super) icon: Edit<IssueIcon>,
    /// `true` retires the choice, `false` offers it again.
    pub(super) retired: Option<bool>,
}

/// The new order of the choices.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ReorderChoicesBody {
    /// The version of the field the person looked at.
    pub(super) version: i64,
    /// The IDs of every choice of the field, in the new order.
    pub(super) order: Vec<String>,
}

/// The choice that takes the values of another.
#[derive(Deserialize, ToSchema)]
pub(crate) struct CombineChoicesBody {
    /// The version of the field the person looked at.
    pub(super) version: i64,
    /// The ID of the choice that takes the values.
    pub(super) into: String,
}

/// Which fields to list.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct FieldsParams {
    /// Only fields for this kind of item.
    pub(super) applies_to: Option<FieldFor>,
    /// Only the fields used in this project (its ID or short name).
    pub(super) project: Option<String>,
    /// Also the retired fields.
    pub(super) retired: Option<bool>,
}

/// One kind of information a field can hold, with what the screens need to know about it.
#[derive(Serialize, ToSchema)]
pub(crate) struct FieldTypeBody {
    /// The kind.
    pub(super) kind: FieldType,
    /// What is true of it.
    pub(super) traits: Vec<KindTrait>,
    /// The looks that can show it.
    pub(super) looks: Vec<Look>,
    /// The look a new field of this kind starts with.
    pub(super) starting_look: Look,
}

/// The kinds of information a field can hold.
#[derive(Serialize, ToSchema)]
pub(crate) struct FieldTypeListBody {
    /// The kinds, in the order they are offered in.
    pub(super) kinds: Vec<FieldTypeBody>,
}

impl Masked for FieldTypeListBody {}

impl From<&FieldTypePage> for FieldTypeListBody {
    fn from(page: &FieldTypePage) -> Self {
        Self {
            kinds: page
                .kinds
                .iter()
                .map(|info| FieldTypeBody {
                    kind: info.kind,
                    traits: info.traits.clone(),
                    looks: info.looks.clone(),
                    starting_look: info.starting_look,
                })
                .collect(),
        }
    }
}
