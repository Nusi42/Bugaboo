//! The labels and tags of one issue: reading, adding and taking off, and the suggestions (TAG-01, TAG-02).

use super::{
    AddLabel, AddTag, ApiError, ApiJson, ApiPath, ApiQuery, AuthHandle, ConflictAnswer,
    Deserialize, ForbiddenAnswer, Info, InvalidAnswer, IssueLabelsAndTagsBody,
    IssueLabelsAndTagsQuery, LabelId, NewIssueLabelBody, NewTagBody, NotFoundAnswer,
    NotSignedInAnswer, RemoveLabel, RemoveTag, Response, Session, State, TagId, TagRefBody,
    TagSuggestionsBody, TagSuggestionsQuery, answer_authorized, body_id, path_id,
};

/// The tags and labels of an issue and what the person may do with them.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/labels-and-tags",
    operation_id = "getIssueLabelsAndTags",
    tag = "tags",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    responses((status = 200, description = "The tags and labels of the issue.", body = IssueLabelsAndTagsBody), NotSignedInAnswer, NotFoundAnswer)
)]
pub(super) async fn get_issue_labels_and_tags(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .issue_labels_and_tags(
            info,
            IssueLabelsAndTagsQuery {
                session_token,
                issue,
            },
        )
        .await?;
    answer_authorized(&found.map(IssueLabelsAndTagsBody::from))
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
        (status = 200, description = "The tags and labels of the issue now.", body = IssueLabelsAndTagsBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn add_tag(
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
    answer_authorized(&changed.map(IssueLabelsAndTagsBody::from))
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
        (status = 200, description = "The tags and labels of the issue now.", body = IssueLabelsAndTagsBody),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn remove_tag(
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
                tag: path_id::<TagId>(&tag)?,
            },
        )
        .await?;
    answer_authorized(&changed.map(IssueLabelsAndTagsBody::from))
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
        (status = 200, description = "The tags and labels of the issue now.", body = IssueLabelsAndTagsBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn add_label(
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
                label: body_id::<LabelId>(&body.label)?,
            },
        )
        .await?;
    answer_authorized(&changed.map(IssueLabelsAndTagsBody::from))
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
        (status = 200, description = "The tags and labels of the issue now.", body = IssueLabelsAndTagsBody),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn remove_label(
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
                label: path_id::<LabelId>(&label)?,
            },
        )
        .await?;
    answer_authorized(&changed.map(IssueLabelsAndTagsBody::from))
}

/// What a person who types in a project is offered: the tags used in projects they see whose name
/// holds what they typed, that may go on an issue of this project.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct SuggestionParams {
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
pub(super) async fn list_tag_suggestions(
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
                issue_type: params.issue_type.as_deref().map(body_id).transpose()?,
            },
        )
        .await?;
    answer_authorized(&found.map(|found| TagSuggestionsBody {
        tags: found.tags.iter().map(TagRefBody::from).collect(),
    }))
}
