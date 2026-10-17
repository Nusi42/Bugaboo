//! The routes of files (ATT-01 … ATT-04, ATT-07, ADR-0032): uploads in parts below
//! `/api/v1/uploads`, the files of an issue below `/api/v1/issues/{issue}`, attachments below
//! `/api/v1/attachments`, and the downloads below `/api/v1/files/{file}`.
//!
//! An upload starts with the name and the size, goes on in parts that each say where they start
//! (an interrupted upload asks where the server stands and goes on from there), and is finished.
//! Only the route of a part has a body limit above the small one of the API
//! ([`PART_BYTES`], M1-F30). A download is a signed link for one person and five minutes; the use
//! case judges access again every time, and the answer carries the headers that make a browser
//! save the file and never run it ([`download_headers`]).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse as _, Response};
use bb_app::files::{
    AttachmentView, CancelUploadQuery, ChunkReader, DeleteAttachment, DownloadQuery, FileBody,
    FileRules, FileRulesQuery, FinishUpload, IssueFilesQuery, RestoreAttachment, SendPartQuery,
    StartUploadQuery, UploadStateQuery, UploadView,
};
use bb_core::error_code::PROBLEM_CONTENT_TYPE;
use bb_core::files::PART_BYTES;
use bb_core::id::{AttachmentId, CommentId, UploadId};
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::openapi::response::{Response as ResponseDescription, ResponseBuilder};
use utoipa::openapi::{ContentBuilder, Ref, RefOr};
use utoipa::{IntoResponses, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer, answer_authorized, done, problem_answers,
};
use super::{VERSION_PATH, body_id, path_id};
use crate::errors::{ApiBytes, ApiError, ApiJson, ApiPath, ApiQuery};
use crate::headers::DOWNLOAD_POLICY;
use crate::lean::{FieldSpec, ListParams, Listed, Page};

const FILES_LIST: &str = "issue-files";

const ITEM_FIELDS: [FieldSpec; 12] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("issue"),
    FieldSpec::by_default("comment"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("media_type"),
    FieldSpec::by_default("size"),
    FieldSpec::by_default("added_by"),
    FieldSpec::by_default("added_at"),
    FieldSpec::by_default("version"),
    FieldSpec::by_default("download_url"),
    FieldSpec::by_default("preview_url"),
    FieldSpec::by_default("may_delete"),
];

problem_answers! {
    /// A file over the limit, or a part over the limit of a part.
    TooBigAnswer => ("413", "The file is bigger than the project allows, or the part is bigger than a part may be (file.too_big, request.too_large)."),
    /// A type the organization does not accept.
    BlockedAnswer => ("415", "The organization does not accept this type of file (file.type_blocked)."),
    /// A part out of order, or an upload that is not complete.
    OutOfOrderAnswer => ("409", "The part does not start where the file stands, or the upload is not complete yet (file.wrong_part, file.incomplete)."),
}

/// The address of a signed link. The route of the download is `/api/v1/files/{file}`.
fn url_of(link: &str) -> String {
    format!("{VERSION_PATH}/files/{link}")
}

/// A file on an issue or a comment, as the reader sees it.
#[derive(Serialize, ToSchema)]
pub(crate) struct AttachmentItem {
    /// The ID of the attachment.
    id: String,
    /// The issue it belongs to.
    issue: String,
    /// The comment it was added to, if it was.
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    /// The name of the file.
    name: String,
    /// The real type of the file, read from its content.
    media_type: String,
    /// The size in bytes.
    size: u64,
    /// Who added it, as the reader may see people of the kind *Authors*.
    added_by: PersonRef,
    /// When it was added.
    added_at: String,
    /// Counts up with every change.
    version: i64,
    /// The signed link that downloads the file: good for five minutes and for this reader only,
    /// and judged again when it is used.
    download_url: String,
    /// The signed link of the preview, while the file has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    preview_url: Option<String>,
    /// Whether the reader may put it into the trash.
    may_delete: bool,
}

impl Masked for AttachmentItem {}

impl From<&AttachmentView> for AttachmentItem {
    fn from(view: &AttachmentView) -> Self {
        Self {
            id: view.id.to_string(),
            issue: view.issue.to_string(),
            comment: view.comment.map(|comment| comment.to_string()),
            name: view.name.clone(),
            media_type: view.media_type.clone(),
            size: view.size,
            added_by: view.added_by.clone(),
            added_at: view.added_at.to_string(),
            version: view.version,
            download_url: url_of(&view.download_link),
            preview_url: view.preview_link.as_deref().map(url_of),
            may_delete: view.may_delete,
        }
    }
}

/// An upload that is going on.
#[derive(Serialize, ToSchema)]
pub(crate) struct UploadItem {
    /// The ID of the upload.
    id: String,
    /// The size the file will have.
    size: u64,
    /// Where the next part starts: the bytes the server has.
    offset: u64,
    /// The most bytes one part may have.
    part_bytes: u64,
}

impl Masked for UploadItem {}

impl From<&UploadView> for UploadItem {
    fn from(view: &UploadView) -> Self {
        Self {
            id: view.id.to_string(),
            size: view.size,
            offset: view.offset,
            part_bytes: view.part_bytes,
        }
    }
}

/// What the reader may add to an issue and the limits of its project.
#[derive(Serialize, ToSchema)]
pub(crate) struct FileRulesItem {
    /// Whether the reader may add files to the issue itself.
    may_add: bool,
    /// The most bytes one file may have in the project of the issue.
    limit_bytes: u64,
}

impl Masked for FileRulesItem {}

impl From<&FileRules> for FileRulesItem {
    fn from(rules: &FileRules) -> Self {
        Self {
            may_add: rules.may_add,
            limit_bytes: rules.limit_bytes,
        }
    }
}

/// The file to upload.
#[derive(Deserialize, ToSchema)]
pub(crate) struct StartUploadBody {
    /// The name of the file as the device has it. A path is cut off.
    name: String,
    /// The size in bytes.
    size: u64,
    /// The comment the file is for; left out for a file of the issue itself.
    comment: Option<String>,
}

/// Where a part starts.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct PartQuery {
    /// The number of bytes the server has: the part starts there. The answer of the upload says it.
    offset: u64,
}

/// The bytes of a part, as they are.
pub(crate) struct PartBytes;

impl utoipa::PartialSchema for PartBytes {
    fn schema() -> RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::schema::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::String)
            .format(Some(utoipa::openapi::schema::SchemaFormat::KnownFormat(
                utoipa::openapi::schema::KnownFormat::Binary,
            )))
            .into()
    }
}

impl ToSchema for PartBytes {}

/// Starts an upload of a file to an issue, or to a comment under it. Needs the right to change the
/// issue (a Guest: an issue they reported), or, for a comment, to have written it. A file that is
/// too big for the project or of a type the organization never accepts is refused here, before
/// any part is sent.
#[utoipa::path(
    post,
    path = "/api/v1/issues/{issue}/uploads",
    operation_id = "startUpload",
    tag = "files",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    request_body = StartUploadBody,
    responses(
        (status = 200, description = "The upload.", body = UploadItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        OutOfOrderAnswer,
        TooBigAnswer,
        BlockedAnswer
    )
)]
async fn start_upload(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiJson(body): ApiJson<StartUploadBody>,
) -> Result<Response, ApiError> {
    let comment = body
        .comment
        .as_deref()
        .map(body_id::<CommentId>)
        .transpose()?;
    let started = auth
        .api()?
        .start_upload(
            info,
            StartUploadQuery {
                session_token,
                issue,
                comment,
                name: body.name,
                size: body.size,
            },
        )
        .await?;
    answer_authorized(&started.map(|upload| UploadItem::from(&upload)))
}

/// How far an upload is. After a lost connection the client asks this and sends the next part from
/// `offset`.
#[utoipa::path(
    get,
    path = "/api/v1/uploads/{upload}",
    operation_id = "getUpload",
    tag = "files",
    params(("upload" = String, Path, description = "The ID of the upload.")),
    responses((status = 200, description = "The upload.", body = UploadItem), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn get_upload(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(upload): ApiPath<String>,
) -> Result<Response, ApiError> {
    let state = auth
        .api()?
        .upload_state(
            info,
            UploadStateQuery {
                session_token,
                upload: path_id::<UploadId>(&upload)?,
            },
        )
        .await?;
    answer_authorized(&state.map(|upload| UploadItem::from(&upload)))
}

/// Sends the next part of an upload. The body is the bytes of the part, at most `part_bytes`; the
/// query says where the part starts, which must be the number of bytes the server has.
#[utoipa::path(
    put,
    path = "/api/v1/uploads/{upload}",
    operation_id = "sendUploadPart",
    tag = "files",
    params(("upload" = String, Path, description = "The ID of the upload."), PartQuery),
    request_body(content = PartBytes, content_type = "application/octet-stream"),
    responses(
        (status = 200, description = "The upload, with the bytes the server has now.", body = UploadItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        OutOfOrderAnswer,
        TooBigAnswer,
        BlockedAnswer
    )
)]
async fn send_upload_part(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(upload): ApiPath<String>,
    ApiQuery(part): ApiQuery<PartQuery>,
    ApiBytes(bytes): ApiBytes,
) -> Result<Response, ApiError> {
    let sent = auth
        .api()?
        .send_part(
            info,
            SendPartQuery {
                session_token,
                upload: path_id::<UploadId>(&upload)?,
                offset: part.offset,
                bytes: bytes.to_vec(),
            },
        )
        .await?;
    answer_authorized(&sent.map(|upload| UploadItem::from(&upload)))
}

/// Throws an unfinished upload away.
#[utoipa::path(
    delete,
    path = "/api/v1/uploads/{upload}",
    operation_id = "cancelUpload",
    tag = "files",
    params(("upload" = String, Path, description = "The ID of the upload.")),
    responses((status = 204, description = "The upload is gone."), NotSignedInAnswer, NotFoundAnswer)
)]
async fn cancel_upload(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(upload): ApiPath<String>,
) -> Result<Response, ApiError> {
    auth.api()?
        .cancel_upload(
            info,
            CancelUploadQuery {
                session_token,
                upload: path_id::<UploadId>(&upload)?,
            },
        )
        .await?;
    Ok(done())
}

/// Finishes an upload: the whole file is checked, and it becomes an attachment of the issue or the
/// comment. A file that is stored already is not stored again.
#[utoipa::path(
    post,
    path = "/api/v1/uploads/{upload}/finish",
    operation_id = "finishUpload",
    tag = "files",
    params(("upload" = String, Path, description = "The ID of the upload.")),
    responses(
        (status = 200, description = "The attachment.", body = AttachmentItem),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        OutOfOrderAnswer,
        TooBigAnswer,
        BlockedAnswer
    )
)]
async fn finish_upload(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(upload): ApiPath<String>,
) -> Result<Response, ApiError> {
    let finished = auth
        .api()?
        .finish_upload(
            info,
            FinishUpload {
                session_token,
                upload: path_id::<UploadId>(&upload)?,
            },
        )
        .await?;
    answer_authorized(&finished.map(|attachment| AttachmentItem::from(&attachment)))
}

/// The files of an issue and of the comments under it, oldest first. Each has the signed links of
/// the file and of its preview for the reader.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/files",
    operation_id = "listIssueFiles",
    tag = "files",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue."), ListParams),
    responses((status = 200, description = "The files.", body = Page<AttachmentItem>), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_issue_files(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    params.position(FILES_LIST)?;
    let found = auth
        .api()?
        .issue_files(
            info,
            IssueFilesQuery {
                session_token,
                issue,
            },
        )
        .await?;
    let list = found.map(|files| Listed {
        items: files.items.iter().map(AttachmentItem::from).collect(),
        next: files.next_cursor,
    });
    let fields = params.fields_readable(&ITEM_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(FILES_LIST, &list, &fields)?))
}

/// What the reader may add to the issue, and the limits of its project: how many megabytes a file
/// may have, the types that are never accepted and the size of a part.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/file-rules",
    operation_id = "getFileRules",
    tag = "files",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    responses((status = 200, description = "The rules.", body = FileRulesItem), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_file_rules(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
) -> Result<Response, ApiError> {
    let rules = auth
        .api()?
        .file_rules(
            info,
            FileRulesQuery {
                session_token,
                issue,
            },
        )
        .await?;
    answer_authorized(&rules.map(|rules| FileRulesItem::from(&rules)))
}

/// Puts a file into the trash (*Undo* takes it out again). The person who added it may, and whoever
/// may change issues.
#[utoipa::path(
    delete,
    path = "/api/v1/attachments/{attachment}",
    operation_id = "deleteAttachment",
    tag = "files",
    params(("attachment" = String, Path, description = "The ID of the attachment.")),
    responses((status = 204, description = "The file is in the trash."), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn delete_attachment(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(attachment): ApiPath<String>,
) -> Result<Response, ApiError> {
    auth.api()?
        .delete_attachment(
            info,
            DeleteAttachment {
                session_token,
                attachment: path_id::<AttachmentId>(&attachment)?,
            },
        )
        .await?;
    Ok(done())
}

/// Takes a file out of the trash (*Undo*).
#[utoipa::path(
    post,
    path = "/api/v1/attachments/{attachment}/restore",
    operation_id = "restoreAttachment",
    tag = "files",
    params(("attachment" = String, Path, description = "The ID of the attachment.")),
    responses((status = 200, description = "The attachment, out of the trash.", body = AttachmentItem), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn restore_attachment(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(attachment): ApiPath<String>,
) -> Result<Response, ApiError> {
    let restored = auth
        .api()?
        .restore_attachment(
            info,
            RestoreAttachment {
                session_token,
                attachment: path_id::<AttachmentId>(&attachment)?,
            },
        )
        .await?;
    answer_authorized(&restored.map(|attachment| AttachmentItem::from(&attachment)))
}

/// The pieces of a file as the body of an answer, read one after the other, so a big file is never
/// in memory as a whole. A read that fails ends the body after the error.
fn body_of(file: FileBody) -> Body {
    let pieces = futures_util::stream::unfold(
        Some(file.reader),
        |reader: Option<Box<dyn ChunkReader>>| async move {
            let mut reader = reader?;
            match reader.next_chunk().await {
                Ok(Some(chunk)) => Some((Ok(Bytes::from(chunk)), Some(reader))),
                Ok(None) => None,
                Err(error) => Some((Err(error), None)),
            }
        },
    );
    Body::from_stream(pieces)
}

/// The value of `Content-Disposition` for a file: always `attachment`, so a browser saves it and
/// never shows it, with the name as a plain fallback and in its full form (RFC 6266, RFC 8187).
fn content_disposition(name: &str) -> HeaderValue {
    let plain: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '.' | '_' | '~' | ' ')
            {
                character
            } else {
                '_'
            }
        })
        .collect();
    let mut full = String::new();
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            full.push(char::from(byte));
        } else {
            // Writing to a string cannot fail.
            let _ = write!(full, "%{byte:02X}");
        }
    }
    // Both parts are ASCII letters, digits and the few marks above, so this is always a value.
    HeaderValue::from_str(&format!(
        "attachment; filename=\"{plain}\"; filename*=UTF-8''{full}"
    ))
    .unwrap_or_else(|_| HeaderValue::from_static("attachment"))
}

/// The headers of every download and preview (ADR-0032 §3, SEC-HEADERS): saved and not shown
/// (`Content-Disposition: attachment`), the type never guessed (`nosniff`), and a policy that
/// allows the content nothing at all, not even to run as a page of its own (`sandbox`). The
/// other security headers of the table (`crate::headers`) are added to every answer as well.
fn download_headers(name: &str, media_type: &str, size: u64) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_DISPOSITION, content_disposition(name));
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(media_type)
            .unwrap_or_else(|_| HeaderValue::from_static(bb_core::files::UNKNOWN_MEDIA_TYPE)),
    );
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(size));
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(DOWNLOAD_POLICY),
    );
    headers
}

/// Downloads a file, or shows its preview, through a signed link: one made for this person, good
/// for five minutes. The use case judges again that the person sees the issue and the comment, so
/// a link that is passed on or kept after access ended opens nothing. The file is sent as an
/// attachment with `nosniff` and a policy that allows nothing; SVG and HTML are only downloaded.
#[utoipa::path(
    get,
    path = "/api/v1/files/{file}",
    operation_id = "downloadFile",
    tag = "files",
    params(("file" = String, Path, description = "The signed address of a file, as the list of files gives it.")),
    responses(
        (status = 200, description = "The bytes of the file, or the PNG of its preview.", content_type = "application/octet-stream", body = PartBytes),
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn download_file(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(file): ApiPath<String>,
) -> Result<Response, ApiError> {
    let file = auth
        .api()?
        .download(
            info,
            DownloadQuery {
                session_token,
                link: file,
            },
        )
        .await?;
    let headers = download_headers(&file.name, &file.media_type, file.body.size);
    Ok((StatusCode::OK, headers, body_of(file.body)).into_response())
}

/// The routes of files.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    // Only the address of an upload takes a body above the small limit of the API, and only as big
    // as a part (M1-F30).
    let uploads = OpenApiRouter::new()
        .routes(routes!(get_upload, send_upload_part, cancel_upload))
        .layer(DefaultBodyLimit::max(PART_BYTES));
    OpenApiRouter::new()
        .routes(routes!(start_upload))
        .merge(uploads)
        .routes(routes!(finish_upload))
        .routes(routes!(list_issue_files))
        .routes(routes!(get_file_rules))
        .routes(routes!(delete_attachment))
        .routes(routes!(restore_attachment))
        .routes(routes!(download_file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec_13_a_name_is_sent_as_an_attachment_with_a_plain_fallback_and_its_full_form() {
        let value = content_disposition("Bericht \u{fc}ber \"Q3\".pdf");
        assert_eq!(
            value.to_str().unwrap(),
            "attachment; filename=\"Bericht _ber _Q3_.pdf\"; \
             filename*=UTF-8''Bericht%20%C3%BCber%20%22Q3%22.pdf"
        );
        // A name cannot end the value or start a second header.
        let hostile = content_disposition("a\"; filename=\"b\r\nX-Evil: 1");
        let text = hostile.to_str().unwrap();
        assert!(text.starts_with("attachment; filename=\""));
        assert!(!text.contains(['\r', '\n']));
        assert_eq!(text.matches("filename=").count(), 1);
    }

    #[test]
    fn att_02_every_download_is_saved_never_shown_and_allowed_nothing() {
        let headers = download_headers("page.html", "application/octet-stream", 12);
        assert!(
            headers[header::CONTENT_DISPOSITION]
                .to_str()
                .unwrap()
                .starts_with("attachment;")
        );
        assert_eq!(headers["x-content-type-options"], "nosniff");
        assert_eq!(headers[header::CONTENT_SECURITY_POLICY], DOWNLOAD_POLICY);
        assert_eq!(headers[header::CONTENT_LENGTH], "12");
    }

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_the_item() {
        let item = AttachmentItem {
            id: String::new(),
            issue: String::new(),
            comment: Some(String::new()),
            name: String::new(),
            media_type: String::new(),
            size: 0,
            added_by: PersonRef::Someone,
            added_at: String::new(),
            version: 1,
            download_url: String::new(),
            preview_url: Some(String::new()),
            may_delete: true,
        };
        crate::lean::assert_fields_are_the_keys_of(&ITEM_FIELDS, &item);
    }
}
