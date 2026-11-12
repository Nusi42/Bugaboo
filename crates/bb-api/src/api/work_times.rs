//! The routes of work times (PLAN-11, PLAN-15, PRIV-09, ADR-0030): one's own below
//! `/api/v1/me/work-times`, another person's below `/api/v1/people/{person}/work-times`, the away
//! times below `/api/v1/away-times`, and the shares of a project below
//! `/api/v1/projects/{project}/shares`. An away time has no reason, and its note is the person's
//! alone. Every answer is personal data that no cache keeps. The use cases are in `bb-app`, behind
//! [`AuthApi`](bb_app::auth::AuthApi); no access key reaches them.

mod away;
mod people;
mod shares;

use bb_app::work_times::{
    AwayDays, AwayView, ChangedAwayTime, ChangedUsualDays, ShareItem, SharesOfPerson, WorkTimesView,
};
use bb_core::holidays::Region as HolidayRegion;
use bb_core::work_times::UsualDay;
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;

use super::access_keys::ProjectLabelBody;
use super::auth::AuthHandle;
use super::date_range::invalid;
use crate::errors::ApiError;

/// One weekday of the usual work days.
#[derive(Clone, Copy, Serialize, Deserialize, ToSchema)]
pub(crate) struct UsualDayBody {
    /// Whether the person usually works on this weekday.
    works: bool,
    /// The work hours of the day, in minutes: at most 1440. The hours are kept when the day is not
    /// ticked.
    #[schema(minimum = 0, maximum = 1440)]
    minutes: u16,
}

impl From<UsualDay> for UsualDayBody {
    fn from(day: UsualDay) -> Self {
        Self {
            works: day.works,
            minutes: day.minutes,
        }
    }
}

/// The seven usual work days, Monday first.
#[derive(Serialize, Deserialize, ToSchema)]
pub(crate) struct UsualDaysBody {
    /// The version of the usual work days the person looked at; 0 while they have none of their
    /// own. A change is made on the version that was looked at.
    version: i64,
    /// The seven days, Monday first.
    #[schema(min_items = 7, max_items = 7)]
    days: Vec<UsualDayBody>,
}

impl Masked for UsualDaysBody {}

impl UsualDaysBody {
    fn of(days: &[UsualDay], version: i64) -> Self {
        Self {
            version,
            days: days.iter().copied().map(UsualDayBody::from).collect(),
        }
    }

    /// The seven days it says; anything else is not a valid request.
    fn read(&self) -> Result<[UsualDay; 7], ApiError> {
        let days: Vec<UsualDay> = self
            .days
            .iter()
            .map(|day| UsualDay {
                works: day.works,
                minutes: day.minutes,
            })
            .collect();
        <[UsualDay; 7]>::try_from(days).map_err(|_| invalid())
    }
}

impl From<&ChangedUsualDays> for UsualDaysBody {
    fn from(changed: &ChangedUsualDays) -> Self {
        Self::of(&changed.usual, changed.version)
    }
}

/// An away time as its reader sees it.
#[derive(Serialize, ToSchema)]
pub(crate) struct AwayBody {
    /// The ID of the away time.
    id: String,
    /// The first day (`2026-10-26`).
    first: String,
    /// The last day.
    last: String,
    /// Whether the person is away only in the afternoon of the first day.
    first_afternoon_only: bool,
    /// Whether the person is away only in the morning of the last day.
    last_morning_only: bool,
    /// The note, which only the person reads; `null` for everybody else and when there is none.
    #[schema(required = true)]
    note: Option<String>,
    /// Who entered it, which only the person sees; `null` for everybody else.
    #[schema(required = true)]
    entered_by: Option<PersonRef>,
    /// Counts up with every change. A change is made on the version that was looked at.
    version: i64,
}

impl From<&AwayView> for AwayBody {
    fn from(view: &AwayView) -> Self {
        Self {
            id: view.id.to_string(),
            first: view.first.to_string(),
            last: view.last.to_string(),
            first_afternoon_only: view.first_afternoon_only,
            last_morning_only: view.last_morning_only,
            note: view.note.clone(),
            entered_by: view.entered_by.clone(),
            version: view.version,
        }
    }
}

impl Masked for AwayBody {}

impl From<&ChangedAwayTime> for AwayBody {
    fn from(changed: &ChangedAwayTime) -> Self {
        Self::from(&changed.away)
    }
}

/// One share of a person, with the project as the reader may name it.
#[derive(Serialize, ToSchema)]
pub(crate) struct ShareItemBody {
    /// The project: its names only when the reader may know it.
    project: ProjectLabelBody,
    /// The share in percent.
    percent: u8,
    /// Whether it is in force: it counts as nothing while the person has no role in the project.
    applies: bool,
}

impl From<&ShareItem> for ShareItemBody {
    fn from(item: &ShareItem) -> Self {
        Self {
            project: ProjectLabelBody::from(&item.project),
            percent: item.percent,
            applies: item.applies,
        }
    }
}

/// The shares a person set, and what the ones that count add up to.
#[derive(Serialize, ToSchema)]
pub(crate) struct SharesBody {
    /// The shares, by the name of the project.
    items: Vec<ShareItemBody>,
    /// What the shares that count add up to, in percent.
    sum: u32,
    /// Whether that is more than 100 percent.
    over_100: bool,
}

impl From<&SharesOfPerson> for SharesBody {
    fn from(shares: &SharesOfPerson) -> Self {
        Self {
            items: shares.items.iter().map(ShareItemBody::from).collect(),
            sum: shares.sum,
            over_100: shares.over_100,
        }
    }
}

/// A person's work times.
#[derive(Serialize, ToSchema)]
pub(crate) struct WorkTimesBody {
    /// Whose they are, as the reader sees people of the kind *Work times*.
    person: PersonRef,
    /// Whether they are the reader's own.
    own: bool,
    /// Whether the reader may change the usual work days and enter away times.
    may_change: bool,
    /// The seven usual work days, Monday first.
    usual: Vec<UsualDayBody>,
    /// The version of the usual work days; 0 while the person has none of their own.
    usual_version: i64,
    /// Which public holidays count for the person: `none`, `de` or a state such as `de-by`.
    holidays: HolidayRegion,
    /// The away times that touch the stretch, by first day. Another person's show their days only.
    away: Vec<AwayBody>,
    /// Whether more away times touch the stretch than the answer holds: the latest are left out.
    truncated: bool,
    /// The shares the person set; only for the person themselves.
    #[schema(required = true)]
    shares: Option<SharesBody>,
}

impl Masked for WorkTimesBody {}

impl From<&WorkTimesView> for WorkTimesBody {
    fn from(view: &WorkTimesView) -> Self {
        Self {
            person: view.person.clone(),
            own: view.own,
            may_change: view.may_change,
            usual: view.usual.iter().copied().map(UsualDayBody::from).collect(),
            usual_version: view.usual_version,
            holidays: view.holidays,
            away: view.away.iter().map(AwayBody::from).collect(),
            truncated: view.truncated,
            shares: view.shares.as_ref().map(SharesBody::from),
        }
    }
}

/// The days of an away time, as a request names them.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AwayDaysBody {
    /// The first day (`2026-10-26`).
    first: String,
    /// The last day, on or after the first and less than a year after it.
    last: String,
    /// Away only in the afternoon of the first day.
    #[serde(default)]
    first_afternoon_only: bool,
    /// Away only in the morning of the last day.
    #[serde(default)]
    last_morning_only: bool,
    /// The note, which only the person reads: refused for another person's away time, at most 500
    /// characters. Bugaboo stores no reason.
    #[serde(default)]
    note: Option<String>,
}

impl AwayDaysBody {
    /// The days it says; a date that is none is not a valid request.
    fn read(self) -> Result<AwayDays, ApiError> {
        let date = |text: &str| bb_app::working_hours::read_date(text).ok_or_else(invalid);
        Ok(AwayDays {
            first: date(&self.first)?,
            last: date(&self.last)?,
            first_afternoon_only: self.first_afternoon_only,
            last_morning_only: self.last_morning_only,
            note: self.note,
        })
    }
}

/// The routes of work times.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .merge(people::routes())
        .merge(away::routes())
        .merge(shares::routes())
}
