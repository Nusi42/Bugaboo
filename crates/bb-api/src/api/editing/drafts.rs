//! Drafts and the check of versions.

use super::{
    ApiError, ApiJson, ApiQuery, AuthHandle, CheckVersionsBody, CheckVersionsQuery, ConflictAnswer,
    DraftBody, DraftSlotBody, DraftTargetBody, DraftText, ForbiddenAnswer, Info, InvalidAnswer,
    NotFoundAnswer, NotSignedInAnswer, ReadDraftQuery, RecordRefBody, Response, SaveDraftBody,
    SaveDraftQuery, Session, State, ThrowAwayDraftQuery, VersionsBody, answer_authorized, done,
};

/// Keeps a text as the draft of the person, in place of the older one for the same target. Only the
/// person can read it, until it is saved, thrown away, or 30 days old.
#[utoipa::path(
    put,
    path = "/api/v1/drafts",
    operation_id = "saveDraft",
    tag = "editing",
    request_body = SaveDraftBody,
    responses(
        (status = 200, description = "The draft as it is kept.", body = DraftBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn save_draft(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<SaveDraftBody>,
) -> Result<Response, ApiError> {
    let saved = auth
        .api()?
        .save_draft(
            info,
            SaveDraftQuery {
                session_token,
                target: body.target.target()?,
                draft: DraftText::from(body.draft),
            },
        )
        .await?;
    answer_authorized(&saved.map(|draft| DraftBody::from(&draft)))
}

/// The draft of the person for a target: theirs, and nobody else's.
#[utoipa::path(
    get,
    path = "/api/v1/drafts",
    operation_id = "readDraft",
    tag = "editing",
    params(DraftTargetBody),
    responses(
        (status = 200, description = "The draft, or nothing.", body = DraftSlotBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn read_draft(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(target): ApiQuery<DraftTargetBody>,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .read_draft(
            info,
            ReadDraftQuery {
                session_token,
                target: target.target()?,
            },
        )
        .await?;
    answer_authorized(&found.map(|slot| DraftSlotBody::from(&slot)))
}

/// Throws the draft of the person away (*Throw away*).
#[utoipa::path(
    post,
    path = "/api/v1/drafts/throw-away",
    operation_id = "throwAwayDraft",
    tag = "editing",
    request_body = DraftTargetBody,
    responses((status = 204, description = "The draft is gone."), InvalidAnswer, NotSignedInAnswer)
)]
pub(super) async fn throw_away_draft(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(target): ApiJson<DraftTargetBody>,
) -> Result<Response, ApiError> {
    auth.api()?
        .throw_away_draft(
            info,
            ThrowAwayDraftQuery {
                session_token,
                target: target.target()?,
            },
        )
        .await?;
    Ok(done())
}

/// The versions of the records on the screen, in one small request: what the browser asks every 30
/// seconds and when it returns to a tab, to offer *Show changes* where somebody else changed
/// something.
#[utoipa::path(
    post,
    path = "/api/v1/versions/check",
    operation_id = "checkVersions",
    tag = "editing",
    request_body = CheckVersionsBody,
    responses(
        (status = 200, description = "The version of every record asked about.", body = VersionsBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn check_versions(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<CheckVersionsBody>,
) -> Result<Response, ApiError> {
    let records = body
        .records
        .iter()
        .map(RecordRefBody::record)
        .collect::<Result<Vec<_>, _>>()?;
    let checked = auth
        .api()?
        .check_versions(
            info,
            CheckVersionsQuery {
                session_token,
                records,
            },
        )
        .await?;
    answer_authorized(&checked.map(|versions| VersionsBody::from(&versions)))
}
