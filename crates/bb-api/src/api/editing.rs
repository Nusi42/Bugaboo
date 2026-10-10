//! The editing routes (ISS-17, ISS-11, ADR-0019): edit locks below `/api/v1/edit-locks`, drafts
//! below `/api/v1/drafts`, and the check of what is on the screen at `/api/v1/versions/check`.
//!
//! They work on any record by its kind and ID, and on one part of it (`description`). What a person
//! needs to edit a record depends on its kind (for a project: *Change the project's name, short name
//! and settings*; for an issue: *Change issues*), so these routes declare `by-record` and the use
//! case judges it; *Edit anyway* and releasing a stuck lock name `edit-anyway`. The browser reads
//! the lease and the heartbeat from the answer to taking a lock (and from the generated client), so
//! it never writes those numbers itself. A save of long text carries the lock token and the
//! version in the body of the route of the record (ARCH-VERSIONED).

use std::str::FromStr as _;

use axum::extract::State;
use axum::response::Response;
use bb_app::editing::{
    AskToFinish, CheckVersionsQuery, DraftSlot, DraftText, DraftView, EditAnyway, HeartbeatAnswer,
    HeartbeatQuery, HeldLock, LockHolder, LockOutcome, LockStatus, NoticeView, ReadDraftQuery,
    ReadLockQuery, ReleaseLockQuery, ReleaseStuckLock, SaveDraftQuery, TakeLockQuery, TakenLock,
    ThrowAwayDraftQuery, Versions,
};
use bb_core::ErrorCode;
use bb_core::edit::{DraftTarget, EditWhat, NoticeKind};
use bb_core::event::{EntityKind, EntityRef};
use bb_core::id::{EditorId, EntityId, LockToken, ProjectId};
use bb_core::technical::TechnicalName;
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized, done,
};
use crate::errors::{ApiError, ApiJson, ApiQuery};

/// A record, by its kind and ID.
#[derive(Deserialize, Serialize, ToSchema)]
pub(crate) struct RecordRefBody {
    /// The kind of record, like `issue`.
    kind: String,
    /// The ID of the record.
    id: String,
}

impl RecordRefBody {
    fn record(&self) -> Result<EntityRef, ApiError> {
        Ok(EntityRef {
            kind: EntityKind::new(&self.kind).map_err(|_| ApiError::new(ErrorCode::NotFound))?,
            id: EntityId::from_str(&self.id).map_err(|_| ApiError::new(ErrorCode::NotFound))?,
        })
    }
}

impl From<&EntityRef> for RecordRefBody {
    fn from(record: &EntityRef) -> Self {
        Self {
            kind: record.kind.as_str().to_owned(),
            id: record.id.to_string(),
        }
    }
}

/// One thing being edited: one part of one record.
#[derive(Deserialize, ToSchema)]
pub(crate) struct EditWhatBody {
    /// The record.
    record: RecordRefBody,
    /// Which long text of the record, like `description`.
    part: String,
}

impl EditWhatBody {
    fn what(&self) -> Result<EditWhat, ApiError> {
        Ok(EditWhat {
            record: self.record.record()?,
            part: TechnicalName::new(&self.part)
                .map_err(|_| ApiError::new(ErrorCode::InvalidRequest))?,
        })
    }
}

fn editor(text: &str) -> Result<EditorId, ApiError> {
    EditorId::from_str(text).map_err(|_| ApiError::new(ErrorCode::InvalidRequest))
}

fn token(text: &str) -> Result<LockToken, ApiError> {
    LockToken::from_str(text).map_err(|_| ApiError::new(ErrorCode::InvalidRequest))
}

/// Which thing a person wants the lock on, and from which open editor.
#[derive(Deserialize, ToSchema)]
pub(crate) struct TakeLockBody {
    /// The thing that is going to be edited.
    what: EditWhatBody,
    /// The open editor (a tab): an ID the browser makes when the editor opens.
    editor_id: String,
}

/// The editor has the lock.
#[derive(Serialize, ToSchema)]
pub(crate) struct TakenLockBody {
    /// The lock token to save with and to send with every heartbeat.
    lock_token: String,
    /// When the lock runs out if no heartbeat renews it.
    expires_at: String,
    /// How long a lease lasts, in seconds.
    lease_seconds: i64,
    /// How often to send a heartbeat, in seconds.
    heartbeat_seconds: i64,
}

/// Somebody else is editing.
#[derive(Serialize, ToSchema)]
pub(crate) struct HeldLockBody {
    /// Who is editing, as the person who asks may see them.
    holder: PersonRef,
    /// Since when.
    since: String,
    /// The holder is the person who asks, in another window.
    yours_elsewhere: bool,
    /// The person who asks may take the editing over (*Edit anyway*).
    can_edit_anyway: bool,
}

impl From<&HeldLock> for HeldLockBody {
    fn from(held: &HeldLock) -> Self {
        Self {
            holder: held.holder.clone(),
            since: held.since.to_string(),
            yours_elsewhere: held.yours_elsewhere,
            can_edit_anyway: held.can_edit_anyway,
        }
    }
}

/// Which thing to ask about, in the address.
#[derive(Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct EditWhatQuery {
    /// The kind of record, like `issue`.
    kind: String,
    /// The ID of the record.
    id: String,
    /// Which long text of the record, like `description`.
    part: String,
}

/// Whether somebody is editing a thing now.
#[derive(Serialize, ToSchema)]
pub(crate) struct LockHolderBody {
    /// Who is editing, or nothing when the thing is free.
    held: Option<HeldLockBody>,
}

impl Masked for LockHolderBody {}

impl From<&LockHolder> for LockHolderBody {
    fn from(state: &LockHolder) -> Self {
        Self {
            held: state.held.as_ref().map(HeldLockBody::from),
        }
    }
}

/// The answer to asking for a lock.
#[derive(Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum LockOutcomeBody {
    /// The editor has the lock.
    Taken(TakenLockBody),
    /// Somebody else is editing.
    Held(HeldLockBody),
}

impl Masked for LockOutcomeBody {}

impl From<&LockOutcome> for LockOutcomeBody {
    fn from(outcome: &LockOutcome) -> Self {
        match outcome {
            LockOutcome::Taken(TakenLock {
                token,
                expires_at,
                lease_seconds,
                heartbeat_seconds,
            }) => Self::Taken(TakenLockBody {
                lock_token: token.to_string(),
                expires_at: expires_at.to_string(),
                lease_seconds: *lease_seconds,
                heartbeat_seconds: *heartbeat_seconds,
            }),
            LockOutcome::Held(held) => Self::Held(HeldLockBody::from(held)),
        }
    }
}

/// The text so far of a draft, with the version it started from.
#[derive(Deserialize, ToSchema)]
pub(crate) struct DraftTextBody {
    /// The text as it stands now.
    text: String,
    /// The version of the record the person started from; nothing for a new record.
    base_version: Option<i64>,
}

impl From<DraftTextBody> for DraftText {
    fn from(body: DraftTextBody) -> Self {
        Self {
            text: body.text,
            base_version: body.base_version,
        }
    }
}

/// A heartbeat: the editor is still open.
#[derive(Deserialize, ToSchema)]
pub(crate) struct HeartbeatBody {
    /// The thing that is being edited.
    what: EditWhatBody,
    /// The lock token the lock was taken with.
    lock_token: String,
    /// The text so far: kept as the draft of the person, also when the lock is gone.
    draft: Option<DraftTextBody>,
}

/// Whether the editor still has the lock, and if not, why.
#[derive(Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum LockStatusBody {
    /// The lease is renewed.
    Held {
        /// When it runs out now.
        expires_at: String,
    },
    /// No heartbeat came for a minute, or the lock was let go. The draft is kept.
    RanOut,
    /// Somebody took the editing over (*Edit anyway*). The draft is kept.
    TakenOver {
        /// Who, as the person who asks may see them.
        by: PersonRef,
    },
}

impl From<&LockStatus> for LockStatusBody {
    fn from(status: &LockStatus) -> Self {
        match status {
            LockStatus::Held { expires_at } => Self::Held {
                expires_at: expires_at.to_string(),
            },
            LockStatus::RanOut => Self::RanOut,
            LockStatus::TakenOver { by } => Self::TakenOver { by: by.clone() },
        }
    }
}

/// A note for the person who is editing.
#[derive(Serialize, ToSchema)]
pub(crate) struct NoticeBody {
    /// What happened.
    kind: NoticeKind,
    /// Who did it, as the person who asks may see them.
    from: PersonRef,
    /// When.
    at: String,
}

impl From<&NoticeView> for NoticeBody {
    fn from(notice: &NoticeView) -> Self {
        Self {
            kind: notice.kind,
            from: notice.from.clone(),
            at: notice.at.to_string(),
        }
    }
}

/// The answer to a heartbeat.
#[derive(Serialize, ToSchema)]
pub(crate) struct HeartbeatAnswerBody {
    /// Whether the editor still has the lock.
    lock: LockStatusBody,
    /// What others did since the last heartbeat; each note is shown once.
    notices: Vec<NoticeBody>,
    /// The text that came with the heartbeat was kept as the draft of the person.
    draft_saved: bool,
}

impl Masked for HeartbeatAnswerBody {}

impl From<&HeartbeatAnswer> for HeartbeatAnswerBody {
    fn from(heartbeat: &HeartbeatAnswer) -> Self {
        Self {
            lock: LockStatusBody::from(&heartbeat.lock),
            notices: heartbeat.notices.iter().map(NoticeBody::from).collect(),
            draft_saved: heartbeat.draft_saved,
        }
    }
}

/// Which lock to let go of, and the token it was taken with.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ReleaseLockBody {
    /// The thing that was edited.
    what: EditWhatBody,
    /// The lock token the lock was taken with.
    lock_token: String,
}

/// Which thing somebody else is editing.
#[derive(Deserialize, ToSchema)]
pub(crate) struct EditWhatOnlyBody {
    /// The thing somebody else is editing.
    what: EditWhatBody,
}

/// Take the editing over from the person who is editing.
#[derive(Deserialize, ToSchema)]
pub(crate) struct EditAnywayBody {
    /// The thing somebody else is editing.
    what: EditWhatBody,
    /// The open editor (a tab) that takes over.
    editor_id: String,
}

/// What a draft is for: a part of a record (`kind`, `id` and `part`), or the form that makes a new
/// record of a kind in a project (`kind` and `project`).
#[derive(Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct DraftTargetBody {
    /// The kind of record, like `issue`.
    kind: String,
    /// The ID of the record, for a draft of a part of a record.
    id: Option<String>,
    /// Which long text of the record, for a draft of a part of a record.
    part: Option<String>,
    /// The project, for a draft of a new record.
    project: Option<String>,
}

impl DraftTargetBody {
    fn target(&self) -> Result<DraftTarget, ApiError> {
        let invalid = || ApiError::new(ErrorCode::InvalidRequest);
        match (&self.id, &self.part, &self.project) {
            (Some(id), Some(part), None) => Ok(DraftTarget::Editing(
                EditWhatBody {
                    record: RecordRefBody {
                        kind: self.kind.clone(),
                        id: id.clone(),
                    },
                    part: part.clone(),
                }
                .what()?,
            )),
            (None, None, Some(project)) => Ok(DraftTarget::NewRecord {
                project: ProjectId::from_str(project).map_err(|_| invalid())?,
                kind: EntityKind::new(&self.kind).map_err(|_| invalid())?,
            }),
            _ => Err(invalid()),
        }
    }
}

/// A draft to keep.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SaveDraftBody {
    /// What the draft is for.
    target: DraftTargetBody,
    /// The text and the version it started from.
    draft: DraftTextBody,
}

/// A draft as its author sees it.
#[derive(Serialize, ToSchema)]
pub(crate) struct DraftBody {
    /// The text.
    text: String,
    /// The version of the record the person started from.
    base_version: Option<i64>,
    /// When the draft last changed.
    saved_at: String,
}

impl Masked for DraftBody {}

impl From<&DraftView> for DraftBody {
    fn from(draft: &DraftView) -> Self {
        Self {
            text: draft.text.clone(),
            base_version: draft.base_version,
            saved_at: draft.saved_at.to_string(),
        }
    }
}

/// The draft of the person for a target, if there is one.
#[derive(Serialize, ToSchema)]
pub(crate) struct DraftSlotBody {
    /// The draft, or nothing.
    draft: Option<DraftBody>,
}

impl Masked for DraftSlotBody {}

impl From<&DraftSlot> for DraftSlotBody {
    fn from(slot: &DraftSlot) -> Self {
        Self {
            draft: slot.draft.as_ref().map(DraftBody::from),
        }
    }
}

/// The records on the screen.
#[derive(Deserialize, ToSchema)]
pub(crate) struct CheckVersionsBody {
    /// The records, at most the number the generated client calls `versionsMax`.
    records: Vec<RecordRefBody>,
}

/// The version of one record on the screen.
#[derive(Serialize, ToSchema)]
pub(crate) struct RecordVersionBody {
    /// The record.
    record: RecordRefBody,
    /// Its version now; nothing when it is gone or the person may not know about it (the two look
    /// the same).
    version: Option<i64>,
}

/// The versions of the records asked about, in the order asked.
#[derive(Serialize, ToSchema)]
pub(crate) struct VersionsBody {
    /// One entry for each record asked about.
    items: Vec<RecordVersionBody>,
}

impl Masked for VersionsBody {}

impl From<&Versions> for VersionsBody {
    fn from(checked: &Versions) -> Self {
        Self {
            items: checked
                .items
                .iter()
                .map(|item| RecordVersionBody {
                    record: RecordRefBody::from(&item.record),
                    version: item.version,
                })
                .collect(),
        }
    }
}

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
async fn take_edit_lock(
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
                what: body.what.what()?,
                editor: editor(&body.editor_id)?,
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
    operation_id = "readEditLock",
    tag = "editing",
    params(EditWhatQuery),
    responses(
        (status = 200, description = "Who is editing, or nothing.", body = LockHolderBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn read_edit_lock(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(query): ApiQuery<EditWhatQuery>,
) -> Result<Response, ApiError> {
    let what = EditWhatBody {
        record: RecordRefBody {
            kind: query.kind,
            id: query.id,
        },
        part: query.part,
    }
    .what()?;
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
    path = "/api/v1/edit-locks/heartbeat",
    operation_id = "heartbeatEditLock",
    tag = "editing",
    request_body = HeartbeatBody,
    responses(
        (status = 200, description = "Whether the editor still has the lock, and the notes for it.", body = HeartbeatAnswerBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn heartbeat_edit_lock(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<HeartbeatBody>,
) -> Result<Response, ApiError> {
    let heartbeat = auth
        .api()?
        .heartbeat(
            info,
            HeartbeatQuery {
                session_token,
                what: body.what.what()?,
                token: token(&body.lock_token)?,
                draft: body.draft.map(DraftText::from),
            },
        )
        .await?;
    answer_authorized(&heartbeat.map(|heartbeat| HeartbeatAnswerBody::from(&heartbeat)))
}

/// Lets go of a lock: the text was saved, or the editor was closed.
#[utoipa::path(
    post,
    path = "/api/v1/edit-locks/release",
    operation_id = "releaseEditLock",
    tag = "editing",
    request_body = ReleaseLockBody,
    responses((status = 204, description = "The lock is gone."), InvalidAnswer, NotSignedInAnswer)
)]
async fn release_edit_lock(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<ReleaseLockBody>,
) -> Result<Response, ApiError> {
    auth.api()?
        .release_lock(
            info,
            ReleaseLockQuery {
                session_token,
                what: body.what.what()?,
                token: token(&body.lock_token)?,
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
    request_body = EditWhatOnlyBody,
    responses(
        (status = 204, description = "The person is asked."),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn ask_to_finish(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<EditWhatOnlyBody>,
) -> Result<Response, ApiError> {
    auth.api()?
        .ask_to_finish(
            info,
            AskToFinish {
                session_token,
                what: body.what.what()?,
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
    request_body = EditAnywayBody,
    responses(
        (status = 200, description = "The editor has the lock now.", body = LockOutcomeBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn edit_anyway(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<EditAnywayBody>,
) -> Result<Response, ApiError> {
    let outcome = auth
        .api()?
        .edit_anyway(
            info,
            EditAnyway {
                session_token,
                what: body.what.what()?,
                editor: editor(&body.editor_id)?,
            },
        )
        .await?;
    answer_authorized(&outcome.map(|outcome| LockOutcomeBody::from(&outcome)))
}

/// Releases a lock that is stuck, whoever holds it. Needs the permission *Take over text somebody
/// else is editing*; it is written to the security log.
#[utoipa::path(
    post,
    path = "/api/v1/edit-locks/release-stuck",
    operation_id = "releaseStuckEditLock",
    tag = "editing",
    request_body = EditWhatOnlyBody,
    responses(
        (status = 204, description = "The lock is gone."),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn release_stuck_edit_lock(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<EditWhatOnlyBody>,
) -> Result<Response, ApiError> {
    auth.api()?
        .release_stuck_lock(
            info,
            ReleaseStuckLock {
                session_token,
                what: body.what.what()?,
            },
        )
        .await?;
    Ok(done())
}

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
async fn save_draft(
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
async fn read_draft(
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
async fn throw_away_draft(
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
async fn check_versions(
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

/// The routes of editing.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(take_edit_lock, read_edit_lock))
        .routes(routes!(heartbeat_edit_lock))
        .routes(routes!(release_edit_lock))
        .routes(routes!(ask_to_finish))
        .routes(routes!(edit_anyway))
        .routes(routes!(release_stuck_edit_lock))
        .routes(routes!(save_draft, read_draft))
        .routes(routes!(throw_away_draft))
        .routes(routes!(check_versions))
}
