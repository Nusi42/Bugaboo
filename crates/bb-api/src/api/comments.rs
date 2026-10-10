//! The comment routes (ISS-05): comments below `/api/v1/issues/{issue}/comments` and
//! `/api/v1/comments/{comment}`, what the writer may name after `@`, and the *Mentions* block of
//! the start page below `/api/v1/me/mentions`.
//!
//! The text of a comment is Markdown; every answer that holds it also holds `html`, made on the
//! server by `bb-markdown` for the reader (the mark `x-safe-html`). Saving a new text is the save
//! of a long text: it takes the token of the edit lock besides the version (ARCH-VERSIONED).

use std::str::FromStr as _;

use axum::extract::State;
use axum::response::Response;
use bb_app::comments::{
    AddComment, CommentTextView, CommentVersionsQuery, CommentView, DeleteComment, MentionChoice,
    MentionChoicesQuery, MentionItem, MentionsQuery, ReadMentions, RestoreComment, SaveComment,
};
use bb_core::ErrorCode;
use bb_core::comment::MentionTarget;
use bb_core::id::{CommentId, LockToken};
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::cursor_error;
use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer, answer_authorized, done,
};
use super::relations::IssueRef;
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

const VERSIONS_LIST: &str = "comment-versions";
const CHOICES_LIST: &str = "mention-choices";
const MENTIONS_LIST: &str = "mentions";

const TEXT_FIELDS: [FieldSpec; 4] = [
    FieldSpec::by_default("version"),
    FieldSpec::by_default("text"),
    FieldSpec::by_default("html"),
    FieldSpec::by_default("saved_at"),
];
const CHOICE_FIELDS: [FieldSpec; 3] = [
    FieldSpec::by_default("target"),
    FieldSpec::by_default("handle"),
    FieldSpec::by_default("name"),
];
const MENTION_FIELDS: [FieldSpec; 6] = [
    FieldSpec::by_default("comment"),
    FieldSpec::by_default("issue"),
    FieldSpec::by_default("author"),
    FieldSpec::by_default("html"),
    FieldSpec::by_default("at"),
    FieldSpec::by_default("unread"),
];

/// A comment as the reader sees it.
#[derive(Serialize, ToSchema)]
pub(crate) struct CommentItem {
    /// The ID of the comment.
    pub(crate) id: String,
    /// The ID of the issue it is under.
    pub(crate) issue: String,
    /// Who wrote it, as the reader may see people of the kind *Authors*.
    pub(crate) author: PersonRef,
    /// The text, in Markdown.
    pub(crate) text: String,
    /// The text as safe HTML, made on the server for this reader.
    pub(crate) html: String,
    /// When it was written.
    pub(crate) created_at: String,
    /// When its text was last changed, if it was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) edited_at: Option<String>,
    /// Counts up with every change. A change is made on the version that was looked at.
    pub(crate) version: i64,
    /// Whether the reader may change the text (they wrote it).
    pub(crate) may_edit: bool,
    /// Whether the reader may put it into the trash.
    pub(crate) may_delete: bool,
}

impl Masked for CommentItem {}

impl From<&CommentView> for CommentItem {
    fn from(comment: &CommentView) -> Self {
        Self {
            id: comment.id.to_string(),
            issue: comment.issue.to_string(),
            author: comment.author.clone(),
            text: comment.text.clone(),
            html: comment.html.clone(),
            created_at: comment.created_at.to_string(),
            edited_at: comment.edited_at.map(|at| at.to_string()),
            version: comment.version,
            may_edit: comment.may_edit,
            may_delete: comment.may_delete,
        }
    }
}

/// One text a comment had.
#[derive(Serialize, ToSchema)]
pub(crate) struct CommentTextItem {
    /// The version of the comment this text belonged to.
    version: i64,
    /// The text, in Markdown.
    text: String,
    /// The text as safe HTML, made on the server for this reader.
    html: String,
    /// When it was saved.
    saved_at: String,
}

impl Masked for CommentTextItem {}

impl From<&CommentTextView> for CommentTextItem {
    fn from(view: &CommentTextView) -> Self {
        Self {
            version: view.version,
            text: view.text.clone(),
            html: view.html.clone(),
            saved_at: view.saved_at.to_string(),
        }
    }
}

/// One thing the writer may name after `@`.
#[derive(Serialize, ToSchema)]
pub(crate) struct MentionChoiceItem {
    /// What it is.
    target: MentionTarget,
    /// What is written after `@`.
    handle: String,
    /// The name as people read it.
    name: String,
}

impl Masked for MentionChoiceItem {}

impl From<&MentionChoice> for MentionChoiceItem {
    fn from(choice: &MentionChoice) -> Self {
        Self {
            target: choice.target,
            handle: choice.handle.clone(),
            name: choice.name.clone(),
        }
    }
}

/// A comment that mentions the person.
#[derive(Serialize, ToSchema)]
pub(crate) struct MentionedComment {
    /// The ID of the comment.
    comment: String,
    /// The issue it is under.
    issue: IssueRef,
    /// Who wrote it, as the person may see people of the kind *Authors*.
    author: PersonRef,
    /// The text as safe HTML, made on the server for this reader.
    html: String,
    /// When it was written.
    at: String,
    /// Whether the person has not looked at it under *Mentions* yet.
    unread: bool,
}

impl Masked for MentionedComment {}

impl From<&MentionItem> for MentionedComment {
    fn from(item: &MentionItem) -> Self {
        Self {
            comment: item.comment.to_string(),
            issue: IssueRef::from(&item.issue),
            author: item.author.clone(),
            html: item.html.clone(),
            at: item.at.to_string(),
            unread: item.unread,
        }
    }
}

/// A comment to write.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewCommentBody {
    /// The text, in Markdown.
    text: String,
}

/// The new text of a comment, saved under the lock the editor holds.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SaveCommentBody {
    /// The version of the comment the person started from.
    version: i64,
    /// The token the lock on the text was taken with.
    lock_token: String,
    /// The new text, in Markdown.
    text: String,
}

/// The mentions to mark as read.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ReadMentionsBody {
    /// The IDs of the comments.
    comments: Vec<String>,
}

/// What the writer has typed after `@`.
#[derive(Deserialize, utoipa::IntoParams)]
pub(crate) struct ChoiceSearch {
    /// The start of a name; every choice when left out.
    search: Option<String>,
}

fn comment_id(text: &str) -> Result<CommentId, ApiError> {
    text.parse().map_err(|_| ApiError::new(ErrorCode::NotFound))
}

/// Writes a comment under an issue. Needs the permission *Comment* and the right to see the issue.
#[utoipa::path(
    post,
    path = "/api/v1/issues/{issue}/comments",
    operation_id = "addComment",
    tag = "comments",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    request_body = NewCommentBody,
    responses(
        (status = 200, description = "The comment.", body = CommentItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn add_comment(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiJson(body): ApiJson<NewCommentBody>,
) -> Result<Response, ApiError> {
    let added = auth
        .api()?
        .add_comment(
            info,
            AddComment {
                session_token,
                issue,
                text: body.text,
            },
        )
        .await?;
    answer_authorized(&added.map(|comment| CommentItem::from(&comment)))
}

/// Saves the new text of a comment. Only the author changes a text. The editor holds the lock on
/// the text (`edit-locks`, the part `text`) and sends its token and the version it started from;
/// every earlier text is kept.
#[utoipa::path(
    patch,
    path = "/api/v1/comments/{comment}",
    operation_id = "saveComment",
    tag = "comments",
    params(("comment" = String, Path, description = "The ID of the comment.")),
    request_body = SaveCommentBody,
    responses(
        (status = 200, description = "The comment as it is now.", body = CommentItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn save_comment(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(comment): ApiPath<String>,
    ApiJson(body): ApiJson<SaveCommentBody>,
) -> Result<Response, ApiError> {
    let comment = comment_id(&comment)?;
    let lock_token = LockToken::from_str(&body.lock_token)
        .map_err(|_| ApiError::new(ErrorCode::InvalidRequest))?;
    let saved = auth
        .api()?
        .save_comment(
            info,
            SaveComment {
                session_token,
                comment,
                version: body.version,
                lock_token,
                text: body.text,
            },
        )
        .await?;
    answer_authorized(&saved.map(|comment| CommentItem::from(&comment)))
}

/// Puts a comment into the trash (*Undo* takes it out again). The author may, and whoever may
/// change issues.
#[utoipa::path(
    delete,
    path = "/api/v1/comments/{comment}",
    operation_id = "deleteComment",
    tag = "comments",
    params(("comment" = String, Path, description = "The ID of the comment.")),
    responses(
        (status = 204, description = "The comment is in the trash."),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn delete_comment(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(comment): ApiPath<String>,
) -> Result<Response, ApiError> {
    auth.api()?
        .delete_comment(
            info,
            DeleteComment {
                session_token,
                comment: comment_id(&comment)?,
            },
        )
        .await?;
    Ok(done())
}

/// Takes a comment out of the trash (*Undo*).
#[utoipa::path(
    post,
    path = "/api/v1/comments/{comment}/restore",
    operation_id = "restoreComment",
    tag = "comments",
    params(("comment" = String, Path, description = "The ID of the comment.")),
    responses(
        (status = 200, description = "The comment, out of the trash.", body = CommentItem),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn restore_comment(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(comment): ApiPath<String>,
) -> Result<Response, ApiError> {
    let restored = auth
        .api()?
        .restore_comment(
            info,
            RestoreComment {
                session_token,
                comment: comment_id(&comment)?,
            },
        )
        .await?;
    answer_authorized(&restored.map(|comment| CommentItem::from(&comment)))
}

/// Every text a comment had, the current one last, a page at a time. Whoever may see the comment
/// may see its earlier texts.
#[utoipa::path(
    get,
    path = "/api/v1/comments/{comment}/versions",
    operation_id = "listCommentVersions",
    tag = "comments",
    params(("comment" = String, Path, description = "The ID of the comment."), ListParams),
    responses((status = 200, description = "The texts, oldest first.", body = Page<CommentTextItem>), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_comment_versions(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(comment): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let position = params.position(VERSIONS_LIST);
    let cursor = match &position {
        Ok(position) => position.clone(),
        Err(_) => Some(String::new()),
    };
    let page = auth
        .api()?
        .comment_versions(
            info,
            CommentVersionsQuery {
                session_token,
                comment: comment_id(&comment)?,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    position?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(CommentTextItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&TEXT_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(VERSIONS_LIST, &list, &fields)?))
}

/// What the writer of a comment under this issue may name after `@`: people and groups for someone
/// who sees names, groups and roles for someone who sees roles only, nobody for someone who sees no
/// people. Needs the permission *Comment*.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/mention-choices",
    operation_id = "listMentionChoices",
    tag = "comments",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue."), ListParams, ChoiceSearch),
    responses((status = 200, description = "What may be named.", body = Page<MentionChoiceItem>), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn list_mention_choices(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
    ApiQuery(find): ApiQuery<ChoiceSearch>,
) -> Result<Response, ApiError> {
    let position = params.position(CHOICES_LIST);
    position
        .as_ref()
        .map_err(|_| ApiError::new(ErrorCode::BadCursor))?;
    let found = auth
        .api()?
        .mention_choices(
            info,
            MentionChoicesQuery {
                session_token,
                issue,
                search: find.search,
                limit: params.limit(),
            },
        )
        .await?;
    let list = found.map(|choices| Listed {
        items: choices.items.iter().map(MentionChoiceItem::from).collect(),
        next: choices.next_cursor,
    });
    let fields = params.fields_readable(&CHOICE_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(CHOICES_LIST, &list, &fields)?))
}

/// The comments that mention the person, unread first and then the newest: the *Mentions* block of
/// the start page. Only comments under issues the person may see.
#[utoipa::path(
    get,
    path = "/api/v1/me/mentions",
    operation_id = "listMentions",
    tag = "comments",
    params(ListParams),
    responses((status = 200, description = "The comments.", body = Page<MentionedComment>), NotSignedInAnswer)
)]
async fn list_mentions(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let position = params.position(MENTIONS_LIST);
    position
        .as_ref()
        .map_err(|_| ApiError::new(ErrorCode::BadCursor))?;
    let found = auth
        .api()?
        .mentions(
            info,
            MentionsQuery {
                session_token,
                limit: params.limit(),
            },
        )
        .await?;
    let list = found.map(|block| Listed {
        items: block.items.iter().map(MentionedComment::from).collect(),
        next: block.next_cursor,
    });
    let fields = params.fields_readable(&MENTION_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(MENTIONS_LIST, &list, &fields)?))
}

/// Marks the mentions of the person in these comments as read.
#[utoipa::path(
    post,
    path = "/api/v1/me/mentions/read",
    operation_id = "readMentions",
    tag = "comments",
    request_body = ReadMentionsBody,
    responses((status = 204, description = "They are marked."), InvalidAnswer, NotSignedInAnswer)
)]
async fn read_mentions(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<ReadMentionsBody>,
) -> Result<Response, ApiError> {
    let comments = body
        .comments
        .iter()
        .map(|text| {
            text.parse::<CommentId>()
                .map_err(|_| ApiError::new(ErrorCode::InvalidRequest))
        })
        .collect::<Result<Vec<_>, _>>()?;
    auth.api()?
        .read_mentions(
            info,
            ReadMentions {
                session_token,
                comments,
            },
        )
        .await?;
    Ok(done())
}

/// The routes of comments and mentions.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(add_comment))
        .routes(routes!(save_comment, delete_comment))
        .routes(routes!(restore_comment))
        .routes(routes!(list_comment_versions))
        .routes(routes!(list_mention_choices))
        .routes(routes!(list_mentions))
        .routes(routes!(read_mentions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_field_tables_name_exactly_the_fields_of_the_items() {
        let text = CommentTextItem {
            version: 1,
            text: String::new(),
            html: String::new(),
            saved_at: String::new(),
        };
        crate::lean::assert_fields_are_the_keys_of(&TEXT_FIELDS, &text);
        let choice = MentionChoiceItem {
            target: MentionTarget::Person,
            handle: String::new(),
            name: String::new(),
        };
        crate::lean::assert_fields_are_the_keys_of(&CHOICE_FIELDS, &choice);
    }
}
