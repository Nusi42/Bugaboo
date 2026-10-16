//! The own-field routes below `/api/v1/fields` (CF-01, CF-03, CF-04, CF-06, CF-07): the fields a
//! person sees, and making, changing and retiring a field and its choices for those who hold
//! *Manage fields*. A field is retired, never deleted.
//!
//! A field the person may not see is in no answer and answers `resource.not_found` like one that
//! does not exist. Every change names the `version` it looked at. The use cases are in `bb-app`,
//! behind [`AuthApi`](bb_app::auth::AuthApi).

use axum::extract::State;
use axum::response::Response;
use bb_app::fields::{
    AddChoice, ChangeChoice, ChangeField, ChoiceView, CombineChoices, CreateField, FieldPage,
    FieldQuery, FieldTypePage, FieldTypesQuery, FieldUsage, FieldView, FieldsQuery, NewChoice,
    ReorderChoices, SaveFieldText, ScopeView,
};
use bb_core::id::{ChoiceId, FieldId, IssueTypeId, LockToken, ProjectId};
use bb_core::issue::IssueIcon;
use bb_core::own_fields::{ChoiceColor, Edit, FieldFor, FieldType, KindTrait, Look, Rules, Scope};
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized,
};
use super::issues::{IssueItem, IssueTypeRef};
use super::projects::ProjectRef;
use super::{body_id, path_id};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};

/// A choice of a field.
#[derive(Serialize, ToSchema)]
pub(crate) struct FieldChoiceBody {
    /// The ID of the choice.
    id: String,
    /// What it is called.
    name: String,
    /// Its color, as a category token of the theme.
    color: ChoiceColor,
    /// Its icon, if it has one.
    icon: Option<IssueIcon>,
    /// Whether it is retired: not offered for new values, kept on old ones.
    retired: bool,
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
    whole_organization: bool,
    /// The projects the field is limited to that the person sees.
    projects: Vec<ProjectRef>,
    /// The issue types the field is limited to; empty for all.
    issue_types: Vec<IssueTypeRef>,
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
    id: String,
    /// What it is called.
    name: String,
    /// Words that help people fill it in.
    help: String,
    /// What kind of information it holds.
    kind: FieldType,
    /// What it is added to.
    applies_to: FieldFor,
    /// The rules. The browser checks them for instant feedback; the server decides.
    rules: Rules,
    /// The value a new item starts with, in the form of the type; `null` for none.
    #[schema(value_type = Option<Object>)]
    default_value: Option<Value>,
    /// How the value is shown.
    look: Look,
    /// The assistant is never sent the value.
    never_to_assistant: bool,
    /// Whether the field is retired: no new values, old ones stay.
    retired: bool,
    /// Where it is used.
    scope: FieldScopeBody,
    /// The choices of a *Pick one* or *Pick several* field, in order.
    choices: Vec<FieldChoiceBody>,
    /// Whether the person may change the definition (*Manage fields*).
    may_manage: bool,
    /// Counts up with every change. A change is made on the version that was looked at.
    version: i64,
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
    fields: Vec<FieldBody>,
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
    whole_organization: bool,
    /// How many projects the person sees that use the field.
    project_count: u32,
    /// The first of them.
    projects: Vec<ProjectRef>,
    /// The list of projects was cut.
    truncated: bool,
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
    projects: Vec<String>,
    /// The IDs of the issue types the field is limited to; empty for all.
    #[serde(default)]
    issue_types: Vec<String>,
}

impl ScopeInput {
    fn into_scope(self) -> Result<Scope, ApiError> {
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
    name: String,
    /// Its color; the next in turn when left out.
    color: Option<ChoiceColor>,
    /// Its icon.
    icon: Option<IssueIcon>,
}

/// An own field to make.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewFieldBody {
    /// What it is called.
    name: String,
    /// Words that help people fill it in.
    help: Option<String>,
    /// What kind of information it holds.
    kind: FieldType,
    /// What it is added to.
    applies_to: FieldFor,
    /// The rules; none when left out.
    rules: Option<Rules>,
    /// The value a new item starts with, in the form of the type. A *Pick one* or *Pick several*
    /// field gets its default after its choices exist.
    #[schema(value_type = Option<Object>)]
    default_value: Option<Value>,
    /// How the value is shown; the starting look of the type when left out.
    look: Option<Look>,
    /// The assistant is never sent the value.
    never_to_assistant: Option<bool>,
    /// Where it is used; the whole organization and all issue types when left out.
    scope: Option<ScopeInput>,
    /// The choices of a *Pick one* or *Pick several* field.
    choices: Option<Vec<NewFieldChoiceBody>>,
}

/// What to change about an own field: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeFieldBody {
    /// The version the person looked at.
    version: i64,
    /// A new name.
    name: Option<String>,
    /// New help text; an empty text takes it away.
    help: Option<String>,
    /// New rules.
    rules: Option<Rules>,
    /// A new default value; `null` takes it away.
    #[serde(default, deserialize_with = "crate::api::null_or_value")]
    #[schema(value_type = Option<Object>)]
    default_value: Edit<Value>,
    /// A new look.
    look: Option<Look>,
    /// Whether the assistant is never sent the value.
    never_to_assistant: Option<bool>,
    /// A new scope.
    scope: Option<ScopeInput>,
    /// `true` retires the field (no new values; old ones stay), `false` offers it again.
    retired: Option<bool>,
}

/// A choice to add.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AddChoiceBody {
    /// The version of the field the person looked at.
    version: i64,
    /// What it is called.
    name: String,
    /// Its color; the next in turn when left out.
    color: Option<ChoiceColor>,
    /// Its icon.
    icon: Option<IssueIcon>,
}

/// What to change about a choice: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeChoiceBody {
    /// The version of the field the person looked at.
    version: i64,
    /// A new name.
    name: Option<String>,
    /// A new color.
    color: Option<ChoiceColor>,
    /// A new icon; `null` takes it away.
    #[serde(default, deserialize_with = "crate::api::null_or_value")]
    #[schema(value_type = Option<IssueIcon>)]
    icon: Edit<IssueIcon>,
    /// `true` retires the choice, `false` offers it again.
    retired: Option<bool>,
}

/// The new order of the choices.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ReorderChoicesBody {
    /// The version of the field the person looked at.
    version: i64,
    /// The IDs of every choice of the field, in the new order.
    order: Vec<String>,
}

/// The choice that takes the values of another.
#[derive(Deserialize, ToSchema)]
pub(crate) struct CombineChoicesBody {
    /// The version of the field the person looked at.
    version: i64,
    /// The ID of the choice that takes the values.
    into: String,
}

/// Which fields to list.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct FieldsParams {
    /// Only fields for this kind of item.
    applies_to: Option<FieldFor>,
    /// Only the fields used in this project (its ID or short name).
    project: Option<String>,
    /// Also the retired fields.
    retired: Option<bool>,
}

/// One kind of information a field can hold, with what the screens need to know about it.
#[derive(Serialize, ToSchema)]
pub(crate) struct FieldTypeBody {
    /// The kind.
    kind: FieldType,
    /// What is true of it.
    traits: Vec<KindTrait>,
    /// The looks that can show it.
    looks: Vec<Look>,
    /// The look a new field of this kind starts with.
    starting_look: Look,
}

/// The kinds of information a field can hold.
#[derive(Serialize, ToSchema)]
pub(crate) struct FieldTypeListBody {
    /// The kinds, in the order they are offered in.
    kinds: Vec<FieldTypeBody>,
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

/// The kinds of information a field can hold, with what each kind allows.
#[utoipa::path(
    get,
    path = "/api/v1/field-types",
    operation_id = "listFieldTypes",
    tag = "fields",
    responses((status = 200, description = "The kinds of information.", body = FieldTypeListBody), NotSignedInAnswer)
)]
async fn list_field_types(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let page = auth
        .api()?
        .field_types(info, FieldTypesQuery { session_token })
        .await?;
    answer_authorized(&page.map(|page| FieldTypeListBody::from(&page)))
}

/// The own fields the person sees, for the organization or for one project.
#[utoipa::path(
    get,
    path = "/api/v1/fields",
    operation_id = "listFields",
    tag = "fields",
    params(FieldsParams),
    responses((status = 200, description = "The fields.", body = FieldListBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_fields(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<FieldsParams>,
) -> Result<Response, ApiError> {
    let page = auth
        .api()?
        .fields(
            info,
            FieldsQuery {
                session_token,
                applies_to: params.applies_to,
                project: params.project,
                retired: params.retired.unwrap_or(false),
            },
        )
        .await?;
    answer_authorized(&page.map(|page| FieldListBody::from(&page)))
}

/// One own field the person sees.
#[utoipa::path(
    get,
    path = "/api/v1/fields/{field}",
    operation_id = "getField",
    tag = "fields",
    params(("field" = String, Path, description = "The ID of the field.")),
    responses((status = 200, description = "The field.", body = FieldBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_field(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(field): ApiPath<String>,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .field(
            info,
            FieldQuery {
                session_token,
                field: path_id::<FieldId>(&field)?,
            },
        )
        .await?;
    answer_authorized(&found.map(|field| FieldBody::from(&field)))
}

/// Where an own field is used now. Needs the permission *Manage fields*.
#[utoipa::path(
    get,
    path = "/api/v1/fields/{field}/used-by",
    operation_id = "getFieldUsedBy",
    tag = "fields",
    params(("field" = String, Path, description = "The ID of the field.")),
    responses((status = 200, description = "Where the field is used.", body = FieldUsedByBody), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn get_field_used_by(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(field): ApiPath<String>,
) -> Result<Response, ApiError> {
    let usage = auth
        .api()?
        .field_usage(
            info,
            FieldQuery {
                session_token,
                field: path_id::<FieldId>(&field)?,
            },
        )
        .await?;
    answer_authorized(&usage.map(|usage| FieldUsedByBody::from(&usage)))
}

/// Creates an own field. Needs the permission *Manage fields*.
#[utoipa::path(
    post,
    path = "/api/v1/fields",
    operation_id = "createField",
    tag = "fields",
    request_body = NewFieldBody,
    responses(
        (status = 200, description = "The field.", body = FieldBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        ConflictAnswer
    )
)]
async fn create_field(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewFieldBody>,
) -> Result<Response, ApiError> {
    let scope = body.scope.unwrap_or_default().into_scope()?;
    let choices = body
        .choices
        .unwrap_or_default()
        .into_iter()
        .map(|choice| NewChoice {
            name: choice.name,
            color: choice.color,
            icon: choice.icon,
        })
        .collect();
    let made = auth
        .api()?
        .create_field(
            info,
            CreateField {
                session_token,
                name: body.name,
                help: body.help,
                kind: body.kind,
                applies_to: body.applies_to,
                rules: body.rules.unwrap_or_default(),
                default_value: body.default_value,
                look: body.look,
                never_to_assistant: body.never_to_assistant.unwrap_or(false),
                scope,
                choices,
            },
        )
        .await?;
    answer_authorized(&made.map(|field| FieldBody::from(&field)))
}

/// Changes an own field, or retires it (or offers it again). Needs the permission *Manage
/// fields*.
#[utoipa::path(
    patch,
    path = "/api/v1/fields/{field}",
    operation_id = "changeField",
    tag = "fields",
    params(("field" = String, Path, description = "The ID of the field.")),
    request_body = ChangeFieldBody,
    responses(
        (status = 200, description = "The field as it is now.", body = FieldBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_field(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(field): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeFieldBody>,
) -> Result<Response, ApiError> {
    let field = path_id::<FieldId>(&field)?;
    let scope = body.scope.map(ScopeInput::into_scope).transpose()?;
    let changed = auth
        .api()?
        .change_field(
            info,
            ChangeField {
                session_token,
                field,
                version: body.version,
                name: body.name,
                help: body.help,
                rules: body.rules,
                default_value: body.default_value,
                look: body.look,
                never_to_assistant: body.never_to_assistant,
                scope,
                retired: body.retired,
            },
        )
        .await?;
    answer_authorized(&changed.map(|field| FieldBody::from(&field)))
}

/// Adds a choice to a *Pick one* or *Pick several* field. Needs the permission *Manage fields*.
#[utoipa::path(
    post,
    path = "/api/v1/fields/{field}/choices",
    operation_id = "addChoice",
    tag = "fields",
    params(("field" = String, Path, description = "The ID of the field.")),
    request_body = AddChoiceBody,
    responses(
        (status = 200, description = "The field as it is now.", body = FieldBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn add_choice(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(field): ApiPath<String>,
    ApiJson(body): ApiJson<AddChoiceBody>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .add_choice(
            info,
            AddChoice {
                session_token,
                field: path_id::<FieldId>(&field)?,
                version: body.version,
                name: body.name,
                color: body.color,
                icon: body.icon,
            },
        )
        .await?;
    answer_authorized(&changed.map(|field| FieldBody::from(&field)))
}

/// Changes the name, color or icon of a choice, or retires it. Needs the permission *Manage
/// fields*.
#[utoipa::path(
    patch,
    path = "/api/v1/fields/{field}/choices/{choice}",
    operation_id = "changeChoice",
    tag = "fields",
    params(
        ("field" = String, Path, description = "The ID of the field."),
        ("choice" = String, Path, description = "The ID of the choice.")
    ),
    request_body = ChangeChoiceBody,
    responses(
        (status = 200, description = "The field as it is now.", body = FieldBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_choice(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((field, choice)): ApiPath<(String, String)>,
    ApiJson(body): ApiJson<ChangeChoiceBody>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .change_choice(
            info,
            ChangeChoice {
                session_token,
                field: path_id::<FieldId>(&field)?,
                choice: path_id::<ChoiceId>(&choice)?,
                version: body.version,
                name: body.name,
                color: body.color,
                icon: body.icon,
                retired: body.retired,
            },
        )
        .await?;
    answer_authorized(&changed.map(|field| FieldBody::from(&field)))
}

/// Puts the choices of a field in a new order. Needs the permission *Manage fields*.
#[utoipa::path(
    put,
    path = "/api/v1/fields/{field}/choices/order",
    operation_id = "reorderChoices",
    tag = "fields",
    params(("field" = String, Path, description = "The ID of the field.")),
    request_body = ReorderChoicesBody,
    responses(
        (status = 200, description = "The field as it is now.", body = FieldBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn reorder_choices(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(field): ApiPath<String>,
    ApiJson(body): ApiJson<ReorderChoicesBody>,
) -> Result<Response, ApiError> {
    let order = body
        .order
        .iter()
        .map(|text| body_id::<ChoiceId>(text))
        .collect::<Result<Vec<_>, _>>()?;
    let changed = auth
        .api()?
        .reorder_choices(
            info,
            ReorderChoices {
                session_token,
                field: path_id::<FieldId>(&field)?,
                version: body.version,
                order,
            },
        )
        .await?;
    answer_authorized(&changed.map(|field| FieldBody::from(&field)))
}

/// Moves every value of a choice to another choice and retires the first (*Combine with another
/// choice*). Every item it touches gets a record in its history. Needs the permission *Manage
/// fields*.
#[utoipa::path(
    post,
    path = "/api/v1/fields/{field}/choices/{choice}/combine",
    operation_id = "combineChoices",
    tag = "fields",
    params(
        ("field" = String, Path, description = "The ID of the field."),
        ("choice" = String, Path, description = "The ID of the choice that goes away.")
    ),
    request_body = CombineChoicesBody,
    responses(
        (status = 200, description = "The field as it is now.", body = FieldBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn combine_choices(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((field, choice)): ApiPath<(String, String)>,
    ApiJson(body): ApiJson<CombineChoicesBody>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .combine_choices(
            info,
            CombineChoices {
                session_token,
                field: path_id::<FieldId>(&field)?,
                choice: path_id::<ChoiceId>(&choice)?,
                into: body_id::<ChoiceId>(&body.into)?,
                version: body.version,
            },
        )
        .await?;
    answer_authorized(&changed.map(|field| FieldBody::from(&field)))
}

/// The long text of an own field, saved under the edit lock of the person.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SaveFieldTextBody {
    /// The version of the issue the person started from.
    version: i64,
    /// The token the edit lock was taken with.
    lock_token: String,
    /// The text, in Markdown.
    text: String,
}

/// Saves the long text of an own field of an issue, under the edit lock of the person. Needs the
/// right to change the issue and the field; a lock that ran out, a take-over or a change made
/// meanwhile keeps the text as the person's draft.
#[utoipa::path(
    put,
    path = "/api/v1/issues/{issue}/fields/{field}/text",
    operation_id = "saveIssueFieldText",
    tag = "fields",
    params(
        ("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue."),
        ("field" = String, Path, description = "The ID of the long text field.")
    ),
    request_body = SaveFieldTextBody,
    responses(
        (status = 200, description = "The issue as it is now.", body = IssueItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn save_issue_field_text(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((issue, field)): ApiPath<(String, String)>,
    ApiJson(body): ApiJson<SaveFieldTextBody>,
) -> Result<Response, ApiError> {
    let saved = auth
        .api()?
        .save_field_text(
            info,
            SaveFieldText {
                session_token,
                issue,
                field: path_id::<FieldId>(&field)?,
                token: body_id::<LockToken>(&body.lock_token)?,
                version: body.version,
                text: body.text,
            },
        )
        .await?;
    answer_authorized(&saved.map(|issue| IssueItem::from(&issue)))
}

/// The routes of the own fields.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_field_types))
        .routes(routes!(list_fields, create_field))
        .routes(routes!(get_field, change_field))
        .routes(routes!(get_field_used_by))
        .routes(routes!(add_choice))
        .routes(routes!(reorder_choices))
        .routes(routes!(change_choice))
        .routes(routes!(combine_choices))
        .routes(routes!(save_issue_field_text))
}
