//! The bar of a row of the timeline as the API answers it (PLAN-10, PLAN-11, PLAN-12, PLAN-14,
//! ADR-0030 and its amendment M8): its days, the person on it, its effort and the person's time,
//! the hatched days, and the days of each person's row under *Group by person*.

use bb_app::plan::{BarView, Hatched, HatchedDay, PersonDaysView};
use bb_core::holidays::Holiday;
use bb_core::issue::CalendarDay;
use bb_core::plan::bar::WorkedOut;
use bb_policy::PersonRef;
use serde::Serialize;
use utoipa::ToSchema;

use super::timeline::DaysBody;
use super::work_times::UsualDayBody;

/// What a hatched day holds.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum HatchedKindBody {
    /// The person is away (only ever *Away*).
    Away,
    /// A public holiday.
    Holiday,
    /// Not a usual work day of the person, or no share in the project.
    DayOff,
    /// A day without work of a person the reader does not see by name: away, a holiday and a day
    /// off are told apart only for a person the reader sees.
    NoWork,
}

/// One day of a bar on which its person does not work, as *Who sees people* allows.
#[derive(Serialize, ToSchema)]
pub(crate) struct HatchedDayBody {
    /// The day.
    #[schema(value_type = String, format = Date)]
    date: CalendarDay,
    /// What it holds.
    kind: HatchedKindBody,
    /// For an away day: whether the morning is away.
    morning: bool,
    /// For an away day: whether the afternoon is away.
    afternoon: bool,
    /// For a public holiday: which one.
    #[schema(required = true)]
    holiday: Option<Holiday>,
}

impl From<&HatchedDay> for HatchedDayBody {
    fn from(day: &HatchedDay) -> Self {
        let (kind, morning, afternoon, holiday) = match day.what {
            Hatched::Away { morning, afternoon } => {
                (HatchedKindBody::Away, morning, afternoon, None)
            }
            Hatched::Holiday { holiday } => (HatchedKindBody::Holiday, false, false, Some(holiday)),
            Hatched::DayOff => (HatchedKindBody::DayOff, false, false, None),
            Hatched::NoWork => (HatchedKindBody::NoWork, false, false, None),
        };
        Self {
            date: day.date,
            kind,
            morning,
            afternoon,
            holiday,
        }
    }
}

/// The away days and public holidays of one person over the span, for their row in *Group by
/// person*.
#[derive(Serialize, ToSchema)]
pub(crate) struct TimelinePersonDaysBody {
    /// The person, by name: the same as the person of the group of the row.
    person: PersonRef,
    /// The days on which they are away or have a public holiday.
    hatched: Vec<HatchedDayBody>,
}

impl From<&PersonDaysView> for TimelinePersonDaysBody {
    fn from(each: &PersonDaysView) -> Self {
        Self {
            person: each.person.clone(),
            hatched: each.hatched.iter().map(HatchedDayBody::from).collect(),
        }
    }
}

/// The time factor a bar is planned with.
#[derive(Serialize, ToSchema)]
pub(crate) struct BarTimeFactorBody {
    /// In percent of the estimate (125 is ×1.25).
    percent: u16,
    /// Whether the person on the bar set it themselves.
    own_estimate: bool,
}

/// The effort of a bar, as its card says it: *"Estimate 8h · ×1.25 = 10 work days"*.
#[derive(Serialize, ToSchema)]
pub(crate) struct BarEffortBody {
    /// *Estimated time*, in minutes.
    estimate: u32,
    /// The estimate times the time factor, in minutes.
    effort: u32,
    /// How many of the person's work days the effort takes from the first day of the bar; `null`
    /// unless the bars are planned by effort and the days the plan reads hold it.
    #[schema(required = true)]
    work_days: Option<u32>,
}

/// The time of the person on a bar, as its card says it: *"Mon–Fri, 8 hours a day · 100 % in
/// Mobile app"*.
#[derive(Serialize, ToSchema)]
pub(crate) struct BarWorkTimeBody {
    /// The seven usual work days, Monday first.
    #[schema(min_items = 7, max_items = 7)]
    usual: Vec<UsualDayBody>,
    /// The share in the project of the issue that counts, in percent.
    share: u8,
}

/// The bar of a row.
#[derive(Serialize, ToSchema)]
pub(crate) struct BarBody {
    /// The days it covers.
    days: DaysBody,
    /// Which date *Plan by effort* worked out (shown, never stored): `neither`, `start` or `due`.
    #[schema(value_type = String)]
    worked_out: WorkedOut,
    /// How much of it is done, in percent; `null` without an estimate the person may read.
    #[schema(required = true)]
    done: Option<u8>,
    /// The person on the bar (the first assigned person) as the reader may see them; `null` for
    /// nobody or when the reader may not see who it is.
    #[schema(required = true)]
    person: Option<PersonRef>,
    /// The time factor, when it is not ×1 and the reader may read it.
    #[schema(required = true)]
    time_factor: Option<BarTimeFactorBody>,
    /// The days inside the span on which the person does not work.
    hatched: Vec<HatchedDayBody>,
    /// The effort; `null` without an estimate the reader may read or without a person on the bar.
    #[schema(required = true)]
    effort: Option<BarEffortBody>,
    /// The time of the person on the bar; `null` unless the reader may know their work times.
    #[schema(required = true)]
    work_time: Option<BarWorkTimeBody>,
}

impl From<&BarView> for BarBody {
    fn from(bar: &BarView) -> Self {
        Self {
            days: bar.days.into(),
            worked_out: bar.worked_out,
            done: bar.done,
            person: bar.person.clone(),
            time_factor: bar.factor.map(|factor| BarTimeFactorBody {
                percent: factor.percent,
                own_estimate: factor.own_estimate,
            }),
            hatched: bar.hatched.iter().map(HatchedDayBody::from).collect(),
            effort: bar.effort.map(|effort| BarEffortBody {
                estimate: effort.estimate.get(),
                effort: effort.effort.get(),
                work_days: effort.work_days,
            }),
            work_time: bar.work_time.map(|time| BarWorkTimeBody {
                usual: time
                    .usual
                    .days()
                    .iter()
                    .copied()
                    .map(UsualDayBody::from)
                    .collect(),
                share: time.share.percent(),
            }),
        }
    }
}
