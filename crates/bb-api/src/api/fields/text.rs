//! Saving the long text of an own field of an issue.

use super::{
    ApiError, ApiJson, ApiPath, AuthHandle, ConflictAnswer, FieldId, ForbiddenAnswer, Info,
    InvalidAnswer, IssueItem, LockToken, NotFoundAnswer, NotSignedInAnswer, Response,
    SaveFieldText, Session, State, answer_authorized, body_id, path_id,
};
use crate::api::editing::SaveTextBody;

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
    request_body = SaveTextBody,
    responses(
        (status = 200, description = "The issue as it is now.", body = IssueItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn save_issue_field_text(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((issue, field)): ApiPath<(String, String)>,
    ApiJson(body): ApiJson<SaveTextBody>,
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
