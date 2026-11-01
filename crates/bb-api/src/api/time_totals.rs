//! The routes of the time of an issue and of a project (TIME-03, PRIV-13, ADR-0035 decision 4):
//! `/api/v1/issues/{issue}/time` and `/api/v1/projects/{project}/time`. A total counts only the
//! issues the reader sees and only the fields they may read; a member is left out for a field they
//! may not read and while there is nothing to show. Lengths are whole minutes.

use axum::extract::State;
use axum::response::Response;
use bb_app::time::totals::{IssueTime, IssueTimeQuery, ProjectTimeQuery, TimeTotals};
use bb_core::time::{Minutes, TimeLeft};
use bb_policy::Masked;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, Info, NotFoundAnswer, NotSignedInAnswer, Session, answer_authorized,
};
use crate::errors::{ApiError, ApiPath};

/// *Time left* of an issue: the minutes, and whether nobody typed them and they are worked out from
/// the estimate and the time spent (shown in the quiet style of a value worked out).
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct TimeLeftBody {
    /// The minutes.
    #[masked(open)]
    #[schema(value_type = u32)]
    minutes: Minutes,
    /// Whether it is worked out rather than typed.
    #[masked(open)]
    worked_out: bool,
}

impl From<TimeLeft> for TimeLeftBody {
    fn from(left: TimeLeft) -> Self {
        Self {
            minutes: left.minutes(),
            worked_out: matches!(left, TimeLeft::WorkedOut(_)),
        }
    }
}

/// The totals of the time of many issues the reader sees, in whole minutes.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct TimeTotalsBody {
    /// The sum of *Estimated time*; left out when no issue has one or the reader may not read it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "estimated-time")]
    #[schema(value_type = Option<u32>)]
    estimated_time: Option<Minutes>,
    /// The sum of *Time left*, typed or worked out; left out like the estimate.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "time-left")]
    #[schema(value_type = Option<u32>)]
    time_left: Option<Minutes>,
    /// The sum of *Time spent*; left out when the reader may not read it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "time-spent")]
    #[schema(value_type = Option<u32>)]
    time_spent: Option<Minutes>,
}

impl From<&TimeTotals> for TimeTotalsBody {
    fn from(totals: &TimeTotals) -> Self {
        Self {
            estimated_time: totals.estimated_time,
            time_left: totals.time_left,
            time_spent: totals.time_spent,
        }
    }
}

/// The time of one issue: its own numbers, and the totals with its sub-issues at every level.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct IssueTimeBody {
    /// *Estimated time*; left out while it has none or the reader may not read it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "estimated-time")]
    #[schema(value_type = Option<u32>)]
    estimated_time: Option<Minutes>,
    /// *Time left*, typed or worked out; left out without an estimate and a typed value.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "time-left", nested)]
    time_left: Option<TimeLeftBody>,
    /// *Time spent*: the total of the issue's time entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "time-spent")]
    #[schema(value_type = Option<u32>)]
    time_spent: Option<Minutes>,
    /// The totals with the sub-issues the reader sees; left out for an issue without any.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(nested)]
    with_sub_issues: Option<TimeTotalsBody>,
}

impl From<&IssueTime> for IssueTimeBody {
    fn from(time: &IssueTime) -> Self {
        Self {
            estimated_time: time.estimated_time,
            time_left: time.time_left.map(TimeLeftBody::from),
            time_spent: time.time_spent,
            with_sub_issues: time.with_sub_issues.as_ref().map(TimeTotalsBody::from),
        }
    }
}

/// The time of an issue the reader sees: its *Estimated time*, *Time left* and *Time spent*, and
/// the totals with its sub-issues at every level below it, over the issues the reader sees.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/time",
    operation_id = "getIssueTime",
    tag = "time",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    responses((status = 200, description = "The time of the issue.", body = IssueTimeBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_issue_time(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
) -> Result<Response, ApiError> {
    let shown = auth
        .api()?
        .issue_time(
            info,
            IssueTimeQuery {
                session_token,
                issue,
            },
        )
        .await?;
    answer_authorized(&shown.map(|time| IssueTimeBody::from(&time)))
}

/// The totals of the time of a project: its issues and those of its sub-projects that the reader
/// sees, of the fields they may read in all of them.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{project}/time",
    operation_id = "getProjectTime",
    tag = "time",
    params(("project" = String, Path, description = "The ID of the project, or its short name.")),
    responses((status = 200, description = "The totals of the project.", body = TimeTotalsBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_project_time(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
) -> Result<Response, ApiError> {
    let shown = auth
        .api()?
        .project_time(
            info,
            ProjectTimeQuery {
                session_token,
                project,
            },
        )
        .await?;
    answer_authorized(&shown.map(|totals| TimeTotalsBody::from(&totals)))
}

/// The routes of the time of issues and projects.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_issue_time))
        .routes(routes!(get_project_time))
}
