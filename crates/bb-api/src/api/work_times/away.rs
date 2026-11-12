//! The away times (PLAN-11, ADR-0030, decision 2): entering one's own below `/api/v1/me/away-times`,
//! entering another person's with *Enter work times for others* below
//! `/api/v1/people/{person}/away-times`, and changing and removing one by its ID below
//! `/api/v1/away-times/{id}`. An away time has no reason; its note is its person's alone, so a note
//! on another person's away time is refused. The person is told when somebody else enters, changes
//! or removes one.

use axum::extract::State;
use axum::response::Response;
use bb_app::work_times::{ChangeAwayTime, EnterAwayTime, RemoveAwayTime};
use bb_core::id::{AwayTimeId, UserId};
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized, done,
};
use super::super::path_id;
use super::{AwayBody, AwayDaysBody};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};

/// A change of an away time.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeAwayBody {
    /// The version the person looked at.
    version: i64,
    /// The new days.
    days: AwayDaysBody,
}

/// The version of the away time the person looked at, to remove it.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct VersionParams {
    /// The version the person looked at.
    version: i64,
}

/// Enters an away time for the person themselves.
#[utoipa::path(
    post,
    path = "/api/v1/me/away-times",
    operation_id = "enterMyAwayTime",
    tag = "work-times",
    request_body = AwayDaysBody,
    responses(
        (status = 200, description = "The away time.", body = AwayBody),
        InvalidAnswer,
        NotSignedInAnswer
    )
)]
async fn enter_my_away_time(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<AwayDaysBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| body.read());
    let days = auth.after_who_asks(&info, &session_token, read).await?;
    let away = auth
        .api()?
        .enter_away_time(
            info,
            EnterAwayTime {
                session_token,
                person: None,
                days,
            },
        )
        .await?;
    answer_authorized(&away.map(|away| AwayBody::from(&away)))
}

/// Enters an away time for another person. Needs *Enter work times for others*. The person is told;
/// the note is refused (only the person writes it), and nobody states a reason.
#[utoipa::path(
    post,
    path = "/api/v1/people/{person}/away-times",
    operation_id = "enterAwayTimeForPerson",
    tag = "work-times",
    params(("person" = String, Path, description = "The ID of the person.")),
    request_body = AwayDaysBody,
    responses(
        (status = 200, description = "The away time, without a note.", body = AwayBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn enter_away_time_for_person(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(person): ApiPath<String>,
    body: Result<ApiJson<AwayDaysBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = path_id::<UserId>(&person).and_then(|person| {
        let ApiJson(body) = body?;
        Ok((person, body.read()?))
    });
    let (person, days) = auth.after_who_asks(&info, &session_token, read).await?;
    let away = auth
        .api()?
        .enter_away_time(
            info,
            EnterAwayTime {
                session_token,
                person: Some(person),
                days,
            },
        )
        .await?;
    answer_authorized(&away.map(|away| AwayBody::from(&away)))
}

/// Changes the days of an away time: the person's own, or another person's with *Enter work times
/// for others*, who never reads or writes the note, which stays as it is.
#[utoipa::path(
    patch,
    path = "/api/v1/away-times/{id}",
    operation_id = "changeAwayTime",
    tag = "work-times",
    params(("id" = String, Path, description = "The ID of the away time.")),
    request_body = ChangeAwayBody,
    responses(
        (status = 200, description = "The away time as it is now.", body = AwayBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_away_time(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(id): ApiPath<String>,
    body: Result<ApiJson<ChangeAwayBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = path_id::<AwayTimeId>(&id).and_then(|id| {
        let ApiJson(body) = body?;
        Ok((id, body.version, body.days.read()?))
    });
    let (id, version, days) = auth.after_who_asks(&info, &session_token, read).await?;
    let away = auth
        .api()?
        .change_away_time(
            info,
            ChangeAwayTime {
                session_token,
                id,
                version,
                days,
            },
        )
        .await?;
    answer_authorized(&away.map(|away| AwayBody::from(&away)))
}

/// Removes an away time: the person's own, or another person's with *Enter work times for others*.
#[utoipa::path(
    delete,
    path = "/api/v1/away-times/{id}",
    operation_id = "removeAwayTime",
    tag = "work-times",
    params(("id" = String, Path, description = "The ID of the away time."), VersionParams),
    responses(
        (status = 204, description = "The away time is gone."),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn remove_away_time(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(id): ApiPath<String>,
    which: Result<ApiQuery<VersionParams>, ApiError>,
) -> Result<Response, ApiError> {
    let read = path_id::<AwayTimeId>(&id).and_then(|id| {
        let ApiQuery(which) = which?;
        Ok((id, which.version))
    });
    let (id, version) = auth.after_who_asks(&info, &session_token, read).await?;
    auth.api()?
        .remove_away_time(
            info,
            RemoveAwayTime {
                session_token,
                id,
                version,
            },
        )
        .await?;
    Ok(done())
}

/// The routes of the away times.
pub(super) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(enter_my_away_time))
        .routes(routes!(enter_away_time_for_person))
        .routes(routes!(change_away_time))
        .routes(routes!(remove_away_time))
}
