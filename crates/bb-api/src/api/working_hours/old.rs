//! The routes of the work days older than the keep period, below `/api/v1/work-days/old`: how many
//! there are, and *Delete them*, for everybody or for one person. Both need *Change organization
//! settings*, which no access key holds; nothing deletes a work day by itself (ADR-0035,
//! decision 7).

use axum::extract::State;
use axum::response::Response;
use bb_app::working_hours::{DeleteOldWorkDays, DeletedWorkDays, OldWorkDaysQuery};
use bb_core::id::UserId;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotSignedInAnswer, Session, answer_authorized,
};
use super::super::body_id;
use super::OldWorkDaysBody;
use crate::errors::{ApiError, ApiQuery};

/// Whose old work days to delete.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct WhoseParams {
    /// One person, by ID; everybody's when left out.
    person: Option<String>,
}

/// How many work days were deleted.
#[derive(Serialize, ToSchema)]
pub(crate) struct DeletedBody {
    /// The count.
    deleted: u64,
}

impl Masked for DeletedBody {}

impl From<&DeletedWorkDays> for DeletedBody {
    fn from(deleted: &DeletedWorkDays) -> Self {
        Self {
            deleted: deleted.deleted,
        }
    }
}

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
        .old_work_days(info, OldWorkDaysQuery { session_token })
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
    params(WhoseParams),
    responses(
        (status = 200, description = "How many were deleted.", body = DeletedBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn delete_old_work_days(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    whose: Result<ApiQuery<WhoseParams>, ApiError>,
) -> Result<Response, ApiError> {
    let read = whose
        .and_then(|ApiQuery(whose)| whose.person.as_deref().map(body_id::<UserId>).transpose());
    let person = auth.after_who_asks(&info, &session_token, read).await?;
    let deleted = auth
        .api()?
        .delete_old_work_days(
            info,
            DeleteOldWorkDays {
                session_token,
                person,
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
