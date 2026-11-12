//! The work times of a person: the read and the usual work days with their work hours, one's own
//! below `/api/v1/me/work-times` and another person's below `/api/v1/people/{person}/work-times`
//! (PLAN-11, ADR-0030, decision 2). Another person's are read as *Who sees people* allows for the
//! kind *Work times* (a person shown as *Hidden* answers like a person that does not exist), and
//! changed with *Enter work times for others*, who then never reads or writes a note.

use axum::extract::State;
use axum::response::Response;
use bb_app::work_times::{ChangeUsualDays, WorkTimesQuery};
use bb_core::id::UserId;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized,
};
use super::super::path_id;
use super::super::working_hours::{RangeParams, person_and_range};
use super::{UsualDaysBody, WorkTimesBody};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};

/// The person's own work times, with the away times that touch the stretch from `from` to `to`.
#[utoipa::path(
    get,
    path = "/api/v1/me/work-times",
    operation_id = "getMyWorkTimes",
    tag = "work-times",
    params(RangeParams),
    responses(
        (status = 200, description = "The work times.", body = WorkTimesBody),
        InvalidAnswer,
        NotSignedInAnswer
    )
)]
async fn get_my_work_times(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    range: Result<ApiQuery<RangeParams>, ApiError>,
) -> Result<Response, ApiError> {
    let read = range.and_then(|ApiQuery(range)| range.read());
    let range = auth.after_who_asks(&info, &session_token, read).await?;
    let view = auth
        .api()?
        .work_times(
            info,
            WorkTimesQuery {
                session_token,
                person: None,
                first: range.first,
                last: range.last,
            },
        )
        .await?;
    answer_authorized(&view.map(|view| WorkTimesBody::from(&view)))
}

/// Another person's work times, as *Who sees people* allows for the kind *Work times*: an away time
/// shows only its days. A reader who is shown the person as *Hidden* gets the answer of a person
/// that does not exist. No access key reaches work times.
#[utoipa::path(
    get,
    path = "/api/v1/people/{person}/work-times",
    operation_id = "getWorkTimesOfPerson",
    tag = "work-times",
    params(("person" = String, Path, description = "The ID of the person."), RangeParams),
    responses(
        (status = 200, description = "The work times.", body = WorkTimesBody),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn get_work_times_of_person(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(person): ApiPath<String>,
    range: Result<ApiQuery<RangeParams>, ApiError>,
) -> Result<Response, ApiError> {
    let read = person_and_range(&person, range);
    let (person, range) = auth.after_who_asks(&info, &session_token, read).await?;
    let view = auth
        .api()?
        .work_times(
            info,
            WorkTimesQuery {
                session_token,
                person: Some(person),
                first: range.first,
                last: range.last,
            },
        )
        .await?;
    answer_authorized(&view.map(|view| WorkTimesBody::from(&view)))
}

/// New usual work days and work hours for the person: the seven days, Monday first, each ticked or
/// not, with the work hours of each in minutes.
#[utoipa::path(
    put,
    path = "/api/v1/me/work-times/usual-days",
    operation_id = "changeMyUsualDays",
    tag = "work-times",
    request_body = UsualDaysBody,
    responses(
        (status = 200, description = "The usual work days as they are now.", body = UsualDaysBody),
        InvalidAnswer,
        NotSignedInAnswer
    )
)]
async fn change_my_usual_days(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<UsualDaysBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| body.read());
    let days = auth.after_who_asks(&info, &session_token, read).await?;
    let changed = auth
        .api()?
        .change_usual_days(
            info,
            ChangeUsualDays {
                session_token,
                person: None,
                days,
            },
        )
        .await?;
    answer_authorized(&changed.map(|changed| UsualDaysBody::from(&changed)))
}

/// New usual work days and work hours for another person. Needs *Enter work times for others*; the
/// person is told.
#[utoipa::path(
    put,
    path = "/api/v1/people/{person}/work-times/usual-days",
    operation_id = "changeUsualDaysOfPerson",
    tag = "work-times",
    params(("person" = String, Path, description = "The ID of the person.")),
    request_body = UsualDaysBody,
    responses(
        (status = 200, description = "The usual work days as they are now.", body = UsualDaysBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn change_usual_days_of_person(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(person): ApiPath<String>,
    body: Result<ApiJson<UsualDaysBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = path_id::<UserId>(&person).and_then(|person| {
        let ApiJson(body) = body?;
        Ok((person, body.read()?))
    });
    let (person, days) = auth.after_who_asks(&info, &session_token, read).await?;
    let changed = auth
        .api()?
        .change_usual_days(
            info,
            ChangeUsualDays {
                session_token,
                person: Some(person),
                days,
            },
        )
        .await?;
    answer_authorized(&changed.map(|changed| UsualDaysBody::from(&changed)))
}

/// The routes of the work times of a person.
pub(super) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_my_work_times))
        .routes(routes!(get_work_times_of_person))
        .routes(routes!(change_my_usual_days))
        .routes(routes!(change_usual_days_of_person))
}
