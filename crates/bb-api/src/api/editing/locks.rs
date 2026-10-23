//! The lock on a text: taking, reading, keeping alive, releasing, asking to finish and editing anyway.

use super::{
    ApiError, ApiJson, ApiQuery, AskToFinish, AuthHandle, DraftText, EditAnyway,
    EditTargetOnlyBody, EditTargetQuery, EditorId, ForbiddenAnswer, HeartbeatQuery, Info,
    InvalidAnswer, LetGoBody, LockHolderBody, LockOutcomeBody, LockToken, NotFoundAnswer,
    NotSignedInAnswer, ReadLockQuery, ReleaseLockQuery, ReleaseStuckLock, RenewAnswerBody,
    RenewBody, Response, Session, State, TakeLockBody, TakeLockQuery, answer_authorized, body_id,
    done,
};

/// Takes the lock on a long text, on the first keystroke, or says who is editing it. The lock is a
/// lease of a minute that the heartbeat renews; closing the page lets it run out.
#[utoipa::path(
    post,
    path = "/api/v1/edit-locks",
    operation_id = "takeEditLock",
    tag = "editing",
    request_body = TakeLockBody,
    responses(
        (status = 200, description = "The editor has the lock, or somebody else is editing.", body = LockOutcomeBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn take_edit_lock(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<TakeLockBody>,
) -> Result<Response, ApiError> {
    let outcome = auth
        .api()?
        .take_lock(
            info,
            TakeLockQuery {
                session_token,
                what: body.target.target()?,
                editor: body_id::<EditorId>(&body.editor_id)?,
            },
        )
        .await?;
    answer_authorized(&outcome.map(|outcome| LockOutcomeBody::from(&outcome)))
}

/// Says whether somebody is editing a long text, and who, without taking the lock: the box asks
/// when it opens, so a person who comes second is told before the first key.
#[utoipa::path(
    get,
    path = "/api/v1/edit-locks",
    operation_id = "getEditLock",
    tag = "editing",
    params(EditTargetQuery),
    responses(
        (status = 200, description = "Who is editing, or nothing.", body = LockHolderBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn read_edit_lock(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(query): ApiQuery<EditTargetQuery>,
) -> Result<Response, ApiError> {
    let what = query.target()?;
    let holder = auth
        .api()?
        .read_lock(
            info,
            ReadLockQuery {
                session_token,
                what,
            },
        )
        .await?;
    answer_authorized(&holder.map(|holder| LockHolderBody::from(&holder)))
}

/// Renews the lease of a lock, keeps the text so far as the draft of the person (also when the lock
/// is gone), and hands over what others did since the last heartbeat: somebody asked to finish, or
/// took the editing over.
#[utoipa::path(
    post,
    path = "/api/v1/edit-locks/renew",
    operation_id = "renewEditLock",
    tag = "editing",
    request_body = RenewBody,
    responses(
        (status = 200, description = "Whether the editor still has the lock, and the notes for it.", body = RenewAnswerBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn heartbeat_edit_lock(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<RenewBody>,
) -> Result<Response, ApiError> {
    let heartbeat = auth
        .api()?
        .heartbeat(
            info,
            HeartbeatQuery {
                session_token,
                what: body.target.target()?,
                token: body_id::<LockToken>(&body.lock_token)?,
                draft: body.draft.map(DraftText::from),
            },
        )
        .await?;
    answer_authorized(&heartbeat.map(|heartbeat| RenewAnswerBody::from(&heartbeat)))
}

/// Lets go of a lock: the text was saved, or the editor was closed.
#[utoipa::path(
    post,
    path = "/api/v1/edit-locks/let-go",
    operation_id = "letGoOfEditLock",
    tag = "editing",
    request_body = LetGoBody,
    responses((status = 204, description = "The lock is gone."), InvalidAnswer, NotSignedInAnswer)
)]
pub(super) async fn release_edit_lock(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<LetGoBody>,
) -> Result<Response, ApiError> {
    auth.api()?
        .release_lock(
            info,
            ReleaseLockQuery {
                session_token,
                what: body.target.target()?,
                token: body_id::<LockToken>(&body.lock_token)?,
            },
        )
        .await?;
    Ok(done())
}

/// Asks the person who is editing to finish (*Ask … to finish*). They read it at their next
/// heartbeat.
#[utoipa::path(
    post,
    path = "/api/v1/edit-locks/ask-to-finish",
    operation_id = "askToFinish",
    tag = "editing",
    request_body = EditTargetOnlyBody,
    responses(
        (status = 204, description = "The person is asked."),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn ask_to_finish(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<EditTargetOnlyBody>,
) -> Result<Response, ApiError> {
    auth.api()?
        .ask_to_finish(
            info,
            AskToFinish {
                session_token,
                what: body.target.target()?,
            },
        )
        .await?;
    Ok(done())
}

/// Takes the editing over (*Edit anyway*). Needs the permission *Take over text somebody else is
/// editing*. The person who was editing keeps their text as their draft and reads at their next
/// heartbeat that it was taken over.
#[utoipa::path(
    post,
    path = "/api/v1/edit-locks/edit-anyway",
    operation_id = "editAnyway",
    tag = "editing",
    request_body = TakeLockBody,
    responses(
        (status = 200, description = "The editor has the lock now.", body = LockOutcomeBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn edit_anyway(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<TakeLockBody>,
) -> Result<Response, ApiError> {
    let outcome = auth
        .api()?
        .edit_anyway(
            info,
            EditAnyway {
                session_token,
                what: body.target.target()?,
                editor: body_id::<EditorId>(&body.editor_id)?,
            },
        )
        .await?;
    answer_authorized(&outcome.map(|outcome| LockOutcomeBody::from(&outcome)))
}

/// Releases a lock that is stuck, whoever holds it. Needs the permission *Take over text somebody
/// else is editing*; it is written to the security log.
#[utoipa::path(
    post,
    path = "/api/v1/edit-locks/let-go-stuck",
    operation_id = "letGoOfStuckEditLock",
    tag = "editing",
    request_body = EditTargetOnlyBody,
    responses(
        (status = 204, description = "The lock is gone."),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn release_stuck_edit_lock(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<EditTargetOnlyBody>,
) -> Result<Response, ApiError> {
    auth.api()?
        .release_stuck_lock(
            info,
            ReleaseStuckLock {
                session_token,
                what: body.target.target()?,
            },
        )
        .await?;
    Ok(done())
}
