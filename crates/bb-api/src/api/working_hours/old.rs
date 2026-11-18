//! The routes of the work days older than the keep period, below `/api/v1/work-days/old`: how many
//! there are, and *Delete them*, for everybody or for one person. Both need *Change organization
//! settings*, which no access key holds; nothing deletes a work day by itself (ADR-0035,
//! decision 7). They are the routes of the keep rules (`keep_rules`) for the kind of work days,
//! kept at their addresses for the page *Working hours*.

use axum::extract::State;
use axum::response::Response;
use bb_app::keeping::{DeleteOldRecords, OldRecordsQuery};
use bb_core::keeping::KeepKind;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotSignedInAnswer, PastChangedAnswer,
    Session, answer_authorized,
};
use super::super::keep_rules::{DeleteParams, DeletedBody, delete_params};
use super::OldWorkDaysBody;
use crate::errors::{ApiError, ApiQuery};

/// How many work days are older than the keep period, for each person and in total, counted now.
#[utoipa::path(
    get,
    path = "/api/v1/work-days/old",
    operation_id = "countOldWorkDays",
    tag = "working-hours",
    responses(
        (status = 200, description = "The count.", body = OldWorkDaysBody),
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn count_old_work_days(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let old = auth
        .api()?
        .old_records(
            info,
            OldRecordsQuery {
                session_token,
                kind: KeepKind::WorkDays,
            },
        )
        .await?;
    answer_authorized(&old.map(|old| OldWorkDaysBody::from(&old)))
}

/// *Delete them*: deletes the work days older than the keep period, of everybody or of one person,
/// with their breaks and the history of their changes. Every deletion is in the security log.
#[utoipa::path(
    delete,
    path = "/api/v1/work-days/old",
    operation_id = "deleteOldWorkDays",
    tag = "working-hours",
    params(DeleteParams),
    responses(
        (status = 200, description = "How many were deleted.", body = DeletedBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        PastChangedAnswer
    )
)]
async fn delete_old_work_days(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    params: Result<ApiQuery<DeleteParams>, ApiError>,
) -> Result<Response, ApiError> {
    let (person, before) = auth
        .after_who_asks(&info, &session_token, delete_params(params))
        .await?;
    let deleted = auth
        .api()?
        .delete_old_records(
            info,
            DeleteOldRecords {
                session_token,
                kind: KeepKind::WorkDays,
                person,
                before,
            },
        )
        .await?;
    answer_authorized(&deleted.map(|deleted| DeletedBody::from(&deleted)))
}

/// The routes of the old work days.
pub(super) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(count_old_work_days, delete_old_work_days))
}
