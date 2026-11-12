//! The routes of *Working hours of others*, below `/api/v1/people/{person}/work-days`: another
//! person's days for a holder of *See working hours of others*, as *Totals only* allows, and a
//! change of a day, always with a reason, for a holder of *Change working hours of others*
//! (ADR-0035, decisions 7 and 8). Without the permission a person's days answer like a person that
//! does not exist. No access key holds either permission.

use axum::extract::State;
use axum::response::Response;
use bb_app::working_hours::{
    ChangeWorkDay, PeopleOfHoursQuery, WorkDayHistoryQuery, WorkDaysQuery, read_date,
};
use bb_core::id::UserId;
use serde::Deserialize;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::super::access_bodies::cursor_error;
use super::super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized,
};
use super::super::date_range::{RangeParams, invalid, person_and_range};
use super::super::groups::{ChoiceItem, choice_page};
use super::super::path_id;
use super::{ChangeWorkDayBody, WorkDayBody, WorkDayHistoryBody, WorkDaysBody};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{ListParams, Page};

/// Another person's work days from `from` to `to`. Needs *See working hours of others*; while
/// *Totals only* is on, the answer has the week totals and nothing else.
#[utoipa::path(
    get,
    path = "/api/v1/people/{person}/work-days",
    operation_id = "listWorkDaysOfPerson",
    tag = "working-hours",
    params(("person" = String, Path, description = "The ID of the person."), RangeParams),
    responses(
        (status = 200, description = "The work days.", body = WorkDaysBody),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn list_work_days_of_person(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(person): ApiPath<String>,
    range: Result<ApiQuery<RangeParams>, ApiError>,
) -> Result<Response, ApiError> {
    let read = person_and_range(&person, range);
    let (person, range) = auth.after_who_asks(&info, &session_token, read).await?;
    let page = auth
        .api()?
        .work_days(
            info,
            WorkDaysQuery {
                session_token,
                person: Some(person),
                first: range.first,
                last: range.last,
            },
        )
        .await?;
    answer_authorized(&page.map(|page| WorkDaysBody::from(&page)))
}

/// New times of another person's work day, always with a reason. Needs *Change working hours of
/// others*; refused while *Totals only* is on. The person is told, with the reason.
#[utoipa::path(
    patch,
    path = "/api/v1/people/{person}/work-days/{date}",
    operation_id = "changeWorkDayOfPerson",
    tag = "working-hours",
    params(
        ("person" = String, Path, description = "The ID of the person."),
        ("date" = String, Path, description = "The date of the day (`2026-10-05`).")
    ),
    request_body = ChangeWorkDayBody,
    responses(
        (status = 200, description = "The day as it is now.", body = WorkDayBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_work_day_of_person(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((person, date)): ApiPath<(String, String)>,
    body: Result<ApiJson<ChangeWorkDayBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = path_id::<UserId>(&person).and_then(|person| {
        let date = read_date(&date).ok_or_else(invalid)?;
        let ApiJson(body) = body?;
        let (periods, breaks) = body.times()?;
        Ok((person, date, body.version, periods, breaks, body.reason))
    });
    let (person, date, version, periods, breaks, reason) =
        auth.after_who_asks(&info, &session_token, read).await?;
    let day = auth
        .api()?
        .change_work_day(
            info,
            ChangeWorkDay {
                session_token,
                person: Some(person),
                date,
                version,
                periods,
                breaks,
                reason,
                // The device of the reader says nothing of the zone of another person.
                device_zone: None,
            },
        )
        .await?;
    answer_authorized(&day.map(|day| WorkDayBody::from(&day)))
}

/// The history of another person's work day. Needs *See working hours of others*; refused while
/// *Totals only* is on.
#[utoipa::path(
    get,
    path = "/api/v1/people/{person}/work-days/{date}/history",
    operation_id = "getWorkDayHistoryOfPerson",
    tag = "working-hours",
    params(
        ("person" = String, Path, description = "The ID of the person."),
        ("date" = String, Path, description = "The date of the day (`2026-10-05`).")
    ),
    responses(
        (status = 200, description = "The history.", body = WorkDayHistoryBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn work_day_history_of_person(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((person, date)): ApiPath<(String, String)>,
) -> Result<Response, ApiError> {
    let read = path_id::<UserId>(&person)
        .and_then(|person| Ok((person, read_date(&date).ok_or_else(invalid)?)));
    let (person, date) = auth.after_who_asks(&info, &session_token, read).await?;
    let history = auth
        .api()?
        .work_day_history(
            info,
            WorkDayHistoryQuery {
                session_token,
                person: Some(person),
                date,
            },
        )
        .await?;
    answer_authorized(&history.map(|history| WorkDayHistoryBody::from(&history)))
}

const PEOPLE_LIST: &str = "work-day-people";

/// What narrows the people to choose from.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct PeopleSearch {
    /// Words that every found name holds, without regard to capitals.
    search: Option<String>,
}

/// The people whose working hours the reader may open, by name, a page at a time, to choose one
/// from. Needs *See working hours of others*. Names and IDs only: no hours, no counts.
#[utoipa::path(
    get,
    path = "/api/v1/work-days/people",
    operation_id = "listPeopleOfWorkDays",
    tag = "working-hours",
    params(ListParams, PeopleSearch),
    responses(
        (status = 200, description = "The people to choose from.", body = Page<ChoiceItem>),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn list_people_of_work_days(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
    ApiQuery(filter): ApiQuery<PeopleSearch>,
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(PEOPLE_LIST);
    let found = auth
        .api()?
        .people_of_hours(
            info,
            PeopleOfHoursQuery {
                session_token,
                search: filter.search,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    choice_page(PEOPLE_LIST, &params, found)
}

/// The routes of *Working hours of others*.
pub(super) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_people_of_work_days))
        .routes(routes!(list_work_days_of_person))
        .routes(routes!(change_work_day_of_person))
        .routes(routes!(work_day_history_of_person))
}
