//! The layout routes below `/api/v1/layouts` (CFG-01, ISS-02, ADR-0027): which fields a form shows
//! to the person who asks, and the editor of a layout for those who hold *Manage issue types,
//! kinds of relation and layouts*.
//!
//! The form route answers with what the person may see (or fill in): the layout is applied to
//! their roles and to the levels of the fields on the server, so a field the person may not see
//! is in no answer. A layout is saved under the edit lock its editor holds (`/api/v1/locks`), on
//! the version that was looked at. The use cases are in `bb-app`, behind
//! [`AuthApi`](bb_app::auth::AuthApi).

use crate::api::editing::SlotBody;
use axum::extract::State;
use axum::response::Response;
use bb_app::layouts::{
    CreateLayout, DeleteLayout, EditorField, EditorRole, EditorSection, FieldInput, FormLayout,
    FormLayoutQuery, InheritedFrom, LayoutEditor, LayoutEditorQuery, LayoutSlot, SaveLayout,
    SectionInput,
};
use bb_core::ErrorCode;
use bb_core::fields::FieldKey;
use bb_core::id::{IssueTypeId, LayoutId, LockToken, RoleId};
use bb_core::layout::LayoutKind;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::NamedRef;
use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized, done,
};
use super::issues::IssueTypeRef;
use super::projects::ProjectRef;
use super::{body_id, path_id};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};

/// A section of a form as the person who asks sees it.
#[derive(Serialize, ToSchema)]
pub(crate) struct FormSectionBody {
    /// The tab it is under; empty for none.
    tab: String,
    /// Its title; may be empty.
    title: String,
    /// The fields the person sees (or fills in) here, in order, by their keys: the word of a
    /// built-in field (`priority`) or the ID of an own field.
    fields: Vec<String>,
}

/// The layout of a form for the person who asks: only what they may see, or fill in.
#[derive(Serialize, ToSchema)]
pub(crate) struct FormLayoutBody {
    /// The issue page or the new-issue form.
    kind: LayoutKind,
    /// The sections that have at least one field for the person, in order.
    sections: Vec<FormSectionBody>,
}

impl Masked for FormLayoutBody {}

impl From<&FormLayout> for FormLayoutBody {
    fn from(layout: &FormLayout) -> Self {
        Self {
            kind: layout.kind,
            sections: layout
                .sections
                .iter()
                .map(|section| FormSectionBody {
                    tab: section.tab.clone(),
                    title: section.title.clone(),
                    fields: section
                        .fields
                        .iter()
                        .map(|field| field.code().into_owned())
                        .collect(),
                })
                .collect(),
        }
    }
}

/// A field of a layout in the editor.
#[derive(Serialize, ToSchema)]
pub(crate) struct EditorFieldBody {
    /// The key of the field: the word of a built-in field or the ID of an own field.
    field: String,
    /// The name of an own field; a built-in field is named by its word.
    name: Option<String>,
    /// The IDs of the roles the layout shows it to.
    roles: Vec<String>,
    /// The IDs of the roles for which field permissions win: they may not see the field (on the
    /// new-issue form: not change it), whatever the layout says.
    limited_for: Vec<String>,
    /// An own field that must be filled in: it stays on the new-issue form for everybody who can
    /// fill it in.
    required: bool,
}

impl From<&EditorField> for EditorFieldBody {
    fn from(field: &EditorField) -> Self {
        Self {
            field: field.field.code().into_owned(),
            name: field.name.clone(),
            roles: field.roles.iter().map(ToString::to_string).collect(),
            limited_for: field.limited_for.iter().map(ToString::to_string).collect(),
            required: field.required,
        }
    }
}

/// A section of a layout in the editor.
#[derive(Serialize, ToSchema)]
pub(crate) struct EditorSectionBody {
    /// The tab it is under; empty for none.
    tab: String,
    /// Its title; may be empty.
    title: String,
    /// Its fields, in order.
    fields: Vec<EditorFieldBody>,
}

impl From<&EditorSection> for EditorSectionBody {
    fn from(section: &EditorSection) -> Self {
        Self {
            tab: section.tab.clone(),
            title: section.title.clone(),
            fields: section.fields.iter().map(EditorFieldBody::from).collect(),
        }
    }
}

impl From<&EditorRole> for NamedRef {
    fn from(role: &EditorRole) -> Self {
        Self {
            id: role.id.to_string(),
            name: role.name.clone(),
        }
    }
}

/// Where the layout that counts at a place is set, when it is another place.
#[derive(Serialize, ToSchema)]
pub(crate) struct InheritedFromBody {
    /// It is set for the whole organization.
    whole_organization: bool,
    /// The project it is set for, when the person may know it.
    project: Option<ProjectRef>,
    /// The issue type it is for; none for all types.
    issue_type: Option<IssueTypeRef>,
}

impl From<&InheritedFrom> for InheritedFromBody {
    fn from(from: &InheritedFrom) -> Self {
        Self {
            whole_organization: from.whole_organization,
            project: from.project.as_ref().map(ProjectRef::from),
            issue_type: from.issue_type.as_ref().map(IssueTypeRef::from),
        }
    }
}

/// What the editor shows for a place.
#[derive(Serialize, ToSchema)]
pub(crate) struct LayoutEditorBody {
    /// The issue page or the new-issue form.
    kind: LayoutKind,
    /// The project of the place; none for the whole organization.
    project: Option<ProjectRef>,
    /// The issue type of the place; none for all types.
    issue_type: Option<IssueTypeRef>,
    /// The layout set at this place. Without it the place uses the layout it inherits (or the
    /// starting layout), and *Customize* makes one.
    layout: Option<SlotBody>,
    /// Where the layout that counts is set, when it is another place.
    inherited_from: Option<InheritedFromBody>,
    /// The sections, in order.
    sections: Vec<EditorSectionBody>,
    /// The fields that can be added: placeable here and not in a section yet.
    available: Vec<EditorFieldBody>,
    /// The roles that can be ticked, in the order of roles; none when the person may not choose
    /// the roles of a field.
    roles: Vec<NamedRef>,
    /// The person may choose the roles of a field (*Decide what roles see* at the place). When
    /// false, `roles` is empty, no field has a mark, and a save keeps the roles of the fields on
    /// the layout.
    choose_roles: bool,
}

impl Masked for LayoutEditorBody {}

impl From<&LayoutEditor> for LayoutEditorBody {
    fn from(editor: &LayoutEditor) -> Self {
        Self {
            kind: editor.kind,
            project: editor.project.as_ref().map(ProjectRef::from),
            issue_type: editor.issue_type.as_ref().map(IssueTypeRef::from),
            layout: editor.layout.as_ref().map(SlotBody::from),
            inherited_from: editor.inherited_from.as_ref().map(InheritedFromBody::from),
            sections: editor
                .sections
                .iter()
                .map(EditorSectionBody::from)
                .collect(),
            available: editor.available.iter().map(EditorFieldBody::from).collect(),
            roles: editor.roles.iter().map(NamedRef::from).collect(),
            choose_roles: editor.choose_roles,
        }
    }
}

/// Which form to read the layout of.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct FormLayoutParams {
    /// The project, by its ID or its short name.
    project: String,
    /// The issue type of the issue that is shown or made; leave it out while no type is picked.
    issue_type: Option<String>,
    /// The issue page or the new-issue form.
    kind: LayoutKind,
}

/// Which layout to edit.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct LayoutEditorParams {
    /// The project, by its ID or its short name; leave it out for the whole organization.
    project: Option<String>,
    /// The issue type; leave it out for all types.
    issue_type: Option<String>,
    /// The issue page or the new-issue form.
    kind: LayoutKind,
}

/// The layout to make for a place.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewLayoutBody {
    /// The project, by its ID or its short name; leave it out for the whole organization.
    project: Option<String>,
    /// The issue type; leave it out for all types.
    issue_type: Option<String>,
    /// The issue page or the new-issue form.
    kind: LayoutKind,
}

/// A field in a section that is saved.
#[derive(Deserialize, ToSchema)]
pub(crate) struct FieldInputBody {
    /// The key of the field: the word of a built-in field or the ID of an own field.
    field: String,
    /// The IDs of the roles it is shown to.
    roles: Vec<String>,
}

/// A section that is saved.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SectionInputBody {
    /// The tab it is under; empty for none.
    tab: String,
    /// Its title.
    title: String,
    /// Its fields, in order.
    fields: Vec<FieldInputBody>,
}

/// The sections of a layout to save.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SaveLayoutBody {
    /// The version the person looked at.
    version: i64,
    /// The token the edit lock was taken with.
    lock_token: String,
    /// All the sections, in order. They replace the sections that are stored.
    sections: Vec<SectionInputBody>,
}

/// The version of the layout to take away.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct DeleteLayoutParams {
    /// The version the person looked at.
    version: i64,
}

/// The issue type in a request, if one is named.
fn issue_type_in(text: Option<&str>) -> Result<Option<IssueTypeId>, ApiError> {
    text.map(body_id::<IssueTypeId>).transpose()
}

fn section_input(body: &SectionInputBody) -> Result<SectionInput, ApiError> {
    Ok(SectionInput {
        tab: body.tab.clone(),
        title: body.title.clone(),
        fields: body
            .fields
            .iter()
            .map(|field| {
                Ok(FieldInput {
                    field: FieldKey::from_code(&field.field)
                        .ok_or_else(|| ApiError::new(ErrorCode::InvalidRequest))?,
                    roles: field
                        .roles
                        .iter()
                        .map(|text| body_id::<RoleId>(text))
                        .collect::<Result<_, _>>()?,
                })
            })
            .collect::<Result<_, ApiError>>()?,
    })
}

/// Which fields a form shows to the person who asks: the layout of the project and the issue type,
/// applied to the person's roles and to the levels of the fields.
#[utoipa::path(
    get,
    path = "/api/v1/layouts/form",
    operation_id = "getFormLayout",
    tag = "layouts",
    params(FormLayoutParams),
    responses((status = 200, description = "The sections of the form.", body = FormLayoutBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_form_layout(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<FormLayoutParams>,
) -> Result<Response, ApiError> {
    let layout = auth
        .api()?
        .form_layout(
            info,
            FormLayoutQuery {
                session_token,
                project: params.project,
                issue_type: issue_type_in(params.issue_type.as_deref())?,
                kind: params.kind,
            },
        )
        .await?;
    answer_authorized(&layout.map(|layout| FormLayoutBody::from(&layout)))
}

/// The layout of a place for somebody who may change it. Needs the permission *Manage issue
/// types, kinds of relation and layouts*.
#[utoipa::path(
    get,
    path = "/api/v1/layouts/editor",
    operation_id = "getLayoutEditor",
    tag = "layouts",
    params(LayoutEditorParams),
    responses((status = 200, description = "The layout with its roles and marks.", body = LayoutEditorBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn get_layout_editor(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<LayoutEditorParams>,
) -> Result<Response, ApiError> {
    let editor = auth
        .api()?
        .layout_editor(
            info,
            LayoutEditorQuery {
                session_token,
                project: params.project,
                issue_type: issue_type_in(params.issue_type.as_deref())?,
                kind: params.kind,
            },
        )
        .await?;
    answer_authorized(&editor.map(|editor| LayoutEditorBody::from(&editor)))
}

/// Creates the layout of a place as a copy of the one that counts there (*Customize this form*).
/// Needs the permission *Manage issue types, kinds of relation and layouts*.
#[utoipa::path(
    post,
    path = "/api/v1/layouts",
    operation_id = "createLayout",
    tag = "layouts",
    request_body = NewLayoutBody,
    responses((status = 200, description = "The layout of the place.", body = LayoutEditorBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer, ConflictAnswer)
)]
async fn create_layout(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewLayoutBody>,
) -> Result<Response, ApiError> {
    let editor = auth
        .api()?
        .create_layout(
            info,
            CreateLayout {
                session_token,
                project: body.project,
                issue_type: issue_type_in(body.issue_type.as_deref())?,
                kind: body.kind,
            },
        )
        .await?;
    answer_authorized(&editor.map(|editor| LayoutEditorBody::from(&editor)))
}

/// Saves the sections and fields of a layout, under the edit lock the person holds. Needs the
/// permission *Manage issue types, kinds of relation and layouts*.
#[utoipa::path(
    put,
    path = "/api/v1/layouts/{layout}",
    operation_id = "saveLayout",
    tag = "layouts",
    params(("layout" = String, Path, description = "The ID of the layout.")),
    request_body = SaveLayoutBody,
    responses((status = 200, description = "The layout as it is now.", body = LayoutEditorBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer, ConflictAnswer)
)]
async fn save_layout(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(layout): ApiPath<String>,
    ApiJson(body): ApiJson<SaveLayoutBody>,
) -> Result<Response, ApiError> {
    let editor = auth
        .api()?
        .save_layout(
            info,
            SaveLayout {
                session_token,
                layout: path_id::<LayoutId>(&layout)?,
                version: body.version,
                lock_token: body_id::<LockToken>(&body.lock_token)?,
                sections: body
                    .sections
                    .iter()
                    .map(section_input)
                    .collect::<Result<_, _>>()?,
            },
        )
        .await?;
    answer_authorized(&editor.map(|editor| LayoutEditorBody::from(&editor)))
}

/// Takes the layout of a place away: the place uses the layout it inherits again. Needs the
/// permission *Manage issue types, kinds of relation and layouts*.
#[utoipa::path(
    delete,
    path = "/api/v1/layouts/{layout}",
    operation_id = "deleteLayout",
    tag = "layouts",
    params(("layout" = String, Path, description = "The ID of the layout."), DeleteLayoutParams),
    responses((status = 204, description = "The layout is gone."), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer, ConflictAnswer)
)]
async fn delete_layout(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(layout): ApiPath<String>,
    ApiQuery(params): ApiQuery<DeleteLayoutParams>,
) -> Result<Response, ApiError> {
    auth.api()?
        .delete_layout(
            info,
            DeleteLayout {
                session_token,
                layout: path_id::<LayoutId>(&layout)?,
                version: params.version,
            },
        )
        .await?;
    Ok(done())
}

/// The routes of the layouts.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_form_layout))
        .routes(routes!(get_layout_editor))
        .routes(routes!(create_layout))
        .routes(routes!(save_layout, delete_layout))
}

impl From<&LayoutSlot> for SlotBody {
    fn from(slot: &LayoutSlot) -> Self {
        Self {
            id: slot.id.to_string(),
            version: slot.version,
        }
    }
}
