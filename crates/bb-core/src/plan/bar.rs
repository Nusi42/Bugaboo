//! The days of a bar (ADR-0043, decision 3) and *Plan by effort* (ADR-0030, decision 3).
//!
//! A bar covers whole days. With both dates it runs from the start date to the due date, and a due
//! date never moves. With *Plan by effort* on, an issue with an effort (its estimate times its time
//! factor) and a person on the bar gets its other date worked out when it has only one: forward
//! from the start date, or back from the due date, each day counting with the minutes the person
//! can work on it in the project (work hours times the share, nothing on away days, days off and
//! public holidays). Otherwise an issue with one date is a bar of one day, and an issue with no
//! date has no bar (*Not scheduled yet*).

use jiff::civil::Date;
use serde::Serialize;

use super::Days;
use crate::time::Minutes;
use crate::work_times::DayMinutes;

/// Which date of a bar the plan worked out; shown, never stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkedOut {
    /// Neither: the bar has the dates people set.
    Neither,
    /// The start date, back from the due date.
    Start,
    /// The due date, forward from the start date.
    Due,
}

/// The days of a bar and which of its dates were worked out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bar {
    /// The days it covers.
    pub days: Days,
    /// Which date the plan worked out.
    pub worked_out: WorkedOut,
}

/// What a bar is worked out from.
#[derive(Clone, Copy, Debug)]
pub struct BarInput<'a> {
    /// The start date as the reader may read it.
    pub start: Option<Date>,
    /// The due date as the reader may read it.
    pub due: Option<Date>,
    /// The effort, only with *Plan by effort* on and an issue with an estimate and a person on the
    /// bar: the estimate times the time factor.
    pub effort: Option<Minutes>,
    /// The days of the person on the bar, in order with no gap; empty without one.
    pub days: &'a [DayMinutes],
}

/// The days of the bar of an issue; `None` for an issue with no date (*Not scheduled yet*).
#[must_use]
pub fn bar(input: &BarInput<'_>) -> Option<Bar> {
    let fixed = |days| Bar {
        days,
        worked_out: WorkedOut::Neither,
    };
    match (input.start, input.due, input.effort) {
        (Some(start), Some(due), _) => {
            Some(fixed(Days::new(start, due).unwrap_or(Days::one(start))))
        }
        (Some(start), None, Some(effort)) => {
            let last = forward_end(start, effort, input.days)
                .or_else(|| input.days.last().map(|day| day.date))
                .filter(|last| *last >= start)
                .unwrap_or(start);
            Some(Bar {
                days: Days { first: start, last },
                worked_out: WorkedOut::Due,
            })
        }
        (None, Some(due), Some(effort)) => {
            let first = back_start(due, effort, input.days)
                .or_else(|| input.days.first().map(|day| day.date))
                .filter(|first| *first <= due)
                .unwrap_or(due);
            Some(Bar {
                days: Days { first, last: due },
                worked_out: WorkedOut::Start,
            })
        }
        (Some(date), None, None) | (None, Some(date), None) => Some(fixed(Days::one(date))),
        (None, None, _) => None,
    }
}

/// The day on which an effort that starts on `start` is done: the first day on which the minutes
/// from `start` on add up to it. `None` when the known days do not hold it.
#[must_use]
pub fn forward_end(start: Date, effort: Minutes, days: &[DayMinutes]) -> Option<Date> {
    if effort.get() == 0 {
        return Some(start);
    }
    let mut sum: u64 = 0;
    days.iter()
        .filter(|day| day.date >= start)
        .find(|day| {
            sum += u64::from(day.minutes);
            sum >= u64::from(effort.get())
        })
        .map(|day| day.date)
}

/// The day on which an effort that must be done on `due` starts: back from `due`, the first day on
/// which the minutes up to `due` add up to it. `None` when the known days do not hold it.
#[must_use]
pub fn back_start(due: Date, effort: Minutes, days: &[DayMinutes]) -> Option<Date> {
    if effort.get() == 0 {
        return Some(due);
    }
    let mut sum: u64 = 0;
    days.iter()
        .rev()
        .filter(|day| day.date <= due)
        .find(|day| {
            sum += u64::from(day.minutes);
            sum >= u64::from(effort.get())
        })
        .map(|day| day.date)
}

/// When the days a person set for a bar do not hold its effort (*Not enough days*).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Shortfall {
    /// The work days the effort needs from the start date on; none when the known days do not
    /// hold it.
    pub needs: Option<u32>,
    /// The work days there are from the start date to the due date.
    pub has: u32,
    /// The due date the effort needs (*Make it longer*); none when the known days do not hold it.
    pub due_needed: Option<Date>,
}

/// Whether a bar with both dates set holds its effort: `None` when it does, or when the bar has a
/// worked-out date (it is as long as its effort).
#[must_use]
pub fn shortfall(bar: &Bar, effort: Minutes, days: &[DayMinutes]) -> Option<Shortfall> {
    if bar.worked_out != WorkedOut::Neither {
        return None;
    }
    let within: Vec<&DayMinutes> = days
        .iter()
        .filter(|day| bar.days.contains(day.date))
        .collect();
    let held: u64 = within.iter().map(|day| u64::from(day.minutes)).sum();
    if held >= u64::from(effort.get()) {
        return None;
    }
    let due_needed = forward_end(bar.days.first, effort, days);
    let needs = due_needed.map(|end| work_days(days, Days::new(bar.days.first, end)));
    Some(Shortfall {
        needs,
        has: work_days(days, Some(bar.days)),
        due_needed,
    })
}

/// The days of `within` on which the person can work at all.
fn work_days(days: &[DayMinutes], within: Option<Days>) -> u32 {
    let Some(within) = within else {
        return 0;
    };
    let count = days
        .iter()
        .filter(|day| within.contains(day.date) && day.minutes > 0)
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// The minutes of the effort planned on each day of the bar, for *Too much at once*: the work fills
/// each day as far as the day holds, from the first day on, and the last day holds the rest. A day
/// on which nothing is planned is left out.
#[must_use]
pub fn load(bar: &Bar, effort: Minutes, days: &[DayMinutes]) -> Vec<(Date, u32)> {
    let mut left = effort.get();
    let mut planned = Vec::new();
    for day in days.iter().filter(|day| bar.days.contains(day.date)) {
        if left == 0 {
            break;
        }
        let here = day.minutes.min(left);
        if here > 0 {
            planned.push((day.date, here));
            left -= here;
        }
    }
    planned
}
