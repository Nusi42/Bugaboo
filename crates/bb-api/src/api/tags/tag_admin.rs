//! Listing, previewing, changing and combining tags (TAG-02).

use super::{
    ApiError, ApiJson, ApiPath, ApiQuery, AuthHandle, ChangePreviewBody, ChangeTag, ChangeTagBody,
    CombineBody, CombineTags, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, ListParams,
    Listed, NotFoundAnswer, NotSignedInAnswer, Page, PreviewChangeBody, ProjectId, Response,
    Session, State, TAG_FIELDS, TAGS_LIST, TagChangePreviewQuery, TagId, TagItem, TagsQuery,
    answer, answer_authorized, body_id, cursor_error, path_id,
};

/// The tags of the organization, by name, a page at a time. Needs *Manage labels and tags*.
#[utoipa::path(
    get,
    path = "/api/v1/tags",
    operation_id = "listTags",
    tag = "tags",
    params(ListParams),
    responses((status = 200, description = "The tags.", body = Page<TagItem>), NotSignedInAnswer, ForbiddenAnswer)
)]
pub(super) async fn list_tags(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(TAGS_LIST);
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
    bad_cursor?;
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
pub(super) async fn preview_tag_change(
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
                tag: path_id::<TagId>(&tag)?,
                name: body.name,
                into: body.into.as_deref().map(body_id::<TagId>).transpose()?,
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
pub(super) async fn change_tag(
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
                .map(|text| body_id::<ProjectId>(text))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    let changed = auth
        .api()?
        .change_tag(
            info,
            ChangeTag {
                session_token,
                tag: path_id::<TagId>(&tag)?,
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
pub(super) async fn combine_tags(
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
                tag: path_id::<TagId>(&tag)?,
                version: body.version,
                into: body_id::<TagId>(&body.into)?,
                into_version: body.into_version,
            },
        )
        .await?;
    answer_authorized(&kept.map(|tag| TagItem::from(&tag)))
}
