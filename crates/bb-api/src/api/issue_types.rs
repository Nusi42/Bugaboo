//! The issue-type routes below `/api/v1/issue-types` (ISS-02): the list for everybody who is
//! signed in, and making and changing a type for those who hold *Manage issue types, kinds of
//! relation and layouts*. A type is retired, never deleted.

use axum::extract::State;
use axum::response::Response;
use bb_app::issues::{ChangeIssueType, CreateIssueType, IssueTypeView, IssueTypesQuery};
use bb_core::id::IssueTypeId;
use bb_core::issue::IssueIcon;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::{PlaceRef, cursor_error};
use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer, answer_authorized,
};
use super::offers::OfferProject;
use super::path_id;
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The name of the list, so its cursors are its own.
const ISSUE_TYPES_LIST: &str = "issue-types";

/// The fields of an issue type in a list that can be asked for: all of them are by default, a type
/// is small.
const ISSUE_TYPE_FIELDS: [FieldSpec; 10] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("icon"),
    FieldSpec::by_default("retired"),
    FieldSpec::by_default("created_in"),
    FieldSpec::by_default("may_manage"),
    FieldSpec::by_default("version"),
    FieldSpec::on_request("offered_here"),
    FieldSpec::on_request("own_offer"),
    FieldSpec::on_request("may_change_offer"),
];

/// An issue type: the fields that can be asked for.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct IssueTypeItem {
    /// The ID of the type.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    id: Option<String>,
    /// What it is called.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    name: Option<String>,
    /// Its icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    icon: Option<IssueIcon>,
    /// Whether it is retired: old issues keep it, new ones cannot have it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    retired: Option<bool>,
    /// Where it was made: the whole organization or a project. It is looked after where it was
    /// made.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(nested)]
    created_in: Option<PlaceRef>,
    /// Whether the person who asks looks after it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    may_manage: Option<bool>,
    /// Counts up with every change. A change is made on the version that was looked at.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    version: Option<i64>,
    /// Whether the project the list is asked about offers it. Only when the request names a
    /// `project` and asks for the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    offered_here: Option<bool>,
    /// The row that says so at exactly that project; left out while the project follows the place
    /// above it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    own_offer: Option<bool>,
    /// Whether the person who asks may change that for the project.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    may_change_offer: Option<bool>,
}

impl From<&IssueTypeView> for IssueTypeItem {
    fn from(issue_type: &IssueTypeView) -> Self {
        Self {
            id: Some(issue_type.id.to_string()),
            name: Some(issue_type.name.clone()),
            icon: Some(issue_type.icon),
            retired: Some(issue_type.retired),
            created_in: Some(PlaceRef::from(issue_type.created_in.as_ref())),
            may_manage: Some(issue_type.may_manage),
            version: Some(issue_type.version),
            offered_here: issue_type.here.as_ref().map(|here| here.offered),
            own_offer: issue_type.here.as_ref().and_then(|here| here.own),
            may_change_offer: issue_type.here.as_ref().map(|here| here.may_change),
        }
    }
}

/// An issue type to make.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewIssueTypeBody {
    /// What it is called.
    name: String,
    /// Its icon.
    icon: IssueIcon,
    /// The project it is made in, by its ID or short name; left out for the whole organization.
    project: Option<String>,
}

/// What to change about an issue type: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeIssueTypeBody {
    /// The version the person looked at.
    version: i64,
    /// A new name.
    name: Option<String>,
    /// A new icon.
    icon: Option<IssueIcon>,
    /// `true` retires the type (no new issues of it), `false` offers it again.
    retired: Option<bool>,
}

/// The issue types in the order they are offered in, a page at a time, retired ones too. Open to
/// everybody who is signed in. With `project`, each type can say whether that project offers it.
#[utoipa::path(
    get,
    path = "/api/v1/issue-types",
    operation_id = "listIssueTypes",
    tag = "issue-types",
    params(ListParams, OfferProject),
    responses((status = 200, description = "The issue types.", body = Page<IssueTypeItem>), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_issue_types(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
    ApiQuery(asked): ApiQuery<OfferProject>,
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(ISSUE_TYPES_LIST);
    let page = auth
        .api()?
        .issue_types(
            info,
            IssueTypesQuery {
                session_token,
                cursor,
                limit: params.limit(),
                project: asked.project,
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(IssueTypeItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&ISSUE_TYPE_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(
        ISSUE_TYPES_LIST,
        &list,
        &fields,
    )?))
}

/// Creates an issue type for the whole organization or in a project. Needs the permission *Manage
/// issue types, kinds of relation and layouts* where it is made.
#[utoipa::path(
    post,
    path = "/api/v1/issue-types",
    operation_id = "createIssueType",
    tag = "issue-types",
    request_body = NewIssueTypeBody,
    responses(
        (status = 200, description = "The issue type.", body = IssueTypeItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        ConflictAnswer
    )
)]
async fn create_issue_type(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewIssueTypeBody>,
) -> Result<Response, ApiError> {
    let made = auth
        .api()?
        .create_issue_type(
            info,
            CreateIssueType {
                session_token,
                project: body.project,
                name: body.name,
                icon: body.icon,
            },
        )
        .await?;
    answer_authorized(&made.map(|issue_type| IssueTypeItem::from(&issue_type)))
}

/// Changes the name or the icon of an issue type, or retires it (or offers it again). Needs the
/// permission *Manage issue types, kinds of relation and layouts* where the type was made.
#[utoipa::path(
    patch,
    path = "/api/v1/issue-types/{issue_type}",
    operation_id = "changeIssueType",
    tag = "issue-types",
    params(("issue_type" = String, Path, description = "The ID of the issue type.")),
    request_body = ChangeIssueTypeBody,
    responses(
        (status = 200, description = "The issue type as it is now.", body = IssueTypeItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_issue_type(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue_type): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeIssueTypeBody>,
) -> Result<Response, ApiError> {
    let issue_type: IssueTypeId = path_id(&issue_type)?;
    let changed = auth
        .api()?
        .change_issue_type(
            info,
            ChangeIssueType {
                session_token,
                issue_type,
                version: body.version,
                name: body.name,
                icon: body.icon,
                retired: body.retired,
            },
        )
        .await?;
    answer_authorized(&changed.map(|issue_type| IssueTypeItem::from(&issue_type)))
}

/// The routes of the issue types.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_issue_types, create_issue_type))
        .routes(routes!(change_issue_type))
}
