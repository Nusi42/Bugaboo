//! The routes of one's own work days, below `/api/v1/me/work-days`: the week, the taps of the day
//! that runs, a change typed afterwards, the history of a day and the comparison with the time
//! booked on issues. Every person has them; no permission of the catalogue is needed.

use axum::extract::State;
use axum::response::Response;
use bb_app::working_hours::{
    BookedWeekQuery, ChangeWorkDay, EndBreak, EndWorkDay, StartBreak, StartWorkDay,
    WorkDayHistoryQuery, WorkDaysQuery, read_date,
};
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::super::auth::{
    AuthHandle, ConflictAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized,
};
use super::{
    BookedWeekBody, ChangeWorkDayBody, RangeParams, WorkDayBody, WorkDayHistoryBody, WorkDaysBody,
    invalid,
};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};

/// What *Start work day* may say.
#[derive(Deserialize, ToSchema)]
pub(crate) struct StartBody {
    /// The time zone of the device (`Europe/Berlin`); it counts only when the person and the
    /// organization chose none under *Region*.
    device_zone: Option<String>,
}

/// The version of the day the person looked at, for a tap.
#[derive(Deserialize, ToSchema)]
pub(crate) struct TapBody {
    /// The version of the day that runs: a stored day has at least the first version.
    #[schema(minimum = 1)]
    version: i64,
}

/// A date of the week to compare.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct WeekParams {
    /// A date of the week (`2026-10-05`).
    date: String,
}

/// The person's own work days from `from` to `to`.
#[utoipa::path(
    get,
    path = "/api/v1/me/work-days",
    operation_id = "listMyWorkDays",
    tag = "working-hours",
    params(RangeParams),
    responses(
        (status = 200, description = "The work days.", body = WorkDaysBody),
        InvalidAnswer,
        NotSignedInAnswer
    )
)]
async fn list_my_work_days(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    range: Result<ApiQuery<RangeParams>, ApiError>,
) -> Result<Response, ApiError> {
    let read = range.and_then(|ApiQuery(range)| {
        Ok((
            read_date(&range.from).ok_or_else(invalid)?,
            read_date(&range.to).ok_or_else(invalid)?,
        ))
    });
    let (first, last) = auth.after_who_asks(&info, &session_token, read).await?;
    let page = auth
        .api()?
        .work_days(
            info,
            WorkDaysQuery {
                session_token,
                person: None,
                first,
                last,
            },
        )
        .await?;
    answer_authorized(&page.map(|page| WorkDaysBody::from(&page)))
}

/// *Start work day*: records the moment of the tap as the start of the person's day, or of a new
/// period of it. A day left open on another date is never ended here: the answer
/// `working-hours.day-left-open` names it (`open_day`), and the person says when it ended first.
#[utoipa::path(
    post,
    path = "/api/v1/me/work-days/start",
    operation_id = "startWorkDay",
    tag = "working-hours",
    request_body = StartBody,
    responses(
        (status = 200, description = "The day as it is now.", body = WorkDayBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ConflictAnswer
    )
)]
async fn start_work_day(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<StartBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.map(|ApiJson(body)| body.device_zone);
    let device_zone = auth.after_who_asks(&info, &session_token, read).await?;
    let day = auth
        .api()?
        .start_work_day(
            info,
            StartWorkDay {
                session_token,
                device_zone,
            },
        )
        .await?;
    answer_authorized(&day.map(|day| WorkDayBody::from(&day)))
}

/// *Start break*.
#[utoipa::path(
    post,
    path = "/api/v1/me/work-days/break/start",
    operation_id = "startBreak",
    tag = "working-hours",
    request_body = TapBody,
    responses(
        (status = 200, description = "The day as it is now.", body = WorkDayBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ConflictAnswer
    )
)]
async fn start_break(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<TapBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.map(|ApiJson(body)| body.version);
    let version = auth.after_who_asks(&info, &session_token, read).await?;
    let day = auth
        .api()?
        .start_break(
            info,
            StartBreak {
                session_token,
                version,
            },
        )
        .await?;
    answer_authorized(&day.map(|day| WorkDayBody::from(&day)))
}

/// *End break*.
#[utoipa::path(
    post,
    path = "/api/v1/me/work-days/break/end",
    operation_id = "endBreak",
    tag = "working-hours",
    request_body = TapBody,
    responses(
        (status = 200, description = "The day as it is now.", body = WorkDayBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ConflictAnswer
    )
)]
async fn end_break(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<TapBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.map(|ApiJson(body)| body.version);
    let version = auth.after_who_asks(&info, &session_token, read).await?;
    let day = auth
        .api()?
        .end_break(
            info,
            EndBreak {
                session_token,
                version,
            },
        )
        .await?;
    answer_authorized(&day.map(|day| WorkDayBody::from(&day)))
}

/// *End work day*: ends the break that runs too.
#[utoipa::path(
    post,
    path = "/api/v1/me/work-days/end",
    operation_id = "endWorkDay",
    tag = "working-hours",
    request_body = TapBody,
    responses(
        (status = 200, description = "The day as it is now.", body = WorkDayBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ConflictAnswer
    )
)]
async fn end_work_day(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<TapBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.map(|ApiJson(body)| body.version);
    let version = auth.after_who_asks(&info, &session_token, read).await?;
    let day = auth
        .api()?
        .end_work_day(
            info,
            EndWorkDay {
                session_token,
                version,
            },
        )
        .await?;
    answer_authorized(&day.map(|day| WorkDayBody::from(&day)))
}

/// New times of one's own work day, typed afterwards. A past day changes only with a reason, and
/// the history of the day keeps the old times.
#[utoipa::path(
    patch,
    path = "/api/v1/me/work-days/{date}",
    operation_id = "changeMyWorkDay",
    tag = "working-hours",
    params(("date" = String, Path, description = "The date of the day (`2026-10-05`).")),
    request_body = ChangeWorkDayBody,
    responses(
        (status = 200, description = "The day as it is now.", body = WorkDayBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ConflictAnswer
    )
)]
async fn change_my_work_day(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(date): ApiPath<String>,
    body: Result<ApiJson<ChangeWorkDayBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = read_date(&date).ok_or_else(invalid).and_then(|date| {
        let ApiJson(body) = body?;
        let (periods, breaks) = body.times()?;
        Ok((date, body.version, periods, breaks, body.reason))
    });
    let (date, version, periods, breaks, reason) =
        auth.after_who_asks(&info, &session_token, read).await?;
    let day = auth
        .api()?
        .change_work_day(
            info,
            ChangeWorkDay {
                session_token,
                person: None,
                date,
                version,
                periods,
                breaks,
                reason,
            },
        )
        .await?;
    answer_authorized(&day.map(|day| WorkDayBody::from(&day)))
}

/// The history of one's own work day: every change with the old and the new times, who and why.
#[utoipa::path(
    get,
    path = "/api/v1/me/work-days/{date}/history",
    operation_id = "getMyWorkDayHistory",
    tag = "working-hours",
    params(("date" = String, Path, description = "The date of the day (`2026-10-05`).")),
    responses(
        (status = 200, description = "The history.", body = WorkDayHistoryBody),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn my_work_day_history(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(date): ApiPath<String>,
) -> Result<Response, ApiError> {
    let date = auth
        .after_who_asks(&info, &session_token, read_date(&date).ok_or_else(invalid))
        .await?;
    let history = auth
        .api()?
        .work_day_history(
            info,
            WorkDayHistoryQuery {
                session_token,
                person: None,
                date,
            },
        )
        .await?;
    answer_authorized(&history.map(|history| WorkDayHistoryBody::from(&history)))
}

/// One's own week: the working hours, the time booked on issues and the difference of each day.
/// Only for the person themselves.
#[utoipa::path(
    get,
    path = "/api/v1/me/work-days/booked",
    operation_id = "getMyBookedWeek",
    tag = "working-hours",
    params(WeekParams),
    responses(
        (status = 200, description = "The week.", body = BookedWeekBody),
        InvalidAnswer,
        NotSignedInAnswer
    )
)]
async fn my_booked_week(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    week: Result<ApiQuery<WeekParams>, ApiError>,
) -> Result<Response, ApiError> {
    let read = week.and_then(|ApiQuery(week)| read_date(&week.date).ok_or_else(invalid));
    let date = auth.after_who_asks(&info, &session_token, read).await?;
    let week = auth
        .api()?
        .booked_week(
            info,
            BookedWeekQuery {
                session_token,
                date,
            },
        )
        .await?;
    answer_authorized(&week.map(|week| BookedWeekBody::from(&week)))
}

/// The routes of one's own work days.
pub(super) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_my_work_days))
        .routes(routes!(start_work_day))
        .routes(routes!(start_break))
        .routes(routes!(end_break))
        .routes(routes!(end_work_day))
        .routes(routes!(change_my_work_day))
        .routes(routes!(my_work_day_history))
        .routes(routes!(my_booked_week))
}
