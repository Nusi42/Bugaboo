//! The route of the calendar (PLAN-09, PRJ-06, ADR-0034, amendment of 2026-11-24): one route answers
//! the issues of a window of days by the chosen date field (`POST /api/v1/calendar`), each on its
//! day, with the due dates of their goals as marks. A drag is the change of that date field with
//! its version (`PATCH /api/v1/issues/{issue}`), which refuses an old version with the newer value.

use std::sync::Arc;

use axum::Extension;
use axum::extract::{FromRef, State};
use axum::response::Response;
use bb_app::calendar::{CalendarEntry, CalendarPage, CalendarQuery};
use bb_core::issue::CalendarDay;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session, answer_authorized,
};
use super::issues::IssueItem;
use super::saved_searches::CalendarOptionsBody;
use super::search::{counted, view_search};
use crate::errors::{ApiError, ApiJson};
use crate::limits::Limits;

/// Ask for the issues of a window of days.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CalendarBody {
    /// The saved search the calendar is a view of, by its ID; or else `search_text`.
    #[serde(default)]
    saved_search: Option<String>,
    /// The search text, which need not be saved; used when `saved_search` is left out.
    #[serde(default)]
    search_text: Option<String>,
    /// The project whose calendar it is, by ID or short name: `project within KEY` is written into
    /// the text; left out, everywhere the person sees.
    #[serde(default)]
    project: Option<String>,
    /// *Only this project*: `project is KEY` in place of `project within KEY`.
    #[serde(default)]
    only_this_project: bool,
    /// The options of the view: the date field (*Dates from*).
    calendar: CalendarOptionsBody,
    /// The first day of the window (`2026-11-30`).
    #[schema(value_type = String, format = Date)]
    first: CalendarDay,
    /// The last day of the window; a window is six weeks at most.
    #[schema(value_type = String, format = Date)]
    last: CalendarDay,
}

/// An issue on its day.
#[derive(Serialize, ToSchema)]
pub(crate) struct CalendarEntryBody {
    /// The day of its date field.
    #[schema(value_type = String, format = Date)]
    day: CalendarDay,
    /// The issue, with what the reader may see of it in its project.
    #[schema(value_type = IssueItem)]
    issue: Value,
    /// Whether the reader may change the date field on this issue: only then can it be dragged.
    may_move: bool,
}

/// The due date of a goal, a mark that is not dragged.
#[derive(Serialize, ToSchema)]
pub(crate) struct CalendarMarkBody {
    /// The goal, by its internal ID.
    issue: String,
    /// Its readable ID.
    readable_id: String,
    /// Its title; `null` when the reader may not read it.
    #[schema(required = true)]
    title: Option<String>,
    /// Its *Due date*.
    #[schema(value_type = String, format = Date)]
    day: CalendarDay,
}

/// The issues of a window of days.
#[derive(Default, Serialize, ToSchema)]
pub(crate) struct CalendarPageBody {
    /// The search text that was run: a project's calendar has its project written in.
    search_text: String,
    /// The issues, by day, then in the order of the search.
    entries: Vec<CalendarEntryBody>,
    /// Whether the window holds more than the 200 issues answered.
    more: bool,
    /// The due dates of the goals of the issues shown that lie in the window.
    marks: Vec<CalendarMarkBody>,
}

impl Masked for CalendarPageBody {}

fn entry_body(entry: CalendarEntry) -> Result<CalendarEntryBody, ApiError> {
    Ok(CalendarEntryBody {
        day: entry.day,
        issue: entry
            .issue
            .map(|view| IssueItem::from(&view))
            .masked_value()
            .map_err(|error| ApiError::internal(&error))?,
        may_move: entry.may_move,
    })
}

fn page_body(page: CalendarPage) -> Result<CalendarPageBody, ApiError> {
    Ok(CalendarPageBody {
        search_text: page.search_text,
        entries: page
            .entries
            .into_iter()
            .map(entry_body)
            .collect::<Result<_, _>>()?,
        more: page.more,
        marks: page
            .marks
            .into_iter()
            .map(|mark| CalendarMarkBody {
                issue: mark.issue.to_string(),
                readable_id: mark.readable_id,
                title: mark.title,
                day: mark.day,
            })
            .collect(),
    })
}

/// The issues of a search whose date field (*Dates from*: *Due date* unless the options name
/// *Start date* or an own date field) lies in a window of at most six weeks, each on its day, at
/// most 200 (`more` says when there are more), and the due dates of their goals as marks. Restricted
/// issues and dates the reader may not read are on no day. It reads and counts against the
/// searches of the person; an access key reads it as its person may.
#[utoipa::path(
    post,
    path = "/api/v1/calendar",
    operation_id = "getCalendar",
    tag = "calendar",
    request_body = CalendarBody,
    responses(
        (status = 200, description = "The issues of a window of days.", body = CalendarPageBody),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn get_calendar(
    State(auth): State<AuthHandle>,
    Extension(limits): Extension<Arc<Limits>>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<CalendarBody>,
) -> Result<Response, ApiError> {
    counted(&auth, &limits, &info, &session_token).await?;
    let search = view_search(body.saved_search.as_deref(), body.search_text.as_deref())?;
    let page = auth
        .api()?
        .calendar(
            info,
            CalendarQuery {
                session_token,
                search,
                project: body.project,
                only_this_project: body.only_this_project,
                options: body.calendar.into(),
                first: body.first,
                last: body.last,
            },
        )
        .await?;
    let mut failed = None;
    let answer = page.map(|page| {
        page_body(page).unwrap_or_else(|error| {
            failed = Some(error);
            CalendarPageBody::default()
        })
    });
    if let Some(error) = failed {
        return Err(error);
    }
    answer_authorized(&answer)
}

pub(crate) fn routes<S>(limits: &Arc<Limits>) -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_calendar))
        .layer(Extension(Arc::clone(limits)))
}
