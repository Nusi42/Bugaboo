//! The route that moves the dates of several issues in one run (PLAN-03, ADR-0043, decision 6):
//! the move of *Move it after* and of a drag of several bars. Each issue is changed like a single
//! change (field levels of the dates, its version, start before due date); one refusal stops the
//! whole move, and the run it makes is undone in one step (`/api/v1/runs/{run}/undo`).

use axum::extract::State;
use axum::response::Response;
use bb_app::issues::{DatesMoved, DatesOf, MoveDates, MovedIssue};
use bb_core::given::SetTo;
use bb_core::issue::CalendarDay;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized,
};
use crate::errors::{ApiError, ApiJson};

/// The new dates of one issue: what is left out stays as it is, `null` takes a date away.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DatesOfBody {
    /// The issue, by its readable ID or its internal ID.
    issue: String,
    /// The version of the issue the person looked at.
    version: i64,
    /// A new *Start date* (`2026-11-14`); `null` takes it away.
    #[serde(default, deserialize_with = "bb_core::given::deserialize")]
    #[schema(value_type = Option<String>, format = Date, nullable)]
    start_date: Option<SetTo<CalendarDay>>,
    /// A new *Due date* (`2026-11-14`); `null` takes it away.
    #[serde(default, deserialize_with = "bb_core::given::deserialize")]
    #[schema(value_type = Option<String>, format = Date, nullable)]
    due_date: Option<SetTo<CalendarDay>>,
}

/// The issues whose dates move, each with its new dates.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MoveDatesBody {
    /// The issues, each named once and with at least one date.
    issues: Vec<DatesOfBody>,
}

/// One issue after its dates moved.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct MovedIssueBody {
    /// The internal ID of the issue.
    #[masked(open)]
    id: String,
    /// The readable ID now.
    #[masked(open)]
    readable_id: String,
    /// The version the issue has now.
    #[masked(open)]
    version: i64,
}

/// The run of a move, and the issues in it.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct DatesMovedBody {
    /// The run: *Undo the run* takes every date of the move back in one step.
    #[masked(open)]
    run: String,
    /// The issues, in the order they were named.
    #[masked(nested)]
    issues: Vec<MovedIssueBody>,
}

impl From<&MovedIssue> for MovedIssueBody {
    fn from(issue: &MovedIssue) -> Self {
        Self {
            id: issue.id.to_string(),
            readable_id: issue.key.clone(),
            version: issue.version,
        }
    }
}

impl From<&DatesMoved> for DatesMovedBody {
    fn from(moved: &DatesMoved) -> Self {
        Self {
            run: moved.run.to_string(),
            issues: moved.issues.iter().map(MovedIssueBody::from).collect(),
        }
    }
}

/// Moves the dates of several issues in one run. Every issue is judged as a single change first (a
/// date field the person may not change, a start date after the due date, a version that is not the
/// one stored): when any is refused nothing is written. The run it makes is undone in one step.
#[utoipa::path(
    post,
    path = "/api/v1/issues/move-dates",
    operation_id = "moveDates",
    tag = "issues",
    request_body = MoveDatesBody,
    responses(
        (status = 200, description = "The run and the issues in it.", body = DatesMovedBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn move_dates(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<MoveDatesBody>,
) -> Result<Response, ApiError> {
    let moved = auth
        .api()?
        .move_dates(
            info,
            MoveDates {
                session_token,
                issues: body
                    .issues
                    .into_iter()
                    .map(|each| DatesOf {
                        issue: each.issue,
                        version: each.version,
                        start_date: each.start_date,
                        due_date: each.due_date,
                    })
                    .collect(),
            },
        )
        .await?;
    answer_authorized(&moved.map(|moved| DatesMovedBody::from(&moved)))
}

/// The routes of the dates of several issues.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(move_dates))
}
