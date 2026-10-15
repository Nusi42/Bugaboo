//! The routes of tags and labels (TAG-01, TAG-02, TAG-03): the tags and labels of an issue below
//! `/api/v1/issues/{issue}`, the tags to offer below `/api/v1/projects/{project}`, and the tags and
//! labels of the organization below `/api/v1/tags`, `/api/v1/labels` and `/api/v1/label-groups` for
//! the people who hold *Manage labels and tags*.
//!
//! A tag or label on an issue is shown only to a person whose role lets them see the field *Tags*
//! or *Labels* of that issue. Renaming and combining ask for the version of what the person looked
//! at, and a preview (`/preview`) says what they would do to the issues the person sees. The use
//! cases are in `bb-app`, behind [`AuthApi`](bb_app::auth::AuthApi).

use axum::extract::State;
use axum::response::Response;
use bb_app::tags::{
    AddLabel, AddTag, ChangeLabel, ChangeLabelGroup, ChangeTag, ChangedIssue, CombineLabels,
    CombineTags, IssueSorting, IssueSortingQuery, LabelChangePreviewQuery, LabelGroupView,
    LabelRef, LabelView, Labels, LabelsQuery, MakeLabel, MakeLabelGroup, RemoveLabel, RemoveTag,
    TagChangePreview, TagChangePreviewQuery, TagRef, TagSuggestionsQuery, TagView, TagsQuery,
};
use bb_core::ErrorCode;
use bb_core::id::{LabelGroupId, LabelId, ProjectId, TagId};
use bb_core::own_fields::{ChoiceColor, Edit};
use bb_policy::Masked;
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::cursor_error;
use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer, answer_authorized,
};
use super::issues::IssueTypeRef;
use super::previews::PreviewSummaryBody;
use super::projects::ProjectRef;
use super::relations::IssueRef;
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The fields of a tag in the list the people who look after tags read, and which of them the list
/// holds when the request names none.
const TAG_FIELDS: [FieldSpec; 6] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("color"),
    FieldSpec::by_default("whole_organization"),
    FieldSpec::on_request("projects"),
    FieldSpec::by_default("version"),
];
const TAGS_LIST: &str = "tags";

/// A tag as a small reference: what a chip needs.
#[derive(Serialize, ToSchema)]
pub(crate) struct TagRefBody {
    /// The ID of the tag.
    id: String,
    /// What it is called.
    name: String,
    /// Its color, as a category token of the theme.
    color: ChoiceColor,
}

impl From<&TagRef> for TagRefBody {
    fn from(tag: &TagRef) -> Self {
        Self {
            id: tag.id.to_string(),
            name: tag.name.clone(),
            color: tag.color,
        }
    }
}

/// A label as a small reference: what a chip needs.
#[derive(Serialize, ToSchema)]
pub(crate) struct LabelRefBody {
    /// The ID of the label.
    id: String,
    /// What it is called.
    name: String,
    /// Its color, as a category token of the theme.
    color: ChoiceColor,
    /// The group it is in, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<String>,
}

impl From<&LabelRef> for LabelRefBody {
    fn from(label: &LabelRef) -> Self {
        Self {
            id: label.id.to_string(),
            name: label.name.clone(),
            color: label.color,
            group: label.group.map(|group| group.to_string()),
        }
    }
}

/// The tags and labels of one issue, and what the person may do with them. A member that holds a
/// field of the catalogue is left out when the person's level for the field is *Hidden*.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct IssueSortingBody {
    /// The internal ID of the issue.
    #[masked(open)]
    issue: String,
    /// The version of the issue now. A change of a tag or a label counts it up.
    #[masked(open)]
    version: i64,
    /// Its issue type. The levels set for it count for the fields of this issue.
    #[masked(nested, item_type)]
    issue_type: IssueTypeRef,
    /// The tags on it, in the order they were put there.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "tags")]
    tags: Option<Vec<TagRefBody>>,
    /// The labels on it, in the order they were put there.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "labels")]
    labels: Option<Vec<LabelRefBody>>,
    /// Whether the person may put tags on the issue and take them off.
    #[masked(open)]
    may_change_tags: bool,
    /// Whether the person may put labels on the issue and take them off.
    #[masked(open)]
    may_change_labels: bool,
}

impl From<IssueSorting> for IssueSortingBody {
    fn from(sorting: IssueSorting) -> Self {
        Self {
            issue: sorting.issue.to_string(),
            version: sorting.version,
            issue_type: IssueTypeRef::from(&sorting.issue_type),
            tags: Some(sorting.tags.iter().map(TagRefBody::from).collect()),
            labels: Some(sorting.labels.iter().map(LabelRefBody::from).collect()),
            may_change_tags: sorting.may_change_tags,
            may_change_labels: sorting.may_change_labels,
        }
    }
}

/// The tags to offer while a person types.
#[derive(Serialize, ToSchema)]
pub(crate) struct TagSuggestionsBody {
    /// The tags, by name.
    tags: Vec<TagRefBody>,
}

impl Masked for TagSuggestionsBody {}

/// A tag as the people who look after tags see it: the fields that can be asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct TagItem {
    /// The ID of the tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    /// What it is called.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Its color, as a category token of the theme.
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<ChoiceColor>,
    /// Whether it is for every project.
    #[serde(skip_serializing_if = "Option::is_none")]
    whole_organization: Option<bool>,
    /// The projects it is limited to, as far as the person sees them.
    #[serde(skip_serializing_if = "Option::is_none")]
    projects: Option<Vec<ProjectRef>>,
    /// Counts up with every change. A change is made on the version that was looked at.
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<i64>,
}

impl Masked for TagItem {}

impl From<&TagView> for TagItem {
    fn from(tag: &TagView) -> Self {
        Self {
            id: Some(tag.id.to_string()),
            name: Some(tag.name.clone()),
            color: Some(tag.color),
            whole_organization: Some(tag.whole_organization),
            projects: Some(tag.projects.iter().map(ProjectRef::from).collect()),
            version: Some(tag.version),
        }
    }
}

/// One issue that a rename or a combine changes.
#[derive(Serialize, ToSchema)]
pub(crate) struct ChangedIssueBody {
    /// The issue, as the person sees it.
    issue: IssueRef,
    /// The names it has now.
    before: Vec<String>,
    /// The names it will have.
    after: Vec<String>,
}

impl From<&ChangedIssue> for ChangedIssueBody {
    fn from(changed: &ChangedIssue) -> Self {
        Self {
            issue: IssueRef::from(&changed.issue),
            before: changed.before.clone(),
            after: changed.after.clone(),
        }
    }
}

/// What renaming or combining a tag or a label would do to the issues the person sees.
#[derive(Serialize, ToSchema)]
pub(crate) struct ChangePreviewBody {
    /// How many issues change.
    summary: PreviewSummaryBody,
    /// The first of them.
    issues: Vec<ChangedIssueBody>,
    /// The list is cut: there are more issues than it names.
    truncated: bool,
}

impl Masked for ChangePreviewBody {}

impl From<TagChangePreview> for ChangePreviewBody {
    fn from(preview: TagChangePreview) -> Self {
        Self {
            summary: PreviewSummaryBody::from(preview.summary),
            issues: preview.issues.iter().map(ChangedIssueBody::from).collect(),
            truncated: preview.truncated,
        }
    }
}

/// A group of labels.
#[derive(Serialize, ToSchema)]
pub(crate) struct LabelGroupBody {
    /// The ID of the group.
    id: String,
    /// What it is called.
    name: String,
    /// Whether an issue has at most one label of the group.
    pick_one: bool,
    /// Whether it is retired: old issues keep its labels, new ones are not offered them.
    retired: bool,
    /// Counts up with every change.
    version: i64,
}

impl Masked for LabelGroupBody {}

impl From<&LabelGroupView> for LabelGroupBody {
    fn from(group: &LabelGroupView) -> Self {
        Self {
            id: group.id.to_string(),
            name: group.name.clone(),
            pick_one: group.pick_one,
            retired: group.retired,
            version: group.version,
        }
    }
}

/// A label of the organization's list.
#[derive(Serialize, ToSchema)]
pub(crate) struct LabelBody {
    /// The ID of the label.
    id: String,
    /// The group it is in, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    /// What it is called.
    name: String,
    /// Its color, as a category token of the theme.
    color: ChoiceColor,
    /// Whether it is retired: old issues keep it, new ones are not offered it.
    retired: bool,
    /// Counts up with every change.
    version: i64,
}

impl Masked for LabelBody {}

impl From<&LabelView> for LabelBody {
    fn from(label: &LabelView) -> Self {
        Self {
            id: label.id.to_string(),
            group: label.group.map(|group| group.to_string()),
            name: label.name.clone(),
            color: label.color,
            retired: label.retired,
            version: label.version,
        }
    }
}

/// The labels of the organization with their groups.
#[derive(Serialize, ToSchema)]
pub(crate) struct LabelsBody {
    /// The groups, in the order they are offered in.
    groups: Vec<LabelGroupBody>,
    /// The labels, those in a group and those in none.
    labels: Vec<LabelBody>,
    /// Whether the person looks after labels and tags.
    may_manage: bool,
}

impl Masked for LabelsBody {}

impl From<Labels> for LabelsBody {
    fn from(labels: Labels) -> Self {
        Self {
            groups: labels.groups.iter().map(LabelGroupBody::from).collect(),
            labels: labels.labels.iter().map(LabelBody::from).collect(),
            may_manage: labels.may_manage,
        }
    }
}

/// A tag to put on an issue.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewTagBody {
    /// The name of the tag. A name nobody used makes the tag.
    name: String,
}

/// A label to put on an issue.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewIssueLabelBody {
    /// The ID of the label.
    label: String,
}

/// What to change about a tag: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeTagBody {
    /// The version the person looked at.
    version: i64,
    /// A new name. Every issue that has the tag records it.
    name: Option<String>,
    /// A new color.
    color: Option<ChoiceColor>,
    /// The IDs of the projects the tag is limited to from now on; an empty list makes it a tag for
    /// every project.
    projects: Option<Vec<String>>,
}

/// The tag that takes the issues of another.
#[derive(Deserialize, ToSchema)]
pub(crate) struct CombineBody {
    /// The ID of the tag or label that stays.
    into: String,
    /// The version of the tag or label that goes away, as the person looked at it.
    version: i64,
    /// The version of the one that stays, as the person looked at it.
    into_version: i64,
}

/// What to preview: a new name, or the one it would be combined with.
#[derive(Deserialize, ToSchema)]
pub(crate) struct PreviewChangeBody {
    /// The new name, when it is renamed.
    name: Option<String>,
    /// The ID of the tag or label it would be combined with.
    into: Option<String>,
}

/// A group of labels to make.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewLabelGroupBody {
    /// What it is called.
    name: String,
    /// Whether an issue has at most one label of the group.
    pick_one: bool,
}

/// What to change about a group of labels: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeLabelGroupBody {
    /// The version the person looked at.
    version: i64,
    /// A new name.
    name: Option<String>,
    /// Whether an issue has at most one label of the group from now on.
    pick_one: Option<bool>,
    /// `true` retires the group, `false` offers it again.
    retired: Option<bool>,
}

/// A label to make.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewLabelBody {
    /// What it is called.
    name: String,
    /// Its color; the next in turn when left out.
    color: Option<ChoiceColor>,
    /// The ID of the group it goes into.
    group: Option<String>,
}

/// A field that is `Clear` when the request says `null` and `Keep` when it says nothing.
fn null_or_value<'de, D, T>(deserializer: D) -> Result<Edit<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Edit::said)
}

/// What to change about a label: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeLabelBody {
    /// The version the person looked at.
    version: i64,
    /// A new name. Every issue that has the label records it.
    name: Option<String>,
    /// A new color.
    color: Option<ChoiceColor>,
    /// The ID of a new group; `null` takes the label out of its group.
    #[serde(default, deserialize_with = "null_or_value")]
    #[schema(value_type = Option<String>)]
    group: Edit<String>,
    /// `true` retires the label, `false` offers it again.
    retired: Option<bool>,
}

fn id_of<Id: std::str::FromStr>(text: &str, otherwise: ErrorCode) -> Result<Id, ApiError> {
    text.parse().map_err(|_| ApiError::new(otherwise))
}

/// The tags and labels of an issue and what the person may do with them.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/sorting",
    operation_id = "getIssueSorting",
    tag = "tags",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    responses((status = 200, description = "The tags and labels of the issue.", body = IssueSortingBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_issue_sorting(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .issue_sorting(
            info,
            IssueSortingQuery {
                session_token,
                issue,
            },
        )
        .await?;
    answer_authorized(&found.map(IssueSortingBody::from))
}

/// Puts a tag on an issue by its name. A name nobody used makes the tag. Needs the right to change
/// the issue and its field *Tags*.
#[utoipa::path(
    post,
    path = "/api/v1/issues/{issue}/tags",
    operation_id = "addTag",
    tag = "tags",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    request_body = NewTagBody,
    responses(
        (status = 200, description = "The tags and labels of the issue now.", body = IssueSortingBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn add_tag(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiJson(body): ApiJson<NewTagBody>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .add_tag(
            info,
            AddTag {
                session_token,
                issue,
                name: body.name,
            },
        )
        .await?;
    answer_authorized(&changed.map(IssueSortingBody::from))
}

/// Takes a tag off an issue.
#[utoipa::path(
    delete,
    path = "/api/v1/issues/{issue}/tags/{tag}",
    operation_id = "removeTag",
    tag = "tags",
    params(
        ("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue."),
        ("tag" = String, Path, description = "The ID of the tag.")
    ),
    responses(
        (status = 200, description = "The tags and labels of the issue now.", body = IssueSortingBody),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn remove_tag(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((issue, tag)): ApiPath<(String, String)>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .remove_tag(
            info,
            RemoveTag {
                session_token,
                issue,
                tag: id_of::<TagId>(&tag, ErrorCode::NotFound)?,
            },
        )
        .await?;
    answer_authorized(&changed.map(IssueSortingBody::from))
}

/// Puts a label on an issue. In a *Pick one* group the label takes the place of the one the issue
/// has from that group.
#[utoipa::path(
    post,
    path = "/api/v1/issues/{issue}/labels",
    operation_id = "addLabel",
    tag = "tags",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    request_body = NewIssueLabelBody,
    responses(
        (status = 200, description = "The tags and labels of the issue now.", body = IssueSortingBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn add_label(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiJson(body): ApiJson<NewIssueLabelBody>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .add_label(
            info,
            AddLabel {
                session_token,
                issue,
                label: id_of::<LabelId>(&body.label, ErrorCode::InvalidRequest)?,
            },
        )
        .await?;
    answer_authorized(&changed.map(IssueSortingBody::from))
}

/// Takes a label off an issue.
#[utoipa::path(
    delete,
    path = "/api/v1/issues/{issue}/labels/{label}",
    operation_id = "removeLabel",
    tag = "tags",
    params(
        ("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue."),
        ("label" = String, Path, description = "The ID of the label.")
    ),
    responses(
        (status = 200, description = "The tags and labels of the issue now.", body = IssueSortingBody),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn remove_label(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((issue, label)): ApiPath<(String, String)>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .remove_label(
            info,
            RemoveLabel {
                session_token,
                issue,
                label: id_of::<LabelId>(&label, ErrorCode::NotFound)?,
            },
        )
        .await?;
    answer_authorized(&changed.map(IssueSortingBody::from))
}

/// What a person who types in a project is offered: the tags used in projects they see whose name
/// holds what they typed, that may go on an issue of this project.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct SuggestionParams {
    /// What has been typed so far; empty offers the first tags by name.
    #[serde(default)]
    text: String,
    /// The ID of the issue type of the issue the tag goes on, when it is known.
    issue_type: Option<String>,
}

/// The tags to offer while a person types a tag in a project. Only tags used in projects the
/// person sees are offered.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{project}/tag-suggestions",
    operation_id = "listTagSuggestions",
    tag = "tags",
    params(("project" = String, Path, description = "The ID of the project, or its short name."), SuggestionParams),
    responses((status = 200, description = "The tags to offer.", body = TagSuggestionsBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_tag_suggestions(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
    ApiQuery(params): ApiQuery<SuggestionParams>,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .tag_suggestions(
            info,
            TagSuggestionsQuery {
                session_token,
                project,
                text: params.text,
                issue_type: params
                    .issue_type
                    .map(|id| {
                        id.parse()
                            .map_err(|_| ApiError::new(ErrorCode::InvalidRequest))
                    })
                    .transpose()?,
            },
        )
        .await?;
    answer_authorized(&found.map(|found| TagSuggestionsBody {
        tags: found.tags.iter().map(TagRefBody::from).collect(),
    }))
}

/// The tags of the organization, by name, a page at a time. Needs *Manage labels and tags*.
#[utoipa::path(
    get,
    path = "/api/v1/tags",
    operation_id = "listTags",
    tag = "tags",
    params(ListParams),
    responses((status = 200, description = "The tags.", body = Page<TagItem>), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_tags(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let position = params.position(TAGS_LIST);
    let cursor = match &position {
        Ok(position) => position.clone(),
        Err(_) => Some(String::new()),
    };
    let page = auth
        .api()?
        .tags(
            info,
            TagsQuery {
                session_token,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    position?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(TagItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&TAG_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(TAGS_LIST, &list, &fields)?))
}

/// What renaming a tag, or combining it with another, would do to the issues the person sees.
/// Needs *Manage labels and tags*.
#[utoipa::path(
    post,
    path = "/api/v1/tags/{tag}/preview",
    operation_id = "previewTagChange",
    tag = "tags",
    params(("tag" = String, Path, description = "The ID of the tag.")),
    request_body = PreviewChangeBody,
    responses(
        (status = 200, description = "What would change.", body = ChangePreviewBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn preview_tag_change(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(tag): ApiPath<String>,
    ApiJson(body): ApiJson<PreviewChangeBody>,
) -> Result<Response, ApiError> {
    let preview = auth
        .api()?
        .tag_change_preview(
            info,
            TagChangePreviewQuery {
                session_token,
                tag: id_of::<TagId>(&tag, ErrorCode::NotFound)?,
                name: body.name,
                into: body
                    .into
                    .as_deref()
                    .map(|text| id_of::<TagId>(text, ErrorCode::InvalidRequest))
                    .transpose()?,
            },
        )
        .await?;
    answer_authorized(&preview.map(ChangePreviewBody::from))
}

/// Changes a tag: its name (every issue that has it records the new name), its color and the
/// projects it is limited to. Needs *Manage labels and tags*.
#[utoipa::path(
    patch,
    path = "/api/v1/tags/{tag}",
    operation_id = "changeTag",
    tag = "tags",
    params(("tag" = String, Path, description = "The ID of the tag.")),
    request_body = ChangeTagBody,
    responses(
        (status = 200, description = "The tag as it is now.", body = TagItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_tag(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(tag): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeTagBody>,
) -> Result<Response, ApiError> {
    let projects = body
        .projects
        .as_deref()
        .map(|each| {
            each.iter()
                .map(|text| id_of::<ProjectId>(text, ErrorCode::InvalidRequest))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    let changed = auth
        .api()?
        .change_tag(
            info,
            ChangeTag {
                session_token,
                tag: id_of::<TagId>(&tag, ErrorCode::NotFound)?,
                version: body.version,
                name: body.name,
                color: body.color,
                projects,
            },
        )
        .await?;
    answer_authorized(&changed.map(|tag| TagItem::from(&tag)))
}

/// Moves every issue from this tag to another one, then takes this tag away. Needs *Manage labels
/// and tags*.
#[utoipa::path(
    post,
    path = "/api/v1/tags/{tag}/combine",
    operation_id = "combineTags",
    tag = "tags",
    params(("tag" = String, Path, description = "The ID of the tag that goes away.")),
    request_body = CombineBody,
    responses(
        (status = 200, description = "The tag that stays.", body = TagItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn combine_tags(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(tag): ApiPath<String>,
    ApiJson(body): ApiJson<CombineBody>,
) -> Result<Response, ApiError> {
    let kept = auth
        .api()?
        .combine_tags(
            info,
            CombineTags {
                session_token,
                tag: id_of::<TagId>(&tag, ErrorCode::NotFound)?,
                version: body.version,
                into: id_of::<TagId>(&body.into, ErrorCode::InvalidRequest)?,
                into_version: body.into_version,
            },
        )
        .await?;
    answer_authorized(&kept.map(|tag| TagItem::from(&tag)))
}

/// The labels of the organization with their groups. Open to everybody who is signed in: the
/// people who put labels on issues choose from them.
#[utoipa::path(
    get,
    path = "/api/v1/labels",
    operation_id = "listLabels",
    tag = "tags",
    responses((status = 200, description = "The groups and the labels.", body = LabelsBody), NotSignedInAnswer)
)]
async fn list_labels(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let labels = auth
        .api()?
        .labels(info, LabelsQuery { session_token })
        .await?;
    answer_authorized(&labels.map(LabelsBody::from))
}

/// Makes a group of labels. Needs *Manage labels and tags*.
#[utoipa::path(
    post,
    path = "/api/v1/label-groups",
    operation_id = "makeLabelGroup",
    tag = "tags",
    request_body = NewLabelGroupBody,
    responses(
        (status = 200, description = "The group.", body = LabelGroupBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        ConflictAnswer
    )
)]
async fn make_label_group(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewLabelGroupBody>,
) -> Result<Response, ApiError> {
    let made = auth
        .api()?
        .make_label_group(
            info,
            MakeLabelGroup {
                session_token,
                name: body.name,
                pick_one: body.pick_one,
            },
        )
        .await?;
    answer_authorized(&made.map(|group| LabelGroupBody::from(&group)))
}

/// Changes a group of labels. Needs *Manage labels and tags*.
#[utoipa::path(
    patch,
    path = "/api/v1/label-groups/{group}",
    operation_id = "changeLabelGroup",
    tag = "tags",
    params(("group" = String, Path, description = "The ID of the group.")),
    request_body = ChangeLabelGroupBody,
    responses(
        (status = 200, description = "The group as it is now.", body = LabelGroupBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_label_group(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(group): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeLabelGroupBody>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .change_label_group(
            info,
            ChangeLabelGroup {
                session_token,
                group: id_of::<LabelGroupId>(&group, ErrorCode::NotFound)?,
                version: body.version,
                name: body.name,
                pick_one: body.pick_one,
                retired: body.retired,
            },
        )
        .await?;
    answer_authorized(&changed.map(|group| LabelGroupBody::from(&group)))
}

/// Makes a label. Needs *Manage labels and tags*.
#[utoipa::path(
    post,
    path = "/api/v1/labels",
    operation_id = "makeLabel",
    tag = "tags",
    request_body = NewLabelBody,
    responses(
        (status = 200, description = "The label.", body = LabelBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn make_label(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewLabelBody>,
) -> Result<Response, ApiError> {
    let group = body
        .group
        .as_deref()
        .map(|text| id_of::<LabelGroupId>(text, ErrorCode::InvalidRequest))
        .transpose()?;
    let made = auth
        .api()?
        .make_label(
            info,
            MakeLabel {
                session_token,
                name: body.name,
                color: body.color,
                group,
            },
        )
        .await?;
    answer_authorized(&made.map(|label| LabelBody::from(&label)))
}

/// What renaming a label, or combining it with another, would do to the issues the person sees.
/// Needs *Manage labels and tags*.
#[utoipa::path(
    post,
    path = "/api/v1/labels/{label}/preview",
    operation_id = "previewLabelChange",
    tag = "tags",
    params(("label" = String, Path, description = "The ID of the label.")),
    request_body = PreviewChangeBody,
    responses(
        (status = 200, description = "What would change.", body = ChangePreviewBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn preview_label_change(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(label): ApiPath<String>,
    ApiJson(body): ApiJson<PreviewChangeBody>,
) -> Result<Response, ApiError> {
    let preview = auth
        .api()?
        .label_change_preview(
            info,
            LabelChangePreviewQuery {
                session_token,
                label: id_of::<LabelId>(&label, ErrorCode::NotFound)?,
                name: body.name,
                into: body
                    .into
                    .as_deref()
                    .map(|text| id_of::<LabelId>(text, ErrorCode::InvalidRequest))
                    .transpose()?,
            },
        )
        .await?;
    answer_authorized(&preview.map(ChangePreviewBody::from))
}

/// Changes a label: its name (every issue that has it records the new name), color, group and
/// whether it is retired. Needs *Manage labels and tags*.
#[utoipa::path(
    patch,
    path = "/api/v1/labels/{label}",
    operation_id = "changeLabel",
    tag = "tags",
    params(("label" = String, Path, description = "The ID of the label.")),
    request_body = ChangeLabelBody,
    responses(
        (status = 200, description = "The label as it is now.", body = LabelBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_label(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(label): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeLabelBody>,
) -> Result<Response, ApiError> {
    let group = match body.group {
        Edit::Keep => Edit::Keep,
        Edit::Clear => Edit::Clear,
        Edit::Set(text) => Edit::Set(id_of::<LabelGroupId>(&text, ErrorCode::InvalidRequest)?),
    };
    let changed = auth
        .api()?
        .change_label(
            info,
            ChangeLabel {
                session_token,
                label: id_of::<LabelId>(&label, ErrorCode::NotFound)?,
                version: body.version,
                name: body.name,
                color: body.color,
                group,
                retired: body.retired,
            },
        )
        .await?;
    answer_authorized(&changed.map(|label| LabelBody::from(&label)))
}

/// Moves every issue from this label to another one, then takes this label away. Needs *Manage
/// labels and tags*.
#[utoipa::path(
    post,
    path = "/api/v1/labels/{label}/combine",
    operation_id = "combineLabels",
    tag = "tags",
    params(("label" = String, Path, description = "The ID of the label that goes away.")),
    request_body = CombineBody,
    responses(
        (status = 200, description = "The label that stays.", body = LabelBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn combine_labels(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(label): ApiPath<String>,
    ApiJson(body): ApiJson<CombineBody>,
) -> Result<Response, ApiError> {
    let kept = auth
        .api()?
        .combine_labels(
            info,
            CombineLabels {
                session_token,
                label: id_of::<LabelId>(&label, ErrorCode::NotFound)?,
                version: body.version,
                into: id_of::<LabelId>(&body.into, ErrorCode::InvalidRequest)?,
                into_version: body.into_version,
            },
        )
        .await?;
    answer_authorized(&kept.map(|label| LabelBody::from(&label)))
}

/// The routes of tags and labels.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_issue_sorting))
        .routes(routes!(add_tag))
        .routes(routes!(remove_tag))
        .routes(routes!(add_label))
        .routes(routes!(remove_label))
        .routes(routes!(list_tag_suggestions))
        .routes(routes!(list_tags))
        .routes(routes!(preview_tag_change))
        .routes(routes!(change_tag))
        .routes(routes!(combine_tags))
        .routes(routes!(list_labels, make_label))
        .routes(routes!(make_label_group))
        .routes(routes!(change_label_group))
        .routes(routes!(preview_label_change))
        .routes(routes!(change_label))
        .routes(routes!(combine_labels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_a_tag() {
        let tag = TagItem {
            id: Some(String::new()),
            name: Some(String::new()),
            color: Some(ChoiceColor::Cat1),
            whole_organization: Some(true),
            projects: Some(Vec::new()),
            version: Some(1),
        };
        crate::lean::assert_fields_are_the_keys_of(&TAG_FIELDS, &tag);
    }
}
