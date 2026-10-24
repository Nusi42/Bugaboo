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
use bb_core::event::{EntityKind, EntityRef, RecordKind as CoreRecordKind};
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
use super::body_id;
use crate::errors::{ApiError, ApiJson, ApiQuery};

mod bodies;
mod drafts;
mod locks;

pub(crate) use bodies::*;
use drafts::{
    __path_check_versions, __path_read_draft, __path_save_draft, __path_throw_away_draft,
    check_versions, read_draft, save_draft, throw_away_draft,
};
use locks::{
    __path_ask_to_finish, __path_edit_anyway, __path_heartbeat_edit_lock, __path_read_edit_lock,
    __path_release_edit_lock, __path_release_stuck_edit_lock, __path_take_edit_lock, ask_to_finish,
    edit_anyway, heartbeat_edit_lock, read_edit_lock, release_edit_lock, release_stuck_edit_lock,
    take_edit_lock,
};

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
