//! The routes of working hours (TIME-08, TIME-09, TIME-10, TIME-12, PRIV-09, ADR-0035): one's own
//! days below `/api/v1/me/work-days`, another person's below `/api/v1/people/{person}/work-days`
//! (*Working hours of others*), and the days older than the keep period below
//! `/api/v1/work-days/old`. No route of work days takes an order: a list of days is in the order of
//! its dates, and nothing ranks people by their time (ADR-0035, decision 9). The use cases are in
//! `bb-app`, behind [`AuthApi`](bb_app::auth::AuthApi).

mod old;
mod others;
mod own;

use bb_app::working_hours::{
    BookedWeek, ChangedWorkDay, OldWorkDays, WorkDayChange, WorkDayHistory, WorkDayView,
    WorkDaysPage, read_span,
};
use bb_core::ErrorCode;
use bb_core::working_time::{Span, Times, Totals, Warning};
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;

use super::auth::AuthHandle;
use crate::errors::ApiError;

/// The answer to a date or a moment that is none.
fn invalid() -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest)
}

/// A stretch of a work day: a period of work or a break, its moments in UTC.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub(crate) struct SpanBody {
    /// When it started (RFC 3339).
    start: String,
    /// When it ended; `null` while it runs.
    #[schema(required = true)]
    end: Option<String>,
}

impl From<&Span> for SpanBody {
    fn from(span: &Span) -> Self {
        Self {
            start: span.start.to_string(),
            end: span.end.map(|end| end.to_string()),
        }
    }
}

impl SpanBody {
    /// The stretch it says; a moment that is none is not a valid request.
    fn read(&self) -> Result<Span, ApiError> {
        read_span(&self.start, self.end.as_deref()).ok_or_else(invalid)
    }
}

fn spans(spans: &[Span]) -> Vec<SpanBody> {
    spans.iter().map(SpanBody::from).collect()
}

/// What a day or a week adds up to, in whole minutes.
#[derive(Clone, Copy, Serialize, ToSchema)]
pub(crate) struct TotalsBody {
    /// The time worked.
    worked: u32,
    /// The breaks, the time between periods included.
    breaks: u32,
}

impl From<Totals> for TotalsBody {
    fn from(totals: Totals) -> Self {
        Self {
            worked: totals.worked.get(),
            breaks: totals.breaks.get(),
        }
    }
}

/// A broken working-time rule of a day: which rule, its numbers in minutes, and its sentence.
#[derive(Clone, Serialize, ToSchema)]
pub(crate) struct WarningBody {
    /// The rule and its numbers, in minutes (`{"rule": "long-day", "worked": 635, "most": 600}`).
    #[schema(value_type = Object)]
    warning: Value,
    /// The sentence, as message keys with their words, which the screen writes from its catalog.
    #[schema(value_type = Object)]
    words: Value,
}

impl From<&Warning> for WarningBody {
    fn from(warning: &Warning) -> Self {
        Self {
            warning: serde_json::to_value(warning).unwrap_or(Value::Null),
            words: serde_json::to_value(warning.said()).unwrap_or(Value::Null),
        }
    }
}

/// A work day.
#[derive(Serialize, ToSchema)]
pub(crate) struct WorkDayBody {
    /// Its date: the person's day of its first start.
    date: String,
    /// The time zone it started in.
    zone: String,
    /// The periods of work, in order. The screens show the first start, the last end and the
    /// breaks; the time between periods counts as a break.
    periods: Vec<SpanBody>,
    /// The breaks, in order.
    breaks: Vec<SpanBody>,
    /// What it adds up to; what runs counts until now.
    totals: TotalsBody,
    /// The warnings of the working-time rules; never a reason to refuse anything.
    warnings: Vec<WarningBody>,
    /// Whether a period or a break runs.
    runs: bool,
    /// Counts up with every change. A change is made on the version that was looked at.
    version: i64,
}

impl From<&WorkDayView> for WorkDayBody {
    fn from(view: &WorkDayView) -> Self {
        Self {
            date: view.date.to_string(),
            zone: view.zone.clone(),
            periods: spans(&view.times.periods),
            breaks: spans(&view.times.breaks),
            totals: TotalsBody::from(view.totals),
            warnings: view.warnings.iter().map(WarningBody::from).collect(),
            runs: view.runs,
            version: view.version,
        }
    }
}

impl Masked for WorkDayBody {}

impl From<&ChangedWorkDay> for WorkDayBody {
    fn from(changed: &ChangedWorkDay) -> Self {
        Self::from(&changed.day)
    }
}

/// The time worked in one week.
#[derive(Serialize, ToSchema)]
pub(crate) struct WeekBody {
    /// The first date of the week.
    first: String,
    /// The time worked, in whole minutes.
    worked: u32,
}

/// The work days of a stretch of dates.
#[derive(Serialize, ToSchema)]
pub(crate) struct WorkDaysBody {
    /// Whose days they are.
    person: PersonRef,
    /// The days with a record, by date; empty while *Totals only* hides them.
    days: Vec<WorkDayBody>,
    /// What the days add up to; `null` while *Totals only* hides it.
    #[schema(required = true)]
    totals: Option<TotalsBody>,
    /// The time worked in each week of the stretch.
    weeks: Vec<WeekBody>,
    /// Whether *Totals only* hides the days.
    totals_only: bool,
    /// Whether the reader may change the days.
    may_change: bool,
    /// Never true: a stretch longer than `RANGE_DAYS_MAX` days is refused, so nothing is cut off.
    truncated: bool,
}

impl Masked for WorkDaysBody {}

impl From<&WorkDaysPage> for WorkDaysBody {
    fn from(page: &WorkDaysPage) -> Self {
        Self {
            person: page.person.clone(),
            days: page.days.iter().map(WorkDayBody::from).collect(),
            totals: page.totals.map(TotalsBody::from),
            weeks: page
                .weeks
                .iter()
                .map(|week| WeekBody {
                    first: week.first.to_string(),
                    worked: week.worked.get(),
                })
                .collect(),
            totals_only: page.totals_only,
            may_change: page.may_change,
            truncated: false,
        }
    }
}

/// The periods and breaks of a day, as a change of it and as its history hold them.
#[derive(Serialize, ToSchema)]
pub(crate) struct TimesBody {
    /// The periods of work.
    periods: Vec<SpanBody>,
    /// The breaks.
    breaks: Vec<SpanBody>,
}

impl From<&Times> for TimesBody {
    fn from(times: &Times) -> Self {
        Self {
            periods: spans(&times.periods),
            breaks: spans(&times.breaks),
        }
    }
}

/// One change of a work day.
#[derive(Serialize, ToSchema)]
pub(crate) struct WorkDayChangeBody {
    /// When.
    at: String,
    /// Who changed it.
    by: PersonRef,
    /// Why, when a reason was given.
    #[schema(required = true)]
    reason: Option<String>,
    /// The times before; `null` for the change that made the day.
    #[schema(required = true)]
    before: Option<TimesBody>,
    /// The times after.
    #[schema(required = true)]
    after: Option<TimesBody>,
}

impl From<&WorkDayChange> for WorkDayChangeBody {
    fn from(change: &WorkDayChange) -> Self {
        Self {
            at: change.at.to_string(),
            by: change.by.clone(),
            reason: change.reason.clone(),
            before: change.before.as_ref().map(TimesBody::from),
            after: change.after.as_ref().map(TimesBody::from),
        }
    }
}

/// The history of a work day, oldest change first.
#[derive(Serialize, ToSchema)]
pub(crate) struct WorkDayHistoryBody {
    /// The changes, at most the newest `HISTORY_CHANGES_MAX`.
    changes: Vec<WorkDayChangeBody>,
    /// Whether older changes were left out.
    truncated: bool,
}

impl Masked for WorkDayHistoryBody {}

impl From<&WorkDayHistory> for WorkDayHistoryBody {
    fn from(history: &WorkDayHistory) -> Self {
        Self {
            changes: history
                .changes
                .iter()
                .map(WorkDayChangeBody::from)
                .collect(),
            truncated: history.truncated,
        }
    }
}

/// One day of one's own week compared with the time booked on issues, in whole minutes.
#[derive(Serialize, ToSchema)]
pub(crate) struct BookedDayBody {
    /// The date.
    date: String,
    /// The working hours.
    worked: u32,
    /// The time booked on issues.
    booked: u32,
    /// *Working hours not booked on issues*; below zero when more was booked than worked.
    not_booked: i64,
}

/// One's own week compared with the time booked on issues.
#[derive(Serialize, ToSchema)]
pub(crate) struct BookedWeekBody {
    /// The seven days, in order.
    days: Vec<BookedDayBody>,
    /// Never true: a week has seven days.
    truncated: bool,
}

impl Masked for BookedWeekBody {}

impl From<&BookedWeek> for BookedWeekBody {
    fn from(week: &BookedWeek) -> Self {
        Self {
            days: week
                .days
                .iter()
                .map(|day| BookedDayBody {
                    date: day.date.to_string(),
                    worked: day.worked.get(),
                    booked: day.booked.get(),
                    not_booked: day.not_booked,
                })
                .collect(),
            truncated: false,
        }
    }
}

/// The work days of one person that are older than the keep period.
#[derive(Serialize, ToSchema)]
pub(crate) struct OldOfPersonBody {
    /// The person, as the reader sees people in the settings of the organization.
    person: PersonRef,
    /// The ID of the person, for *Delete them* of one person; left out when the reader does not
    /// see the person by name.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    /// How many of their days.
    count: u64,
}

/// The work days older than the keep period.
#[derive(Serialize, ToSchema)]
pub(crate) struct OldWorkDaysBody {
    /// The keep period in force, in whole years.
    keep_years: i64,
    /// The days with a date before this one are older than it.
    before: String,
    /// How many there are.
    total: u64,
    /// How many of each person, in the order of their names, at most `OLD_PEOPLE_MAX`.
    people: Vec<OldOfPersonBody>,
    /// Whether more people have old days than the list names.
    truncated: bool,
}

impl Masked for OldWorkDaysBody {}

impl From<&OldWorkDays> for OldWorkDaysBody {
    fn from(old: &OldWorkDays) -> Self {
        Self {
            keep_years: old.keep_years,
            before: old.before.to_string(),
            total: old.total,
            people: old
                .people
                .iter()
                .map(|person| OldOfPersonBody {
                    person: person.person.clone(),
                    id: person.id.map(|id| id.to_string()),
                    count: person.count,
                })
                .collect(),
            truncated: old.truncated,
        }
    }
}

/// New times of a work day.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeWorkDayBody {
    /// The version the person looked at; 0 for a date that has no work day yet.
    version: i64,
    /// The periods of work.
    periods: Vec<SpanBody>,
    /// The breaks.
    breaks: Vec<SpanBody>,
    /// Why, in the person's own words: needed for another person's day and for one's own past
    /// day, at most `REASON_CHARS_MAX` characters.
    reason: Option<String>,
}

impl ChangeWorkDayBody {
    /// The periods and breaks it says.
    fn times(&self) -> Result<(Vec<Span>, Vec<Span>), ApiError> {
        let read = |spans: &[SpanBody]| {
            spans
                .iter()
                .map(SpanBody::read)
                .collect::<Result<Vec<_>, _>>()
        };
        Ok((read(&self.periods)?, read(&self.breaks)?))
    }
}

/// The stretch of dates to read.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct RangeParams {
    /// The first date (`2026-10-05`).
    from: String,
    /// The last date, less than `RANGE_DAYS_MAX` days after the first.
    to: String,
}

/// The routes of working hours.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .merge(own::routes())
        .merge(others::routes())
        .merge(old::routes())
}
